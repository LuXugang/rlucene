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
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::{TYPE_NOT_STORED, TextField};
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::synonym_query::{Builder, SynonymQuery};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hits::Relation;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::search::query_utils::QueryUtils;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
  random,
};
use parking_lot::RwLock;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestSynonymQuery;

#[test]
fn test_equals() -> Result<()> {
  let empty1: Query = Builder::new("foo").build().into();
  let empty2: Query = Builder::new("foo").build().into();
  QueryUtils::check_equal(&empty1, &empty2);

  let mut builder1 = Builder::new("foo");
  builder1.add_term(Term::from_text("foo", "bar"))?;
  let mut builder2 = Builder::new("foo");
  builder2.add_term(Term::from_text("foo", "bar"))?;
  QueryUtils::check_equal::<Query>(&builder1.build().into(), &builder2.build().into());

  let mut builder1 = Builder::new("a");
  builder1.add_term(Term::from_text("a", "a"))?;
  builder1.add_term(Term::from_text("a", "b"))?;
  let mut builder2 = Builder::new("a");
  builder2.add_term(Term::from_text("a", "b"))?;
  builder2.add_term(Term::from_text("a", "a"))?;
  QueryUtils::check_equal::<Query>(&builder1.build().into(), &builder2.build().into());

  let mut builder1 = Builder::new("field");
  builder1.add_term_with_boost(Term::from_text("field", "b"), 0.4)?;
  builder1.add_term_with_boost(Term::from_text("field", "c"), 0.2)?;
  builder1.add_term(Term::from_text("field", "d"))?;
  let mut builder2 = Builder::new("field");
  builder2.add_term_with_boost(Term::from_text("field", "b"), 0.4)?;
  builder2.add_term_with_boost(Term::from_text("field", "c"), 0.2)?;
  builder2.add_term(Term::from_text("field", "d"))?;
  QueryUtils::check_equal::<Query>(&builder1.build().into(), &builder2.build().into());

  let mut builder1 = Builder::new("field");
  builder1.add_term_with_boost(Term::from_text("field", "a"), 0.4)?;
  let mut builder2 = Builder::new("field");
  builder2.add_term_with_boost(Term::from_text("field", "b"), 0.4)?;
  QueryUtils::check_unequal::<Query>(&builder1.build().into(), &builder2.build().into());

  let mut builder1 = Builder::new("field");
  builder1.add_term_with_boost(Term::from_text("field", "a"), 0.2)?;
  let mut builder2 = Builder::new("field");
  builder2.add_term_with_boost(Term::from_text("field", "a"), 0.4)?;
  QueryUtils::check_unequal::<Query>(&builder1.build().into(), &builder2.build().into());

  let mut builder1 = Builder::new("field1");
  builder1.add_term_with_boost(Term::from_text("field1", "b"), 0.4)?;
  let mut builder2 = Builder::new("field2");
  builder2.add_term_with_boost(Term::from_text("field2", "b"), 0.4)?;
  QueryUtils::check_unequal::<Query>(&builder1.build().into(), &builder2.build().into());
  Ok(())
}

#[test]
fn test_get_field() -> Result<()> {
  let mut builder = Builder::new("field1");
  builder.add_term(Term::from_text("field1", "a"))?;
  assert_eq!("field1", builder.build().get_field());
  Ok(())
}

#[test]
fn test_bogus_params() -> Result<()> {
  let mut builder = Builder::new("field1");
  builder.add_term(Term::from_text("field1", "a"))?;
  assert!(matches!(
    builder.add_term(Term::from_text("field2", "b")),
    Err(LuceneError::IllegalArgument(_))
  ));

  for boost in [
    1.3,
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    -0.3,
    0.0,
    -0.0,
  ] {
    let mut builder = Builder::new("field1");
    assert!(matches!(
      builder.add_term_with_boost(Term::from_text("field1", "a"), boost),
      Err(LuceneError::IllegalArgument(_))
    ));
  }

  // Java additionally checks null field names. Rust's Builder accepts an owned String and cannot
  // represent a null field value.
  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  assert_eq!("Synonym()", Builder::new("foo").build().to_string("")?);

  let mut builder = Builder::new("foo");
  builder.add_term(Term::from_text("foo", "bar"))?;
  assert_eq!("Synonym(foo:bar)", builder.build().to_string("")?);

  let mut builder = Builder::new("foo");
  builder.add_term(Term::from_text("foo", "bar"))?;
  builder.add_term(Term::from_text("foo", "baz"))?;
  assert_eq!("Synonym(foo:bar foo:baz)", builder.build().to_string("")?);
  Ok(())
}

