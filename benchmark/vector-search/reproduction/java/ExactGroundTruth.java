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
import org.apache.lucene.index.VectorSimilarityFunction;

/** Exhaustive graph-independent Lucene 10.1 float32 Top100, no ANN index. */
public class ExactGroundTruth {
  static final int DIM=SiftBench.DIM, NQ=SiftBench.NQ, NDOC=SiftBench.NDOC, K=100;
  static volatile long warmSink;
  static float[][] load(Path path,int count,String role)throws Exception {
    float[][] rows=new float[count][];byte[] bytes=new byte[DIM*4];
    var buf=ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN);
    var digest=MessageDigest.getInstance("SHA-256");long zeros=0;double min=Double.POSITIVE_INFINITY,max=0;
    List<Integer> zeroOrdinals=new ArrayList<>();
    try(var in=new DataInputStream(new BufferedInputStream(Files.newInputStream(path)))) {
      for(int i=0;i<count;i++) {
        in.readFully(bytes);digest.update(bytes);buf.position(0);float[] row=new float[DIM];double norm=0;
        for(int j=0;j<DIM;j++){float value=buf.getFloat();if(!Float.isFinite(value))throw new AssertionError(role+" non-finite "+i+"/"+j);row[j]=value;norm+=(double)value*value;}
        if(norm==0){zeros++;if(role.equals("query"))zeroOrdinals.add(i);}min=Math.min(min,norm);max=Math.max(max,norm);rows[i]=row;
      }
      if(in.read()!=-1)throw new AssertionError("extra input "+role);
    }
    System.out.println("INPUT\t"+role+"\t"+count+"\t"+DIM+"\t"+HexFormat.of().formatHex(digest.digest())+"\t"+zeros+"\t"+min+"\t"+max+"\t"+zeroOrdinals);System.out.flush();return rows;
  }
  static class Heap {
    int size;final int[] ids=new int[K];final float[] scores=new float[K];
    boolean worse(int a,int b){int c=Float.compare(scores[a],scores[b]);return c<0||(c==0&&ids[a]>ids[b]);}
    void swap(int a,int b){int id=ids[a];ids[a]=ids[b];ids[b]=id;float score=scores[a];scores[a]=scores[b];scores[b]=score;}
    void offer(int id,float score){
      if(size<K){int x=size++;ids[x]=id;scores[x]=score;while(x>0){int parent=(x-1)/2;if(!worse(x,parent))break;swap(x,parent);x=parent;}return;}
      int c=Float.compare(score,scores[0]);if(c<0||(c==0&&id>=ids[0]))return;
      ids[0]=id;scores[0]=score;int x=0;
      while(x*2+1<size){int child=x*2+1;if(child+1<size&&worse(child+1,child))child++;if(!worse(child,x))break;swap(child,x);x=child;}
    }
    Integer[] ordered(){Integer[] order=new Integer[size];for(int i=0;i<size;i++)order[i]=i;Arrays.sort(order,(a,b)->{int c=Float.compare(scores[b],scores[a]);return c!=0?c:Integer.compare(ids[a],ids[b]);});return order;}
  }
  record Scan(Heap heap,long[] independentKeys){}
  static Scan scan(float[] query,float[][] base,boolean independent) {
    Heap heap=new Heap();long[] all=independent?new long[NDOC]:null;
    for(int doc=0;doc<NDOC;doc++) {
      float score=SiftBench.SIMILARITY.compare(query,base[doc]);
      if(!Float.isFinite(score)||score<0)throw new AssertionError("invalid score "+doc);
      heap.offer(doc,score);if(all!=null)all[doc]=((long)Float.floatToRawIntBits(score)<<32)|(0xffffffffL-doc);
    }
    return new Scan(heap,all);
  }
  public static void main(String[] args)throws Exception {
    Path output=Path.of(args[2]);Files.createDirectories(output);
    float[][] base=load(Path.of(args[0]),NDOC,"base"),queries=load(Path.of(args[1]),NQ,"query");
    long warmStart=System.nanoTime();int warmSweeps=0;
    do {int q=warmSweeps%Math.min(NQ,64);Scan result=scan(queries[q],base,q==0);warmSink^=(long)result.heap.ids[0]+Float.floatToRawIntBits(result.heap.scores[0]);warmSweeps++;} while(System.nanoTime()-warmStart<60_000_000_000L);
    System.out.println("EXACT_WARM\t"+(System.nanoTime()-warmStart)+"\t"+warmSweeps);System.out.flush();
    int sortChecks=0;
    try(var ids=new DataOutputStream(new BufferedOutputStream(Files.newOutputStream(output.resolve("groundtruth.raw"))));
        var scores=new DataOutputStream(new BufferedOutputStream(Files.newOutputStream(output.resolve("groundtruth.scores.raw"))))) {
      for(int q=0;q<NQ;q++) {
        boolean independent=q==0||q==NQ/2||q==NQ-1;Scan result=scan(queries[q],base,independent);Heap heap=result.heap;long[] all=result.independentKeys;
        Integer[] order=heap.ordered();if(order.length!=K)throw new AssertionError();
        if(all!=null){Arrays.sort(all);for(int j=0;j<K;j++){long rank=all[NDOC-1-j];int doc=(int)(0xffffffffL-(rank&0xffffffffL));int bits=(int)(rank>>>32);int slot=order[j];if(doc!=heap.ids[slot]||bits!=Float.floatToRawIntBits(heap.scores[slot]))throw new AssertionError("sort oracle "+q+"/"+j);}sortChecks++;}
        for(int slot:order){ids.writeInt(Integer.reverseBytes(heap.ids[slot]));scores.writeInt(Integer.reverseBytes(Float.floatToRawIntBits(heap.scores[slot])));}
        if(q%25==0||q+1==NQ){System.out.println("GROUNDTRUTH\t"+(q+1)+"\t"+NQ);System.out.flush();}
      }
    }
    System.out.println("EXACT_COMPLETE\t"+NDOC+"\t"+NQ+"\t"+DIM+"\t"+sortChecks);
  }
}
