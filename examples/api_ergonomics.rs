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
use num_bigint::BigInt;
use rlucene::core::analysis::reader::StringReader;
use rlucene::core::document::binary_point::BinaryPoint;
use rlucene::core::document::field::{FieldBase, Store};
use rlucene::core::document::field_type::FieldType;
use rlucene::core::document::int_field::IntField;
use rlucene::core::document::int_point::IntPoint;
use rlucene::core::document::int_range::IntRange;
use rlucene::core::document::int_range_doc_values_field::IntRangeDocValuesField;
use rlucene::core::document::keyword_field::KeywordField;
use rlucene::core::document::knn_byte_vector_field::KnnByteVectorField;
use rlucene::core::document::knn_float_vector_field::KnnFloatVectorField;
use rlucene::core::document::numeric_doc_values_field::NumericDocValuesField;
use rlucene::core::document::sorted_doc_values_field::SortedDocValuesField;
use rlucene::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use rlucene::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use rlucene::core::document::stored_field::StoredField;
use rlucene::core::document::string_field::StringField;
use rlucene::core::document::text_field::TextField;
use rlucene::core::geo::line::Line;
use rlucene::core::index::bytes_ref::BytesRef;
use rlucene::core::index::directory_reader;
use rlucene::core::index::index_reader::IndexReader;
use rlucene::core::index::index_writer::IndexWriter;
use rlucene::core::index::index_writer_config::IndexWriterConfig;
use rlucene::core::index::term::Term;
use rlucene::core::index::two_phase_commit::TwoPhaseCommit;
use rlucene::core::search::boolean_clause::{BooleanClause, Occur};
use rlucene::core::search::boolean_query::Builder as BooleanQueryBuilder;
use rlucene::core::search::byte_vector_similarity_query::ByteVectorSimilarityQuery;
use rlucene::core::search::disjunction_max_query::DisjunctionMaxQuery;
use rlucene::core::search::explanation::Explanation;
use rlucene::core::search::float_vector_similarity_query::FloatVectorSimilarityQuery;
use rlucene::core::search::index_searcher::IndexSearcher;
use rlucene::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use rlucene::core::search::match_no_docs_query::MatchNoDocsQuery;
use rlucene::core::search::multi_phrase_query::MultiPhraseQuery;
use rlucene::core::search::phrase_query::PhraseQuery;
use rlucene::core::search::query::IntoQuery;
use rlucene::core::search::sort::Sort;
use rlucene::core::search::sort_field::{SortField, SortFieldType};
use rlucene::core::search::sorted_numeric_sort_field::SortedNumericSortField;
use rlucene::core::search::synonym_query::Builder as SynonymQueryBuilder;
use rlucene::core::search::term_in_set_query::TermInSetQuery;
use rlucene::core::search::term_query::TermQuery;
use rlucene::core::store::byte_buffers_data_input::ByteBuffersDataInput;
use rlucene::core::store::data_input::DataInput;
use rlucene::core::store::fs_directory::FSDirectory;
use rlucene::core::util::close::CloseableRef;
use rlucene::core::util::error::lucene_error::Result;
use rlucene::core::util::fixed_bit_set::FixedBitSet;
use rlucene::core::util::io_utils::IOUtils;
use rlucene::core::util::string_helper::StringHelper;
use rlucene::sandbox::document::big_integer_point::BigIntegerPoint;
use rlucene::sandbox::search::term_automaton_query::TermAutomatonQuery;
use rlucene::{fields, into_vec, queries, sort_fields};
use std::io::Cursor;
use std::sync::Arc;