#[test]
fn test_scores() -> Result<()> {
  do_test_scores(1)?;
  do_test_scores(i32::MAX as usize)
}

fn do_test_scores(total_hits_threshold: usize) -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(StringField::from_string("f", "a", Store::No)?);
  writer.add_document(&mut random, doc)?;

  for _ in 0..10 {
    let mut doc = Document::new();
    doc.add(StringField::from_string("f", "b", Store::No)?);
    writer.add_document(&mut random, doc)?;
  }

  let boost = if random.random_bool(0.5) {
    random.random::<f32>()
  } else {
    1.0
  };
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  let mut builder = Builder::new("f");
  builder.add_term_with_boost(
    Term::from_text("f", "a"),
    if boost == 0.0 { 1.0 } else { boost },
  )?;
  builder.add_term_with_boost(
    Term::from_text("f", "b"),
    if boost == 0.0 { 1.0 } else { boost },
  )?;
  let query: Query = builder.build().into();

  let collector_manager = TopScoreDocCollectorManager::new(
    (searcher.get_index_reader().num_docs()? as usize).min(total_hits_threshold),
    total_hits_threshold,
  )?;
  let top_docs = searcher.search_with_collector_manager(query, &collector_manager)?;
  if top_docs.total_hits.value() < total_hits_threshold {
    assert_eq!(11, top_docs.total_hits.value());
    assert_eq!(Relation::EqualTo, top_docs.total_hits.relation());
  } else {
    assert_eq!(
      Relation::GreaterThanOrEqualTo,
      top_docs.total_hits.relation()
    );
  }
  // All docs must have the same score.
  for score_doc in &top_docs.score_docs {
    assert_eq!(top_docs.score_docs[0].score, score_doc.score);
  }

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()
}

#[test]
fn test_boosts() -> Result<()> {
  do_test_boosts(1)?;
  do_test_boosts(i32::MAX as usize)
}

fn do_test_boosts(total_hits_threshold: usize) -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_type = FieldType::from_ref(&*TYPE_NOT_STORED)?;
  field_type.set_omit_norms(true)?;

  let mut doc = Document::new();
  doc.add(Field::new("f", "c", field_type.clone()));
  writer.add_document(&mut random, doc)?;
  for i in 0..10 {
    let mut doc = Document::new();
    doc.add(Field::new("f", "a a a a", field_type.clone()));
    writer.add_document(&mut random, doc)?;

    let mut doc = Document::new();
    doc.add(Field::new(
      "f",
      if i % 2 == 0 { "b b" } else { "a a b" },
      field_type.clone(),
    ));
    writer.add_document(&mut random, doc)?;
  }
  let mut doc = Document::new();
  doc.add(Field::new("f", "c", field_type));
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  let mut builder = Builder::new("f");
  builder.add_term_with_boost(Term::from_text("f", "a"), 0.25)?;
  builder.add_term_with_boost(Term::from_text("f", "b"), 0.5)?;
  builder.add_term(Term::from_text("f", "c"))?;
  let query: Query = builder.build().into();

  let collector_manager = TopScoreDocCollectorManager::new(
    (searcher.get_index_reader().num_docs()? as usize).min(total_hits_threshold),
    total_hits_threshold,
  )?;
  let top_docs = searcher.search_with_collector_manager(query, &collector_manager)?;
  if top_docs.total_hits.value() < total_hits_threshold {
    assert_eq!(22, top_docs.total_hits.value());
    assert_eq!(Relation::EqualTo, top_docs.total_hits.relation());
  } else {
    assert_eq!(
      Relation::GreaterThanOrEqualTo,
      top_docs.total_hits.relation()
    );
  }
  // All docs must have the same score.
  for score_doc in &top_docs.score_docs {
    assert_eq!(top_docs.score_docs[0].score, score_doc.score);
  }

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()
}

