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
//! Run with `cargo run --example api_ergonomics`.
//! Demonstrates public input conversions and heterogeneous collection macros.
use rlucene::core::document::field::Store;
use rlucene::core::document::int_field::IntField;
use rlucene::core::document::knn_float_vector_field::KnnFloatVectorField;
use rlucene::core::document::sorted_doc_values_field::SortedDocValuesField;
use rlucene::core::document::stored_field::StoredField;
use rlucene::core::document::string_field::StringField;
use rlucene::core::document::text_field::TextField;
use rlucene::core::index::directory_reader;
use rlucene::core::index::index_reader::IndexReader;
use rlucene::core::index::index_writer::IndexWriter;
use rlucene::core::index::index_writer_config::IndexWriterConfig;
use rlucene::core::index::term::Term;
use rlucene::core::index::two_phase_commit::TwoPhaseCommit;
use rlucene::core::search::index_searcher::IndexSearcher;
use rlucene::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use rlucene::core::search::match_no_docs_query::MatchNoDocsQuery;
use rlucene::core::search::phrase_query::PhraseQuery;
use rlucene::core::search::sort::Sort;
use rlucene::core::search::sort_field::{SortField, SortFieldType};
use rlucene::core::search::sorted_numeric_sort_field::SortedNumericSortField;
use rlucene::core::search::term_query::TermQuery;
use rlucene::core::store::fs_directory::FSDirectory;
use rlucene::core::util::close::CloseableRef;
use rlucene::core::util::error::lucene_error::Result;
use rlucene::core::util::io_utils::IOUtils;
use rlucene::{fields, into_vec, queries, sort_fields};
use std::sync::Arc;

fn main() -> Result<()> {
  let temp = tempfile::tempdir()?;
  // Paths can be borrowed; no PathBuf conversion is needed at the call site.
  let directory = Arc::new(FSDirectory::open(temp.path())?);
  let directory_result = (|| -> Result<()> {
    let writer = IndexWriter::new(directory.clone(), IndexWriterConfig::new()?)?;
    let writer_result = (|| -> Result<()> {
      let version = String::from("1");
      writer.set_live_commit_data([("version", version)]);
      writer.add_document(fields![
        StringField::from_string("id", "42", Store::Yes)?,
        IntField::new("age", 18, Store::Yes)?,
        TextField::from_string("body", "rust lucene", Store::No)?,
        KnnFloatVectorField::new(String::from("embedding"), [0.1, 0.2, 0.3])?,
        SortedDocValuesField::new("category", "book"),
        StoredField::from_binary("payload", b"hello")?,
      ])?;
      writer.commit()?;

      let reader = Arc::new(directory_reader::open(directory.clone())?);
      let reader_result = (|| -> Result<()> {
        let searcher = IndexSearcher::new(reader.clone().get_context()?)?;
        let words: Vec<String> = into_vec!["rust", String::from("lucene")];
        let phrase = PhraseQuery::from_terms(0, "body", &words)?;
        let sort = Sort::with_fields(sort_fields![
          SortedNumericSortField::new("age", SortFieldType::Int)?,
          SortField::get_field_doc()?,
        ])?;
        let hits = searcher.search_with_sort(phrase, 10, sort)?;
        assert_eq!(hits.base.total_hits.value(), 1);

        let target: &[f32] = &[0.1, 0.2, 0.3];
        let hits = searcher.search(KnnFloatVectorQuery::new("embedding", target, 1)?, 10)?;
        assert_eq!(hits.total_hits.value(), 1);
        let hits = searcher.search(IntField::new_set_query("age", [18, 21])?, 10)?;
        assert_eq!(hits.total_hits.value(), 1);
        println!("Phrase, vector and numeric-set queries each matched one document.");
        Ok(())
      })();
      IOUtils::use_or_suppress_result(reader_result, reader.close())?;

      // The macro chooses Query as the element type before constructing the vector.
      writer.delete_documents_with_queries(queries![
        TermQuery::new(Term::from_text("id", "missing")),
        MatchNoDocsQuery::new(),
      ])?;
      // A homogeneous batch can use an array or iterator directly.
      writer.delete_documents_with_terms([Term::from_text("id", "42")])?;
      writer.commit()?;
      let reader = directory_reader::open(directory.clone())?;
      let result = reader.num_docs().map(|count| assert_eq!(count, 0));
      IOUtils::use_or_suppress_result(result, reader.close())?;
      println!("Batch deletion left zero documents.");
      Ok(())
    })();
    IOUtils::use_or_suppress_result(writer_result, writer.close())
  })();
  IOUtils::use_or_suppress_result(directory_result, directory.close())
}
