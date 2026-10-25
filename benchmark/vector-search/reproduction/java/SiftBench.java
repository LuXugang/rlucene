/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
import java.io.*;
import java.nio.*;
import java.nio.channels.FileChannel;
import java.nio.file.*;
import java.util.*;
import org.apache.lucene.analysis.standard.StandardAnalyzer;
import org.apache.lucene.codecs.lucene101.Lucene101Codec;
import org.apache.lucene.document.*;
import org.apache.lucene.index.*;
import org.apache.lucene.search.*;
import org.apache.lucene.store.*;
import org.apache.lucene.util.Version;

/** Lucene 10.1-compatible unfiltered float32 KNN path adapted from luceneutil. */
public class SiftBench {
  static long cpuTime() { return ((com.sun.management.OperatingSystemMXBean)java.lang.management.ManagementFactory.getOperatingSystemMXBean()).getProcessCpuTime(); }
  static final int DIM=Integer.parseInt(System.getenv("RLUCENE_BENCH_DIM")), K=100, NQ=Integer.parseInt(System.getenv("RLUCENE_BENCH_NQ")), NDOC=Integer.parseInt(System.getenv("RLUCENE_BENCH_NDOC"));
  static final String FIELD="knn";
  static final VectorSimilarityFunction SIMILARITY=VectorSimilarityFunction.valueOf(System.getenv("RLUCENE_BENCH_SIMILARITY"));
  static class Vectors implements AutoCloseable {
    final FileChannel channel;
    final ByteBuffer bytes=ByteBuffer.allocate(DIM*4).order(ByteOrder.LITTLE_ENDIAN);
    final float[] target=new float[DIM];
    Vectors(String path) throws Exception { channel=FileChannel.open(Path.of(path)); }
    void reset() throws Exception { channel.position(0); }
    float[] next() throws Exception {
      bytes.clear();
      while(bytes.hasRemaining()) if(channel.read(bytes)<0) throw new EOFException();
      bytes.flip(); bytes.asFloatBuffer().get(target); return target;
    }
    public void close() throws IOException { channel.close(); }
  }
  static final com.sun.management.ThreadMXBean THREADS=(com.sun.management.ThreadMXBean)java.lang.management.ManagementFactory.getThreadMXBean();
  static class Snapshot {
    final long ns=System.nanoTime();
    final long[] ids=THREADS.getAllThreadIds(), cpu=THREADS.getThreadCpuTime(ids);
    long delta(Snapshot start){
      if(!Arrays.equals(ids,start.ids))return -1;
      long sum=0;for(int i=0;i<ids.length;i++){if(cpu[i]<0||start.cpu[i]<0||cpu[i]<start.cpu[i])return -1;sum+=cpu[i]-start.cpu[i];}return sum;
    }
  }
  static class Profiled extends KnnFloatVectorQuery {
    long visited;
    Profiled(float[] target,int fanout){super(FIELD,target,K+fanout);}
    @Override protected TopDocs mergeLeafResults(TopDocs[] leaves){TopDocs docs=TopDocs.merge(K,leaves);visited+=docs.totalHits.value();return docs;}
  }
  record QueryResult(TopDocs docs,long visited){}
  static QueryResult query(IndexSearcher searcher,float[] target,int fanout)throws Exception {
    Profiled query=new Profiled(target,fanout);TopDocs docs=searcher.search(query,K);return new QueryResult(docs,query.visited);
  }
  static void server(String index,String queries,String output,int fanout) throws Exception {
    try(Directory dir=new MMapDirectory(Path.of(index)); DirectoryReader reader=DirectoryReader.open(dir);
        Vectors vectors=new Vectors(queries); BufferedReader commands=new BufferedReader(new InputStreamReader(System.in))) {
      IndexSearcher searcher=new IndexSearcher(reader); searcher.setQueryCache(null);
      if(reader.maxDoc()!=NDOC || reader.leaves().size()!=1) throw new AssertionError("wrong index");
      System.err.println("CONFIG\tjava="+Version.LATEST+"\tsegments="+reader.leaves().size()+"\tcodec="+((SegmentReader)reader.leaves().get(0).reader()).getSegmentInfo().info.getCodec().getName());
      int[] stableIds=new int[NDOC];
      {
        StoredFields mapping=reader.storedFields();boolean[] seen=new boolean[NDOC];int permuted=0;
        for(int doc=0;doc<NDOC;doc++) {
          int id=mapping.document(doc).getField("id").numericValue().intValue();
          if(id<0||id>=NDOC||seen[id])throw new AssertionError("invalid stored ID permutation");
          seen[id]=true;stableIds[doc]=id;if(id!=doc)permuted++;
        }
        System.err.println("ID_MAP\tpermuted="+permuted);
      }
      if(reader.leaves().get(0).reader().getFieldInfos().fieldInfo(FIELD).getVectorSimilarityFunction()!=SIMILARITY) throw new AssertionError("wrong similarity");
      TopDocs[] results=new TopDocs[NQ];
      System.out.println("READY"); System.out.flush();
      for(String line;(line=commands.readLine())!=null;) {
        String[] args=line.split(" ");
        if(args[0].equals("QUIT")) break;
        if(args[0].equals("WARM")) {
          long start=System.nanoTime(), count=0; int sweeps=0;
          do {
            vectors.reset();
            for(int i=0;i<NQ;i++) results[i]=query(searcher,vectors.next(),fanout).docs();
            count+=NQ; sweeps++;
          } while(System.nanoTime()-start<Long.parseLong(args[1])*1000000000L);
          System.out.println("WARM\t"+(System.nanoTime()-start)+"\t"+count+"\t"+sweeps); System.out.flush();
          continue;
        }
        if(args[0].equals("VISIT")) {
          vectors.reset();
          try(DataOutputStream counts=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(output+"/"+args[1]+".visited")));
              DataOutputStream hits=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(output+"/"+args[1]+".hits")))) {
            for(int i=0;i<NQ;i++) {
              TopDocs docs=reader.leaves().get(0).reader().searchNearestVectors(FIELD,vectors.next(),K+fanout,null,Integer.MAX_VALUE);
              counts.writeLong(docs.totalHits.value()); docs.scoreDocs=Arrays.copyOf(docs.scoreDocs,Math.min(K,docs.scoreDocs.length)); hits.writeInt(docs.scoreDocs.length);
              for(ScoreDoc h:docs.scoreDocs) { hits.writeInt(stableIds[h.doc]); hits.writeInt(Float.floatToRawIntBits(h.score)); }
            }
          }
          System.out.println("VISIT\t"+args[1]);System.out.flush();continue;
        }
        if(!args[0].equals("RUN") && !args[0].equals("CHECK") && !args[0].equals("LATENCY")) throw new IllegalArgumentException(line);
        vectors.reset();long[] ns=new long[NQ],visited=new long[NQ];
        long cpuStart=cpuTime();Snapshot first=new Snapshot();
        if(args[0].equals("LATENCY")) {
          for(int i=0;i<NQ;i++){long start=System.nanoTime();QueryResult row=query(searcher,vectors.next(),fanout);results[i]=row.docs();visited[i]=row.visited();ns[i]=System.nanoTime()-start;}
        } else {
          for(int i=0;i<NQ;i++){QueryResult row=query(searcher,vectors.next(),fanout);results[i]=row.docs();visited[i]=row.visited();}
        }
        Snapshot last=new Snapshot();long elapsed=last.ns-first.ns,threadCpu=last.delta(first),cpuElapsed=cpuTime()-cpuStart;
        String prefix=output+"/"+args[1];
        if(args[0].equals("CHECK")) {
          try(DataOutputStream out=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(prefix+".hits")))) {
            for(TopDocs docs:results) {
              out.writeInt(docs.scoreDocs.length);
              for(ScoreDoc hit:docs.scoreDocs) {
                out.writeInt(stableIds[hit.doc]); out.writeInt(Float.floatToRawIntBits(hit.score));
              }
            }
          }
        } else if(args[0].equals("LATENCY")) {
          try(DataOutputStream out=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(prefix+".ns")))) {
            for(long n:ns) out.writeLong(n);
          }
        }
        try(DataOutputStream out=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(prefix+".visited")))){for(long v:visited)out.writeLong(v);}
        String raw="{\"wall_ns\":"+elapsed+",\"process_cpu_ns\":"+cpuElapsed+",\"thread_cpu_ns\":"+threadCpu+",\"thread_cpu_valid\":"+(threadCpu>=0)+",\"thread_start_ids\":"+Arrays.toString(first.ids)+",\"thread_end_ids\":"+Arrays.toString(last.ids)+",\"thread_start_cpu_ns\":"+Arrays.toString(first.cpu)+",\"thread_end_cpu_ns\":"+Arrays.toString(last.cpu)+",\"fanout\":"+fanout+",\"visited_total\":"+Arrays.stream(visited).sum()+"}\n";
        Files.writeString(Path.of(prefix+".metrics.json"),raw);
        long checksum=0;
        for(TopDocs docs:results) for(ScoreDoc hit:docs.scoreDocs) checksum+=hit.doc;
        System.out.println(args[0]+"\t"+args[1]+"\t"+elapsed+"\t"+checksum+"\t"+cpuElapsed+"\t"+threadCpu+"\t"+(threadCpu>=0)); System.out.flush();
      }
    }
  }
  public static void main(String[] args) throws Exception {
    if(!Version.LATEST.toString().equals("10.1.0")) throw new AssertionError("wrong Lucene version");
    if(!args[0].equals("server")) throw new IllegalArgumentException("use WriterMetrics for indexing");
    server(args[1],args[2],args[3],Integer.parseInt(args[4]));
  }
}