#[test]
fn test_merge_impacts() -> Result<()> {
  let impacts1 = DummyImpactsEnum::new();
  impacts1.reset(
    42,
    vec![
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
      vec![Impact::new(5, 11), Impact::new(8, 13), Impact::new(12, 14)],
    ],
    vec![110, 945],
  );
  let impacts2 = DummyImpactsEnum::new();
  impacts2.reset(
    45,
    vec![
      vec![Impact::new(2, 10), Impact::new(6, 13)],
      vec![Impact::new(3, 9), Impact::new(5, 11), Impact::new(7, 13)],
    ],
    vec![90, 1000],
  );

  let merged_impacts =
    SynonymQuery::merge_impacts(vec![impacts1.clone(), impacts2.clone()], vec![1.0, 1.0]);
  assert_impacts_equal(
    &[
      vec![Impact::new(5, 10), Impact::new(7, 12), Impact::new(14, 13)],
      vec![Impact::new(i32::MAX, 1)],
    ],
    &[90, 1000],
    &merged_impacts.get_impacts()?,
  )?;

  let merged_boosted_impacts =
    SynonymQuery::merge_impacts(vec![impacts1.clone(), impacts2.clone()], vec![0.3, 0.9]);
  assert_impacts_equal(
    &[
      vec![Impact::new(3, 10), Impact::new(4, 12), Impact::new(9, 13)],
      vec![Impact::new(i32::MAX, 1)],
    ],
    &[90, 1000],
    &merged_boosted_impacts.get_impacts()?,
  )?;

  // docID is > the first docIdUpTo of impacts1
  impacts2.reset(
    112,
    vec![
      vec![Impact::new(2, 10), Impact::new(6, 13)],
      vec![Impact::new(3, 9), Impact::new(5, 11), Impact::new(7, 13)],
    ],
    vec![150, 1000],
  );
  assert_impacts_equal(
    &[
      vec![Impact::new(3, 10), Impact::new(5, 12), Impact::new(8, 13)],
      vec![
        Impact::new(3, 9),
        Impact::new(10, 11),
        Impact::new(15, 13),
        Impact::new(19, 14),
      ],
    ],
    &[110, 945],
    &merged_impacts.get_impacts()?,
  )?;

  assert_impacts_equal(
    &[
      vec![Impact::new(1, 10), Impact::new(2, 12), Impact::new(3, 13)],
      vec![
        Impact::new(3, 9),
        Impact::new(7, 11),
        Impact::new(10, 13),
        Impact::new(11, 14),
      ],
    ],
    &[110, 945],
    &merged_boosted_impacts.get_impacts()?,
  )
}

fn assert_impacts_equal<I>(impacts: &[Vec<Impact>], doc_id_up_to: &[i32], actual: &I) -> Result<()>
where
  I: Impacts,
{
  assert_eq!(impacts.len() as i32, actual.num_levels());
  for level in 0..impacts.len() {
    assert_eq!(doc_id_up_to[level], actual.get_doc_id_upto(level as i32));
    assert_eq!(impacts[level], actual.get_impacts(level as i32)?);
  }
  Ok(())
}

#[derive(Clone)]
struct DummyImpactsEnum {
  state: Arc<RwLock<DummyImpacts>>,
}

impl DummyImpactsEnum {
  fn new() -> Self {
    Self {
      state: Arc::new(RwLock::new(DummyImpacts::default())),
    }
  }

  fn reset(&self, doc_id: i32, impacts: Vec<Vec<Impact>>, doc_id_up_to: Vec<i32>) {
    *self.state.write() = DummyImpacts {
      doc_id,
      impacts,
      doc_id_up_to,
    };
  }
}

#[derive(Clone, Default)]
struct DummyImpacts {
  doc_id: i32,
  impacts: Vec<Vec<Impact>>,
  doc_id_up_to: Vec<i32>,
}

impl Impacts for DummyImpacts {
  fn num_levels(&self) -> i32 {
    self.impacts.len() as i32
  }

  fn get_doc_id_upto(&self, level: i32) -> i32 {
    self.doc_id_up_to[level as usize]
  }

  fn get_impacts(&self, level: i32) -> Result<Vec<Impact>> {
    Ok(self.impacts[level as usize].clone())
  }
}

impl ImpactsSource for DummyImpactsEnum {
  fn advance_shallow(&mut self, _target: i32) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  type Impacts<'a> = DummyImpacts;

  fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
    Ok(self.state.read().clone())
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for DummyImpactsEnum {}
impl DocIdSetIterator for DummyImpactsEnum {
  fn doc_id(&self) -> i32 {
    self.state.read().doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl PostingsEnum for DummyImpactsEnum {
  fn freq(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn next_position(&mut self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn start_offset(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn end_offset(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl ImpactsEnum for DummyImpactsEnum {}

#[test]
fn test_random_top_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  let num_docs = if is_night_mode() {
    at_least(&mut random, 128 * 8 * 8 * 3)
  } else {
    at_least(&mut random, 100)
  };

  for _ in 0..num_docs {
    let mut doc = Document::new();
    let num_values_shift = random.random_range(0..5);
    let num_values = random.random_range(0..(1 << num_values_shift));
    let start = random.random_range(0..10);
    for j in 0..num_values {
      let frequency_shift = random.random_range(0..3);
      let frequency = random.random_range(1..=1 << frequency_shift);
      for _ in 0..frequency {
        doc.add(TextField::from_string(
          "foo",
          (start + j).to_string(),
          Store::No,
        )?);
      }
    }
    writer.add_document(doc)?;
  }

  let reader = directory_reader::open_from_writer(&writer)?;
  writer.close()?;
  let searcher = new_searcher_with_reader(reader)?;

  for term1 in 0..15 {
    let mut term2 = random.random_range(0..15);
    while term1 == term2 {
      term2 = random.random_range(0..15);
    }
    let boost1 = if random.random_bool(0.5) {
      random.random::<f32>().max(f32::MIN_POSITIVE)
    } else {
      1.0
    };
    let boost2 = if random.random_bool(0.5) {
      random.random::<f32>().max(f32::MIN_POSITIVE)
    } else {
      1.0
    };
    let mut builder = Builder::new("foo");
    builder.add_term_with_boost(Term::from_text("foo", term1.to_string()), boost1)?;
    builder.add_term_with_boost(Term::from_text("foo", term2.to_string()), boost2)?;
    let query: Query = builder.build().into();

    let complete_manager = TopScoreDocCollectorManager::new(10, i32::MAX as usize)?;
    let top_scores_manager = TopScoreDocCollectorManager::new(10, 1)?;
    let complete = searcher.search_with_collector_manager(query.clone(), &complete_manager)?;
    let top_scores = searcher.search_with_collector_manager(query.clone(), &top_scores_manager)?;
    CheckHits::check_equal(&query, complete.score_docs(), top_scores.score_docs())?;

    let filter_term = random.random_range(0..15);
    let mut filtered_query = BooleanQueryBuilder::new();
    filtered_query.add(query.clone(), Occur::Must)?;
    filtered_query.add(
      TermQuery::new(Term::from_text("foo", filter_term.to_string())),
      Occur::Filter,
    )?;
    let filtered_query: Query = filtered_query.build().into();

    let complete_manager = TopScoreDocCollectorManager::new(10, i32::MAX as usize)?;
    let top_scores_manager = TopScoreDocCollectorManager::new(10, 1)?;
    let complete =
      searcher.search_with_collector_manager(filtered_query.clone(), &complete_manager)?;
    let top_scores = searcher.search_with_collector_manager(filtered_query, &top_scores_manager)?;
    CheckHits::check_equal(&query, complete.score_docs(), top_scores.score_docs())?;
  }

  searcher.get_index_reader().close()?;
  dir.close()
}

#[test]
fn test_rewrite() -> Result<()> {
  let searcher = new_searcher_with_reader(crate::core::index::multi_reader::MultiReader::empty()?)?;

  // Zero-length SynonymQuery is rewritten.
  let query = Builder::new("f").build();
  assert!(query.get_terms().is_empty());
  assert_eq!(
    Query::from(MatchNoDocsQuery::new()),
    searcher.rewrite(Query::from(query))?
  );

  // A non-boosted single-term SynonymQuery is rewritten.
  let mut builder = Builder::new("f");
  builder.add_term_with_boost(Term::from_text("f", ""), 1.0)?;
  let query = builder.build();
  assert_eq!(1, query.get_terms().len());
  assert_eq!(
    Query::from(TermQuery::new(Term::from_text("f", ""))),
    searcher.rewrite(Query::from(query))?
  );

  // A boosted single-term SynonymQuery is not rewritten.
  let mut builder = Builder::new("f");
  builder.add_term_with_boost(Term::from_text("f", ""), 0.8)?;
  let query: Query = builder.build().into();
  assert_eq!(
    1,
    match &query {
      Query::Synonym(query) => query.get_terms().len(),
      _ => 0,
    }
  );
  assert_eq!(query, searcher.rewrite(query.clone())?);

  // A multiple-term SynonymQuery is not rewritten.
  let mut builder = Builder::new("f");
  builder.add_term_with_boost(Term::from_text("f", ""), 1.0)?;
  builder.add_term_with_boost(Term::from_text("f", ""), 1.0)?;
  let query: Query = builder.build().into();
  assert_eq!(
    2,
    match &query {
      Query::Synonym(query) => query.get_terms().len(),
      _ => 0,
    }
  );
  assert_eq!(query, searcher.rewrite(query.clone())?);
  searcher.get_index_reader().close()
}