fn main() -> Result<()> {
  // Owned arrays, borrowed values, and independent storage types are accepted.
  let bits = FixedBitSet::with_capacity([0b1011], 4)?;
  let other = FixedBitSet::with_capacity([0b1101], 4)?;
  assert_eq!(FixedBitSet::intersection_count(&bits, &other), 2);
  let line = Line::new([10.0, 20.0], vec![30.0, 40.0])?;
  assert_eq!(line.num_points(), 2);
  let prefix = BytesRef::from_bytes(Arc::new(b"rust".to_vec()));
  assert!(StringHelper::starts_with_byte_ref(
    &BytesRef::from("rust lucene"),
    &prefix
  ));
  let explanation = Explanation::match_(
    1.0f32,
    "total",
    [Explanation::match_no_details(1.0f32, "one contribution")],
  );
  assert_eq!(explanation.get_details().len(), 1);
  let mut input = ByteBuffersDataInput::new([Cursor::new(vec![42u8])], 1)?;
  assert_eq!(input.read_byte()?, 42);
  let big_integer = BigInt::from(42);

  let temp = tempfile::tempdir()?;
  // Paths can be borrowed; no PathBuf conversion is needed at the call site.
  let directory = Arc::new(FSDirectory::open(temp.path())?);
  let directory_result = (|| -> Result<()> {
    let writer = IndexWriter::new(directory.clone(), IndexWriterConfig::new()?)?;
    let writer_result = (|| -> Result<()> {
      let version = String::from("1");
      writer.set_live_commit_data([("version", version)]);
      let mut point_type = FieldType::default();
      point_type.set_dimensions(1, 2)?;
      // Setters accept the same convenient inputs when reusing fields.
      let mut id = StringField::from_bytes_ref("id", b"old", Store::Yes)?;
      id.set_bytes_value(b"42")?;
      let mut kind = KeywordField::from_bytes_ref("kind", b"old", Store::Yes)?;
      kind.set_bytes_value("book")?;
      let mut category = SortedDocValuesField::new("category", "old");
      category.set_bytes_value(b"book")?;
      let mut body = TextField::from_reader("body", "old")?;
      body.set_reader_value(StringReader::new("rust lucene"))?;
      let mut payload = StoredField::from_bytes_ref("payload", b"old")?;
      payload.set_bytes_value(b"hello")?;
      let mut age_window = IntRange::new("age_window", [0], vec![1])?;
      age_window.set_range_values([10], vec![20])?;
      let mut coordinates = IntPoint::new("coordinates", [0, 0])?;
      coordinates.set_int_values([3, 4])?;
      writer.add_document(fields![
        id,
        IntField::new("age", 18, Store::Yes)?,
        body,
        KnnFloatVectorField::new(String::from("embedding"), [0.1, 0.2, 0.3])?,
        category,
        kind,
        NumericDocValuesField::new("rank", 7),
        KnnByteVectorField::new("byte_embedding", [1, 2, 3])?,
        payload,
        BinaryPoint::with_type("code", [0, 42], point_type)?,
        age_window,
        IntRangeDocValuesField::new("age_window_dv", [10], vec![20])?,
        coordinates,
        BigIntegerPoint::new("big_integer", std::slice::from_ref(&big_integer))?,
        BinaryPoint::new("binary_coordinates", [[1, 2], [3, 4]])?,
      ])?;
      writer.commit()?;

      let reader = Arc::new(directory_reader::open(directory.clone())?);
      let reader_result = (|| -> Result<()> {
        let searcher = IndexSearcher::new(reader.clone().get_context()?)?;
        assert_eq!(
          searcher.count(BigIntegerPoint::new_exact_query(
            "big_integer",
            &big_integer
          )?)?,
          1
        );
        assert_eq!(
          searcher.count(BigIntegerPoint::new_range_query(
            "big_integer",
            &big_integer,
            BigInt::from(100)
          )?)?,
          1
        );
        let mut automaton = TermAutomatonQuery::new("body");
        let start = automaton.create_state();
        let end = automaton.create_state();
        automaton.set_accept(end, true);
        automaton.add_transition_bytes(start, end, BytesRef::from("rust"))?;
        automaton.finish()?;
        assert_eq!(searcher.count(automaton)?, 1);
        let words: Vec<String> = into_vec!["rust", String::from("lucene")];
        let phrase = PhraseQuery::from_terms(0, "body", &words)?;
        let sort = Sort::with_fields(sort_fields![
          SortedNumericSortField::new("age", SortFieldType::Int)?,
          SortField::get_field_doc()?,
        ])?;
        let hits = searcher.search_with_sort(phrase, 10, sort)?;
        assert_eq!(hits.base.total_hits.value(), 1);

        // Alternatives at each phrase position accept owned terms and iterators.
        let mut multi_phrase = MultiPhraseQuery::builder();
        multi_phrase.add_terms([Term::new("body", "rust"), Term::new("body", "java")])?;
        multi_phrase.add_terms_with_position(
          ["lucene", "search"]
            .into_iter()
            .map(|word| Term::new("body", word)),
          1,
        )?;
        assert_eq!(searcher.count(multi_phrase.build())?, 1);
        assert_eq!(
          searcher.count(IntRangeDocValuesField::new_slow_intersects_query(
            "age_window_dv",
            [18],
            vec![25]
          )?)?,
          1
        );

        let target: &[f32] = &[0.1, 0.2, 0.3];
        let hits = searcher.search(KnnFloatVectorQuery::new("embedding", target, 1)?, 10)?;
        assert_eq!(hits.total_hits.value(), 1);
        let hits = searcher.search(IntField::new_set_query("age", [18, 21])?, 10)?;
        assert_eq!(hits.total_hits.value(), 1);
        // Each item converts to BytesRef inside the constructor.
        assert_eq!(
          searcher.count(TermInSetQuery::new("id", ["missing", "42", "42"])?)?,
          1
        );
        let bytes = words.iter().map(String::as_bytes);
        assert_eq!(
          searcher.count(PhraseQuery::from_bytes_no_slop("body", bytes)?)?,
          1
        );
        assert_eq!(
          searcher.count(BinaryPoint::new_exact_query("code", [0, 42])?)?,
          1
        );
        let upper: &[u8] = &[0, 100];
        assert_eq!(
          searcher.count(BinaryPoint::new_range_query("code", [0, 0], upper)?)?,
          1
        );

        // Homogeneous queries can be mapped directly without collecting a Vec first.
        let disjuncts = ["missing", "42"]
          .into_iter()
          .map(|id| TermQuery::new(Term::new("id", id)).into_query());
        assert_eq!(
          searcher.count(DisjunctionMaxQuery::new(disjuncts, 0.1)?)?,
          1
        );
        let mut boolean = BooleanQueryBuilder::new();
        boolean.add_all([BooleanClause::new(
          TermQuery::new(Term::new("id", b"42")),
          Occur::Must,
        )])?;
        assert_eq!(searcher.count(boolean.build())?, 1);
        assert_eq!(
          searcher.count(KeywordField::new_set_query("kind", ["book", "article"])?)?,
          1
        );
        assert_eq!(
          searcher.count(SortedDocValuesField::new_slow_set_query(
            "category",
            ["book"]
          )?)?,
          1
        );
        assert_eq!(
          searcher.count(SortedSetDocValuesField::new_slow_set_query(
            "kind",
            ["book"]
          )?)?,
          1
        );
        let ranks: &[i64] = &[7, 9];
        assert_eq!(
          searcher.count(NumericDocValuesField::new_slow_set_query("rank", ranks)?)?,
          1
        );
        assert_eq!(
          searcher.count(SortedNumericDocValuesField::new_slow_set_query(
            "age",
            [18, 21]
          )?)?,
          1
        );
        assert_eq!(
          searcher.count(FloatVectorSimilarityQuery::new("embedding", target, 0.99)?)?,
          1
        );
        assert_eq!(
          searcher.count(ByteVectorSimilarityQuery::new(
            "byte_embedding",
            [1, 2, 3],
            0.99
          )?)?,
          1
        );
        let mut synonyms = SynonymQueryBuilder::new("body");
        synonyms.add_bytes_with_boost("rust", 1.0)?;
        synonyms.add_bytes_with_boost(b"missing", 0.5)?;
        assert_eq!(searcher.count(synonyms.build())?, 1);
        // Lower and upper bounds can use different containers.
        assert_eq!(
          searcher.count(IntRange::new_intersects_query(
            "age_window",
            [18],
            vec![25]
          )?)?,
          1
        );
        assert_eq!(
          searcher.count(IntRange::new_contains_query("age_window", [12], vec![18])?)?,
          1
        );
        assert_eq!(
          searcher.count(IntRange::new_within_query("age_window", [0], vec![30])?)?,
          1
        );
        assert_eq!(
          searcher.count(IntRange::new_crosses_query("age_window", [15], vec![25])?)?,
          1
        );
        assert_eq!(
          searcher.count(IntPoint::new_range_query_n(
            "coordinates",
            [0, 0],
            vec![5, 5]
          )?)?,
          1
        );
        let upper: &[&[u8]] = &[&[5, 5], &[5, 5]];
        assert_eq!(
          searcher.count(BinaryPoint::new_range_query_multi_dim(
            "binary_coordinates",
            [[0, 0], [0, 0]],
            upper
          )?)?,
          1
        );
        assert_eq!(
          searcher.count(BinaryPoint::new_set_query("code", [[0, 1], [0, 42]])?)?,
          1
        );
        println!(
          "All queries matched one document using direct array, slice, and iterator inputs."
        );
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
