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
import org.apache.lucene.analysis.standard.StandardAnalyzer;
import org.apache.lucene.codecs.lucene101.Lucene101Codec;
import org.apache.lucene.codecs.KnnVectorsFormat;
import org.apache.lucene.codecs.lucene99.Lucene99HnswVectorsFormat;
import org.apache.lucene.document.*;
import org.apache.lucene.index.*;
import org.apache.lucene.store.*;
import java.nio.file.*;

public class WriterMetrics {
  static final class ConfiguredCodec extends Lucene101Codec {
    private final KnnVectorsFormat configured = new Lucene99HnswVectorsFormat(Integer.parseInt(System.getenv("RLUCENE_BENCH_M")), Integer.parseInt(System.getenv("RLUCENE_BENCH_BEAM")));
    @Override
    public KnnVectorsFormat getKnnVectorsFormatForField(String field) { return configured; }
  }
  public static void main(String[] args) throws Exception {
    int ndoc=SiftBench.NDOC;int dim=SiftBench.DIM;long totalStart=System.nanoTime();
    IndexWriterConfig cfg=new IndexWriterConfig(new StandardAnalyzer());
    cfg.setCodec(new ConfiguredCodec()).setOpenMode(IndexWriterConfig.OpenMode.CREATE);
    cfg.setUseCompoundFile(false).setRAMBufferSizeMB(Double.parseDouble(System.getenv("RLUCENE_BENCH_RAM_MIB"))).setMaxFullFlushMergeWaitMillis(0);
    if (org.apache.lucene.util.hnsw.HnswGraphBuilder.randSeed!=42) throw new AssertionError("seed");
    System.out.println("CONFIG\t"+cfg+"\nVECTOR_CONFIG\t"+((Lucene101Codec)cfg.getCodec()).getKnnVectorsFormatForField("knn")+"\tseed="+org.apache.lucene.util.hnsw.HnswGraphBuilder.randSeed);
    long indexStart=System.nanoTime();
    Directory dir=new MMapDirectory(Path.of(args[0]));
    SiftBench.Vectors source=new SiftBench.Vectors(args[1]);
    IndexWriter writer=new IndexWriter(dir,cfg);
    long start=System.nanoTime();
    for(int i=0;i<ndoc;i++) {
      Document doc=new Document();
      doc.add(new KnnFloatVectorField("knn",source.next(),SiftBench.SIMILARITY));
      doc.add(new StoredField("id",i));writer.addDocument(doc);
      if((i+1)%100000==0) System.out.println("INDEXED\t"+(i+1));
    }
    long loop=System.nanoTime()-start;System.out.println("FIELD_CONFIG\tdimension="+dim+"\tsimilarity="+System.getenv("RLUCENE_BENCH_SIMILARITY")+"\tdocs="+ndoc);
    long commitStart=System.nanoTime();writer.commit();long commit=System.nanoTime()-commitStart;long indexed=loop+commit;long upstreamIndex=System.nanoTime()-indexStart;
    start=System.nanoTime();((ConcurrentMergeScheduler)writer.getConfig().getMergeScheduler()).sync();long wait=System.nanoTime()-start;
    start=System.nanoTime();writer.close();long close=System.nanoTime()-start;
    IndexWriterConfig append=new IndexWriterConfig(new StandardAnalyzer());append.setCodec(new ConfiguredCodec()).setOpenMode(IndexWriterConfig.OpenMode.APPEND);append.setUseCompoundFile(false).setRAMBufferSizeMB(Double.parseDouble(System.getenv("RLUCENE_BENCH_RAM_MIB"))).setMaxFullFlushMergeWaitMillis(0);
    start=System.nanoTime();try(IndexWriter merged=new IndexWriter(dir,append)){merged.forceMerge(1,false);((ConcurrentMergeScheduler)merged.getConfig().getMergeScheduler()).sync();}long force=System.nanoTime()-start;
    source.close();dir.close();long total=System.nanoTime()-totalStart;
    try(Directory d=new MMapDirectory(Path.of(args[0])); DirectoryReader r=DirectoryReader.open(d)) {
      System.out.println("RESULT\t"+loop+"\t"+indexed+"\t"+wait+"\t"+force+"\t"+close+"\t"+total+"\t"+r.leaves().size()+"\t"+r.numDocs()+"\t"+commit+"\t"+upstreamIndex);
    }
  }
}
