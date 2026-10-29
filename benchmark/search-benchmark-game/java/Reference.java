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
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import org.apache.lucene.analysis.CharArraySet;
import org.apache.lucene.analysis.standard.StandardAnalyzer;
import org.apache.lucene.index.*;
import org.apache.lucene.queryparser.classic.QueryParser;
import org.apache.lucene.search.*;
import org.apache.lucene.search.similarities.BM25Similarity;
import org.apache.lucene.store.*;

/** The recovered serial public API benchmark; verification includes doc IDs and raw f32 bits. */
public class Reference {
  static String jsonString(String text) {
    return "\"" + text.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
  }
  public static void main(String[] args) throws Exception {
    try (var dir=new MMapDirectory(Path.of(args[1]));
         var analyzer=new StandardAnalyzer(CharArraySet.EMPTY_SET);
         var reader=DirectoryReader.open(dir);
         var input=new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      var searcher=new IndexSearcher(reader);
      searcher.setQueryCache(null);
      searcher.setSimilarity(new BM25Similarity(1.2f, .75f));
      var parser=new QueryParser("text",analyzer);
      System.err.println("INDEX max_doc="+reader.maxDoc()+" num_docs="+reader.numDocs()+" segments="+reader.leaves().size()+" deletions="+reader.hasDeletions());
      if (args[0].equals("info")) {
        System.out.println("{\"max_doc\":"+reader.maxDoc()+",\"num_docs\":"+reader.numDocs()+",\"leaves\":"+reader.leaves().size()+",\"text_doc_count\":"+reader.getDocCount("text")+",\"text_sum_total_term_freq\":"+reader.getSumTotalTermFreq("text")+",\"text_sum_doc_freq\":"+reader.getSumDocFreq("text")+"}");
        return;
      }
      boolean verify=args[0].equals("verify");
      String line;
      while ((line=input.readLine())!=null) {
        var parts=line.split("\t",2);
        String command=parts[0];
        Query query=parser.parse(parts[1]);
        if (command.equals("COUNT") || command.equals("UNOPTIMIZED_COUNT")) {
          int count=searcher.count(query);
          System.out.println(verify ? "{\"count\":"+count+"}" : count);
        } else {
          int k=Integer.parseInt(command.split("_")[1]);
          boolean exact=command.endsWith("_COUNT");
          TopDocs hits=searcher.search(query,new TopScoreDocCollectorManager(k, exact ? Integer.MAX_VALUE : k));
          if (verify) {
            String relation=hits.totalHits.relation()==TotalHits.Relation.EQUAL_TO ? "EqualTo":"GreaterThanOrEqualTo";
            StringBuilder out=new StringBuilder("{\"count\":").append(hits.totalHits.value()).append(",\"relation\":\"").append(relation).append("\",\"docs\":[");
            var stored=searcher.storedFields();
            for(int i=0;i<hits.scoreDocs.length;i++) {
              if(i>0)out.append(',');
              ScoreDoc hit=hits.scoreDocs[i];
              out.append("{\"id\":").append(jsonString(stored.document(hit.doc).get("id")))
                 .append(",\"doc\":").append(hit.doc).append(",\"score_bits\":").append(Integer.toUnsignedLong(Float.floatToRawIntBits(hit.score))).append('}');
            }
            System.out.println(out.append("]}"));
          } else System.out.println(exact ? hits.totalHits.value() : 1);
        }
        System.out.flush();
      }
    }
  }
}
