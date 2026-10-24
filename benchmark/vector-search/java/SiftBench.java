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
  static final int DIM=128, K=100, NQ=10000;
  static final String FIELD="knn";
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
  static void build(String index, String vectors) throws Exception {
    try(Directory dir=new MMapDirectory(Path.of(index)); Vectors source=new Vectors(vectors)) {
      IndexWriterConfig cfg=new IndexWriterConfig(new StandardAnalyzer());
      cfg.setCodec(new Lucene101Codec()).setOpenMode(IndexWriterConfig.OpenMode.CREATE);
      cfg.setUseCompoundFile(false).setRAMBufferSizeMB(2048);
      try(IndexWriter w=new IndexWriter(dir,cfg)) {
        for(int i=0;i<1000000;i++) {
          Document doc=new Document();
          doc.add(new KnnFloatVectorField(FIELD,source.next(),VectorSimilarityFunction.EUCLIDEAN));
          doc.add(new StoredField("id",i)); w.addDocument(doc);
          if((i+1)%50000==0) System.out.println("INDEXED\t"+(i+1));
        }
        w.forceMerge(1);
      }
    }
    System.out.println("BUILT");
  }
  static TopDocs query(IndexSearcher searcher,float[] target) throws Exception {
    return searcher.search(new KnnFloatVectorQuery(FIELD,target,K),K);
  }
  static void server(String index,String queries,String output) throws Exception {
    try(Directory dir=new MMapDirectory(Path.of(index)); DirectoryReader reader=DirectoryReader.open(dir);
        Vectors vectors=new Vectors(queries); BufferedReader commands=new BufferedReader(new InputStreamReader(System.in))) {
      IndexSearcher searcher=new IndexSearcher(reader); searcher.setQueryCache(null);
      if(reader.maxDoc()!=1000000 || reader.leaves().size()!=1) throw new AssertionError("wrong index");
      System.err.println("CONFIG\tjava="+Version.LATEST+"\tsegments="+reader.leaves().size()+"\tcodec="+((SegmentReader)reader.leaves().get(0).reader()).getSegmentInfo().info.getCodec().getName());
      TopDocs[] results=new TopDocs[NQ];
      System.out.println("READY"); System.out.flush();
      for(String line;(line=commands.readLine())!=null;) {
        String[] args=line.split(" ");
        if(args[0].equals("QUIT")) break;
        if(args[0].equals("WARM")) {
          long start=System.nanoTime(), count=0; int sweeps=0;
          do {
            vectors.reset();
            for(int i=0;i<NQ;i++) results[i]=query(searcher,vectors.next());
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
              TopDocs docs=reader.leaves().get(0).reader().searchNearestVectors(FIELD,vectors.next(),K,null,Integer.MAX_VALUE);
              counts.writeLong(docs.totalHits.value()); hits.writeInt(docs.scoreDocs.length);
              for(ScoreDoc h:docs.scoreDocs) { hits.writeInt(h.doc); hits.writeInt(Float.floatToRawIntBits(h.score)); }
            }
          }
          System.out.println("VISIT\t"+args[1]);System.out.flush();continue;
        }
        if(!args[0].equals("RUN") && !args[0].equals("CHECK")) throw new IllegalArgumentException(line);
        vectors.reset(); long[] ns=new long[NQ];
        long cpuStart=cpuTime(); long allStart=System.nanoTime();
        for(int i=0;i<NQ;i++) {
          long start=System.nanoTime();
          results[i]=query(searcher,vectors.next());
          ns[i]=System.nanoTime()-start;
        }
        long elapsed=System.nanoTime()-allStart; long cpuElapsed=cpuTime()-cpuStart;
        String prefix=output+"/"+args[1];
        if(args[0].equals("CHECK")) {
          StoredFields stored=reader.storedFields();
          try(DataOutputStream out=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(prefix+".hits")))) {
            for(TopDocs docs:results) {
              out.writeInt(docs.scoreDocs.length);
              for(ScoreDoc hit:docs.scoreDocs) {
                Number id=stored.document(hit.doc).getField("id").numericValue();
                if(id.intValue()!=hit.doc) throw new AssertionError("docid mapping differs");
                out.writeInt(id.intValue()); out.writeInt(Float.floatToRawIntBits(hit.score));
              }
            }
          }
        } else {
          try(DataOutputStream out=new DataOutputStream(new BufferedOutputStream(new FileOutputStream(prefix+".ns")))) {
            for(long n:ns) out.writeLong(n);
          }
        }
        long checksum=0;
        for(TopDocs docs:results) for(ScoreDoc hit:docs.scoreDocs) checksum+=hit.doc;
        System.out.println(args[0]+"\t"+args[1]+"\t"+elapsed+"\t"+checksum+"\t"+cpuElapsed); System.out.flush();
      }
    }
  }
  public static void main(String[] args) throws Exception {
    if(!Version.LATEST.toString().equals("10.1.0")) throw new AssertionError("wrong Lucene version");
    if(args[0].equals("build")) build(args[1],args[2]); else server(args[1],args[2],args[3]);
  }
}
