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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::fields::Fields;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::abstract_knn_vector_query::AbstractKnnVectorQuery;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::new_directory_shared;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::new_searcher_with_reader;
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::fmt::Debug;
use std::sync::Arc;

pub trait BaseKnnVectorQueryTestCase {
  type KnnVectorQuery: AbstractKnnVectorQuery + Clone + PartialEq + Eq + Debug + Into<Query>;

  fn get_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery>;

  fn get_throwing_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery>;

  fn get_knn_vector_query_no_filter(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
  ) -> Result<Self::KnnVectorQuery> {
    self.get_knn_vector_query(field, query, k, None)
  }

  fn random_vector<R>(&self, random: &mut R, dim: usize) -> Vec<f32>
  where
    R: Rng + ?Sized;

  fn get_knn_vector_field_with_similarity(
    &self,
    name: &str,
    vector: Vec<f32>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Fields>;

  fn get_knn_vector_field(&self, name: &str, vector: Vec<f32>) -> Result<Fields>;

  type Directory: Directory + Clone;
  fn new_directory_for_test<R>(&self, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized;
  fn default_new_directory_for_test<R>(&self, random: &mut R) -> Result<Arc<DirEnum>>
  where
    R: Rng + ?Sized,
  {
    new_directory_shared(random)
  }

  fn test_equals(&self) -> Result<()> {
    let q1 = self.get_knn_vector_query_no_filter("f1", vec![0.0, 1.0], 10)?;
    let filter1 = TermQuery::new(Term::from_text("id", "id1"));
    let q2 = self.get_knn_vector_query("f1", vec![0.0, 1.0], 10, Some(filter1.clone().into()))?;

    assert_ne!(q2, q1);
    assert_ne!(q1, q2);
    assert_eq!(
      q2,
      self.get_knn_vector_query("f1", vec![0.0, 1.0], 10, Some(filter1.into()))?
    );

    let filter2 = TermQuery::new(Term::from_text("id", "id2"));
    assert_ne!(
      q2,
      self.get_knn_vector_query("f1", vec![0.0, 1.0], 10, Some(filter2.into()))?
    );

    assert_eq!(
      q1,
      self.get_knn_vector_query_no_filter("f1", vec![0.0, 1.0], 10)?
    );

    assert_ne!(Some(q1.clone()), None);

    let term_query: Query = TermQuery::new(Term::from_text("f1", "x")).into();
    let q1_query: Query = q1.clone().into();
    assert_ne!(q1_query, term_query);

    assert_ne!(
      q1,
      self.get_knn_vector_query_no_filter("f2", vec![0.0, 1.0], 10)?
    );
    assert_ne!(
      q1,
      self.get_knn_vector_query_no_filter("f1", vec![1.0, 1.0], 10)?
    );
    assert_ne!(
      q1,
      self.get_knn_vector_query_no_filter("f1", vec![0.0, 1.0], 2)?
    );
    assert_ne!(
      q1,
      self.get_knn_vector_query_no_filter("f1", vec![0.0], 10)?
    );

    Ok(())
  }

  fn test_get_field(&self) -> Result<()> {
    let q1 = self.get_knn_vector_query_no_filter("f1", vec![0.0, 1.0], 10)?;
    let filter1 = TermQuery::new(Term::from_text("id", "id1"));
    let q2 = self.get_knn_vector_query("f2", vec![0.0, 1.0], 10, Some(filter1.into()))?;

    assert_eq!("f1", q1.base().field);
    assert_eq!("f2", q2.base().field);
    Ok(())
  }

  fn test_get_k(&self) -> Result<()> {
    let q1 = self.get_knn_vector_query_no_filter("f1", vec![0.0, 1.0], 6)?;
    let filter1 = TermQuery::new(Term::from_text("id", "id1"));
    let q2 = self.get_knn_vector_query("f2", vec![0.0, 1.0], 7, Some(filter1.into()))?;

    assert_eq!(6, q1.base().k);
    assert_eq!(7, q2.base().k);
    Ok(())
  }

  fn test_get_filter(&self) -> Result<()> {
    let q1 = self.get_knn_vector_query_no_filter("f1", vec![0.0, 1.0], 6)?;
    let filter1 = TermQuery::new(Term::from_text("id", "id1"));
    let filter1_query: Query = filter1.clone().into();
    let q2 = self.get_knn_vector_query("f2", vec![0.0, 1.0], 7, Some(filter1_query.clone()))?;

    assert!(q1.base().filter.is_none());
    assert_eq!(Some(&filter1_query), q2.base().filter.as_deref());
    Ok(())
  }

  fn test_empty_index(&self) -> Result<()> {
    let searcher = new_searcher_with_reader(MultiReader::empty()?)?;
    let kvq = self.get_knn_vector_query_no_filter("field", vec![1.0, 2.0], 10)?;

    let hits = searcher.search(kvq.clone(), 10)?;
    assert_eq!(0, hits.score_docs.len());

    let rewritten = searcher.rewrite(kvq)?;
    assert!(matches!(
      rewritten,
      Query::MatchNoDocs(MatchNoDocsQuery { .. })
    ));
    Ok(())
  }

  fn test_find_all(&self) -> Result<()> {
    let index_store =
      self.get_index_store("field", &[vec![0.0, 1.0], vec![1.0, 2.0], vec![0.0, 0.0]])?;
    let reader = directory_reader_util::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;
    let kvq = self.get_knn_vector_query_no_filter("field", vec![0.0, 0.0], 10)?;

    self.assert_matches(&searcher, kvq.clone(), 3)?;
    let score_docs = searcher.search(kvq, 3)?.score_docs;
    self.assert_id_matches(searcher.get_index_reader(), "id2", &score_docs[0])?;
    self.assert_id_matches(searcher.get_index_reader(), "id0", &score_docs[1])?;
    self.assert_id_matches(searcher.get_index_reader(), "id1", &score_docs[2])?;
    Ok(())
  }

  fn test_find_fewer(&self) -> Result<()> {
    let index_store =
      self.get_index_store("field", &[vec![0.0, 1.0], vec![1.0, 2.0], vec![0.0, 0.0]])?;
    let reader = directory_reader_util::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;
    let kvq = self.get_knn_vector_query_no_filter("field", vec![0.0, 0.0], 2)?;

    self.assert_matches(&searcher, kvq.clone(), 2)?;
    let score_docs = searcher.search(kvq, 3)?.score_docs;
    assert_eq!(2, score_docs.len());
    self.assert_id_matches(searcher.get_index_reader(), "id2", &score_docs[0])?;
    self.assert_id_matches(searcher.get_index_reader(), "id0", &score_docs[1])?;
    Ok(())
  }

  fn test_search_boost(&self) -> Result<()> {
    let index_store =
      self.get_index_store("field", &[vec![0.0, 1.0], vec![1.0, 2.0], vec![0.0, 0.0]])?;
    let reader = directory_reader_util::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;

    let vector_query: Query = self
      .get_knn_vector_query_no_filter("field", vec![0.0, 0.0], 10)?
      .into();
    let score_docs = searcher.search(vector_query.clone(), 3)?.score_docs;

    let boost_query = BoostQuery::new(vector_query, 3.0)?;
    let boost_score_docs = searcher.search(boost_query, 3)?.score_docs;
    assert_eq!(score_docs.len(), boost_score_docs.len());

    for (score_doc, boost_score_doc) in score_docs.iter().zip(boost_score_docs.iter()) {
      assert_eq!(score_doc.doc, boost_score_doc.doc);
      assert!((score_doc.score * 3.0 - boost_score_doc.score).abs() <= 0.001);
    }
    Ok(())
  }

  fn test_simple_filter(&self) -> Result<()> {
    let index_store =
      self.get_index_store("field", &[vec![0.0, 1.0], vec![1.0, 2.0], vec![0.0, 0.0]])?;
    let reader = directory_reader_util::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;
    let filter: Query = TermQuery::new(Term::from_text("id", "id2")).into();
    let kvq: Query = self
      .get_knn_vector_query("field", vec![0.0, 0.0], 10, Some(filter))?
      .into();
    let top_docs = searcher.search(kvq, 3)?;

    assert_eq!(1, top_docs.total_hits.value());
    self.assert_id_matches(searcher.get_index_reader(), "id2", &top_docs.score_docs[0])?;
    Ok(())
  }

  fn get_index_store(&self, field: &str, contents: &[Vec<f32>]) -> Result<Arc<DirEnum>> {
    self.get_index_store_with_similarity(
      &mut rand::rng(),
      field,
      VectorSimilarityFunction::Euclidean,
      contents,
    )
  }

  fn get_index_store_with_similarity<R>(
    &self,
    random: &mut R,
    field: &str,
    vector_similarity_function: VectorSimilarityFunction,
    contents: &[Vec<f32>],
  ) -> Result<Arc<DirEnum>>
  where
    R: Rng + ?Sized,
  {
    let index_store = self.default_new_directory_for_test(random)?;
    let writer = RandomIndexWriter::new(random, index_store.clone());

    for (i, vector) in contents.iter().enumerate() {
      let mut doc = Document::new();
      doc.add(self.get_knn_vector_field_with_similarity(
        field,
        vector.clone(),
        vector_similarity_function,
      )?);
      doc.add(StringField::from_string(
        "id",
        format!("id{i}"),
        Store::Yes,
      )?);
      writer.add_document(doc)?;

      if random.random_bool(0.5) {
        for j in 0..TestUtil::next_usize(random, 1, 5) {
          let mut doc = Document::new();
          doc.add(StringField::from_string("other", "value", Store::No)?);
          doc.add(StringField::from_string(
            "id",
            format!("id{j}"),
            Store::Yes,
          )?);
          writer.add_document(doc)?;
        }
      }
    }

    for _ in 0..5 {
      let mut doc = Document::new();
      doc.add(StringField::from_string("other", "value", Store::No)?);
      writer.add_document(doc)?;
    }

    writer.close()?;
    Ok(index_store)
  }

  fn get_stable_index_store<R>(
    &self,
    random: &mut R,
    field: &str,
    contents: &[Vec<f32>],
  ) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    let index_store = self.new_directory_for_test(random)?;
    let writer = IndexWriter::new(index_store.clone().into(), IndexWriterConfig::new())?;

    for (i, vector) in contents.iter().enumerate() {
      let mut doc = Document::new();
      doc.add(self.get_knn_vector_field(field, vector.clone())?);
      doc.add(StringField::from_string(
        "id",
        format!("id{i}"),
        Store::Yes,
      )?);
      writer.add_document(doc)?;
    }

    for _ in 0..5 {
      let mut doc = Document::new();
      doc.add(StringField::from_string("other", "value", Store::No)?);
      writer.add_document(doc)?;
    }

    writer.close()?;
    Ok(index_store)
  }

  fn assert_matches<IRC>(
    &self,
    searcher: &IndexSearcher<IRC>,
    q: impl Into<Query>,
    expected_matches: usize,
  ) -> Result<()>
  where
    IRC: crate::core::index::index_reader_context::IndexReaderContext,
  {
    let result = searcher.search(q, 1000)?.score_docs;
    assert_eq!(expected_matches, result.len());
    Ok(())
  }

  fn assert_id_matches<IR>(
    &self,
    reader: &IR,
    expected_id: &str,
    score_doc: &ScoreDoc,
  ) -> Result<()>
  where
    IR: IndexReader,
  {
    let actual_id = reader
      .stored_fields()?
      .document(score_doc.doc)?
      .get("id")?
      .map(|v| v.into_owned());
    assert_eq!(Some(expected_id.to_string()), actual_id);
    Ok(())
  }

  fn assert_doc_score_query_to_string(&self, query: &Query) -> Result<()> {
    let query_string = query.as_string("ignored")?;
    assert!(query_string.starts_with("DocAndScoreQuery["));
    assert!(query_string.contains(",...]["));
    assert!(query_string.contains(",...],"));
    assert!(
      query_string.ends_with(",1") || query_string.ends_with(",1.0"),
      "unexpected query string: {query_string}"
    );
    Ok(())
  }
}
