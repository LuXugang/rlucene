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
import java.nio.file.*;
import java.security.*;
import java.util.*;
import org.apache.lucene.codecs.hnsw.HnswGraphProvider;
import org.apache.lucene.codecs.perfield.PerFieldKnnVectorsFormat;
import org.apache.lucene.index.*;
import org.apache.lucene.search.DocIdSetIterator;
import org.apache.lucene.store.*;
import org.apache.lucene.util.hnsw.HnswGraph;
public class VerifyIndex {
  static void put(MessageDigest md,int value) { md.update((byte)(value>>>24));md.update((byte)(value>>>16));md.update((byte)(value>>>8));md.update((byte)value); }
  public static void main(String[] args)throws Exception {
    try(Directory dir=new MMapDirectory(Path.of(args[0]));DirectoryReader reader=DirectoryReader.open(dir);InputStream raw=new BufferedInputStream(Files.newInputStream(Path.of(args[1])))) {
      if(reader.numDocs()!=SiftBench.NDOC||reader.leaves().size()!=1||reader.hasDeletions())throw new AssertionError("shape");
      var leaf=(SegmentReader)reader.leaves().get(0).reader();
      if(!leaf.getSegmentInfo().info.getCodec().getName().equals("Lucene101"))throw new AssertionError("codec");
      if(leaf.getFieldInfos().fieldInfo("knn").getVectorSimilarityFunction()!=SiftBench.SIMILARITY)throw new AssertionError("wrong similarity");
      var values=leaf.getFloatVectorValues("knn");var stored=leaf.storedFields();
      if(values.dimension()!=SiftBench.DIM||values.size()!=SiftBench.NDOC)throw new AssertionError("vectors");
      byte[] input=raw.readAllBytes();if(input.length!=SiftBench.NDOC*SiftBench.DIM*4)throw new AssertionError("input size");
      var vectorHash=MessageDigest.getInstance("SHA-256");vectorHash.update(input);
      var expected=ByteBuffer.wrap(input).order(ByteOrder.LITTLE_ENDIAN);
      var idHash=MessageDigest.getInstance("SHA-256");boolean[] seen=new boolean[SiftBench.NDOC];int permuted=0;
      for(int ord=0;ord<values.size();ord++) {
        int id=stored.document(ord).getField("id").numericValue().intValue();
        if(values.ordToDoc(ord)!=ord||id<0||id>=SiftBench.NDOC||seen[id])throw new AssertionError("doc "+ord);
        seen[id]=true;if(id!=ord)permuted++;put(idHash,id);
        float[] actual=values.vectorValue(ord);int offset=id*SiftBench.DIM*4;
        for(int j=0;j<SiftBench.DIM;j++)if(Float.floatToRawIntBits(actual[j])!=expected.getInt(offset+j*4))throw new AssertionError("vector "+ord+" "+j);
      }
      System.out.println("MAPPING\t"+permuted+"\t"+HexFormat.of().formatHex(idHash.digest()));
      var format=((PerFieldKnnVectorsFormat.FieldsReader)leaf.getVectorReader()).getFieldReader("knn");
      HnswGraph graph=((HnswGraphProvider)format).getGraph("knn");var digest=MessageDigest.getInstance("SHA-256");
      put(digest,graph.size());put(digest,graph.numLevels());put(digest,graph.entryNode());long nodes=0,edges=0;
      for(int level=0;level<graph.numLevels();level++) {
        put(digest,level);var it=graph.getNodesOnLevel(level);
        while(it.hasNext()) {
          int node=it.nextInt();put(digest,node);nodes++;graph.seek(level,node);int n;
          while((n=graph.nextNeighbor())!=DocIdSetIterator.NO_MORE_DOCS){put(digest,n);edges++;}put(digest,-1);
        }
        put(digest,-2);
      }
      leaf.checkIntegrity();
      System.out.println("VERIFIED\t"+SiftBench.NDOC+"\t"+SiftBench.DIM+"\t"+HexFormat.of().formatHex(vectorHash.digest())+"\t"+HexFormat.of().formatHex(digest.digest())+"\t"+graph.numLevels()+"\t"+graph.entryNode()+"\t"+nodes+"\t"+edges);
    }
  }
}
