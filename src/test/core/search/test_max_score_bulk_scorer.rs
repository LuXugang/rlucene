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
use crate::core::document::string_field::StringField;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{AllDISI, DocIdSetIterator};
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_log_merge_policy, new_searcher_with_reader, random,
};
use std::fmt::{Display, Formatter};

use crate::core::index::directory_reader;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::max_score_bulk_scorer::{INNER_WINDOW_SIZE, MaxScoreBulkScorer};
use crate::core::search::query::Query;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::TwoPhaseState::No;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::term_query::TermQuery;

use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::store::directory::Directory;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use rand::Rng;
use rand::prelude::SliceRandom;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestMaxScoreBulkScorer;

fn write_documents<R, D>(random: &mut R, dir: Arc<D>) -> Result<()>
where
  R: Rng + ?Sized,
  D: Directory,
{
  let mut iwc = IndexWriterConfig::new();
  iwc.set_merge_policy(new_log_merge_policy(random)?);

  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let docs: Vec<Vec<&str>> = vec![
    vec!["A", "B"],      // 0
    vec!["A"],           // 1
    vec![],              // 2
    vec!["A", "B", "C"], // 3
    vec!["B"],           // 4
    vec!["B", "C"],      // 5
  ];

  for values in docs {
    let mut doc = Document::new();
    for value in values {
      doc.add(StringField::from_string("foo", value, Store::No)?);
    }
    writer.add_document(doc)?;

    for _i in 1..INNER_WINDOW_SIZE {
      writer.add_document(Document::new())?;
    }
  }
  writer.force_merge(1)?;
  writer.close()?;
  Ok(())
}

#[test]
fn test_basics_with_two_disjunction_clauses() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  write_documents(&mut random, dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let clause1: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "A"))),
    2.0,
  )?
  .into();
  let clause2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "B"))).into();

  let context = &searcher.get_leaf_contexts()?[0];

  let w1 = searcher.create_weight(searcher.rewrite(clause1)?, ScoreMode::TopScores, 1.0)?;
  let w2 = searcher.create_weight(searcher.rewrite(clause2)?, ScoreMode::TopScores, 1.0)?;

  let scorer1 = w1
    .scorer(context, &searcher)?
    .expect("expected scorer1 to be present");
  let scorer2 = w2
    .scorer(context, &searcher)?
    .expect("expected scorer2 to be present");

  let max_doc = context.reader().max_doc()?;
  let mut bulk_scorer = MaxScoreBulkScorer::with_no_filter(max_doc, vec![scorer1, scorer2])?;

  let mut collector = LeafCollectorImpl1::new();

  bulk_scorer.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  Ok(())
}

#[test]
fn test_filtered_disjunction() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  write_documents(&mut random, dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let clause1: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "A"))),
    2.0,
  )?
  .into();

  let clause2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "C"))).into();

  let filter: Query = TermQuery::new(Term::from_text("foo", "B")).into();

  let context = &searcher.get_leaf_contexts()?[0];

  let w1 = searcher.create_weight(searcher.rewrite(clause1)?, ScoreMode::TopScores, 1.0)?;
  let w2 = searcher.create_weight(searcher.rewrite(clause2)?, ScoreMode::TopScores, 1.0)?;
  let wf = searcher.create_weight(searcher.rewrite(filter)?, ScoreMode::TopScores, 1.0)?;

  let scorer1 = w1
    .scorer(context, &searcher)?
    .expect("expected scorer1 to be present");
  let scorer2 = w2
    .scorer(context, &searcher)?
    .expect("expected scorer2 to be present");
  let filter_scorer = wf
    .scorer(context, &searcher)?
    .expect("expected filter scorer to be present");

  let max_doc = context.reader().max_doc()?;
  let mut bulk_scorer =
    MaxScoreBulkScorer::new(max_doc, vec![scorer1, scorer2], Some(filter_scorer))?;

  let mut collector = LeafCollectorImpl2::new();

  bulk_scorer.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  Ok(())
}

#[test]
fn test_filtered_disjunction_with_skipping() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  write_documents(&mut random, dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let clause1: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "A"))),
    2.0,
  )?
  .into();

  let clause2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "C"))).into();

  let filter: Query = TermQuery::new(Term::from_text("foo", "B")).into();

  let context = &searcher.get_leaf_contexts()?[0];

  let w1 = searcher.create_weight(searcher.rewrite(clause1)?, ScoreMode::TopScores, 1.0)?;
  let w2 = searcher.create_weight(searcher.rewrite(clause2)?, ScoreMode::TopScores, 1.0)?;
  let wf = searcher.create_weight(searcher.rewrite(filter)?, ScoreMode::TopScores, 1.0)?;

  let scorer1 = w1.scorer(context, &searcher)?.expect("expected scorer1");
  let scorer2 = w2.scorer(context, &searcher)?.expect("expected scorer2");
  let filter_scorer = wf
    .scorer(context, &searcher)?
    .expect("expected filter scorer");

  let max_doc = context.reader().max_doc()?;
  let mut bulk_scorer =
    MaxScoreBulkScorer::new(max_doc, vec![scorer1, scorer2], Some(filter_scorer))?;

  let mut collector = LeafCollectorImpl3::new();

  bulk_scorer.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  Ok(())
}

#[test]
fn test_basics_with_two_disjunction_clauses_and_skipping() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  write_documents(&mut random, dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let clause1: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "A"))),
    2.0,
  )?
  .into();

  let clause2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "B"))).into();

  let context = &searcher.get_leaf_contexts()?[0];

  let w1 = searcher.create_weight(searcher.rewrite(clause1)?, ScoreMode::TopScores, 1.0)?;
  let w2 = searcher.create_weight(searcher.rewrite(clause2)?, ScoreMode::TopScores, 1.0)?;

  let scorer1 = w1.scorer(context, &searcher)?.expect("expected scorer1");
  let scorer2 = w2.scorer(context, &searcher)?.expect("expected scorer2");

  let max_doc = context.reader().max_doc()?;
  let mut bulk_scorer = MaxScoreBulkScorer::with_no_filter(max_doc, vec![scorer1, scorer2])?;

  let mut collector = LeafCollectorImpl4::new();

  bulk_scorer.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  Ok(())
}
#[test]
fn test_basics_with_three_disjunction_clauses() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  write_documents(&mut random, dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let clause1: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "A"))),
    2.0,
  )?
  .into();

  let clause2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "B"))).into();

  let clause3: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "C"))),
    3.0,
  )?
  .into();

  let context = &searcher.get_leaf_contexts()?[0];

  let w1 = searcher.create_weight(searcher.rewrite(clause1)?, ScoreMode::TopScores, 1.0)?;
  let w2 = searcher.create_weight(searcher.rewrite(clause2)?, ScoreMode::TopScores, 1.0)?;
  let w3 = searcher.create_weight(searcher.rewrite(clause3)?, ScoreMode::TopScores, 1.0)?;

  let scorer1 = w1.scorer(context, &searcher)?.expect("expected scorer1");
  let scorer2 = w2.scorer(context, &searcher)?.expect("expected scorer2");
  let scorer3 = w3.scorer(context, &searcher)?.expect("expected scorer3");

  let max_doc = context.reader().max_doc()?;
  let mut bulk_scorer =
    MaxScoreBulkScorer::with_no_filter(max_doc, vec![scorer1, scorer2, scorer3])?;

  let mut collector = LeafCollectorImpl5::new();

  bulk_scorer.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  Ok(())
}
#[test]
fn test_basics_with_three_disjunction_clauses_and_skipping() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  write_documents(&mut random, dir.clone())?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let clause1: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "A"))),
    2.0,
  )?
  .into();

  let clause2: Query = ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "B"))).into();

  let clause3: Query = BoostQuery::new(
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "C"))),
    3.0,
  )?
  .into();

  let context = &searcher.get_leaf_contexts()?[0];

  let w1 = searcher.create_weight(searcher.rewrite(clause1)?, ScoreMode::TopScores, 1.0)?;
  let w2 = searcher.create_weight(searcher.rewrite(clause2)?, ScoreMode::TopScores, 1.0)?;
  let w3 = searcher.create_weight(searcher.rewrite(clause3)?, ScoreMode::TopScores, 1.0)?;

  let scorer1 = w1.scorer(context, &searcher)?.expect("expected scorer1");
  let scorer2 = w2.scorer(context, &searcher)?.expect("expected scorer2");
  let scorer3 = w3.scorer(context, &searcher)?.expect("expected scorer3");

  let max_doc = context.reader().max_doc()?;
  let mut bulk_scorer =
    MaxScoreBulkScorer::with_no_filter(max_doc, vec![scorer1, scorer2, scorer3])?;

  let mut collector = LeafCollectorImpl6::new();

  bulk_scorer.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  Ok(())
}

#[test]
fn test_deletes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = IndexWriterConfig::new();
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut doc1 = Document::new();
  doc1.add(StringField::from_string("field", "foo", Store::No)?);
  doc1.add(StringField::from_string("field", "bar", Store::No)?);
  doc1.add(StringField::from_string("field", "quux", Store::No)?);

  let mut doc2 = Document::new();
  let mut doc3 = Document::new();
  for x in &doc1 {
    doc2.add(x.clone());
    doc3.add(x.clone());
  }

  doc1.add(StringField::from_string("id", "1", Store::No)?);
  doc2.add(StringField::from_string("id", "2", Store::No)?);
  doc3.add(StringField::from_string("id", "3", Store::No)?);

  w.add_document(doc1)?;
  w.add_document(doc2)?;
  w.add_document(doc3)?;

  w.force_merge(1)?;

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;

  let mut builder = Builder::new();
  builder
    .add(
      BoostQuery::new(
        ConstantScoreQuery::new(TermQuery::new(Term::from_text("field", "foo"))),
        1.0,
      )?,
      Occur::Should,
    )?
    .add(
      BoostQuery::new(
        ConstantScoreQuery::new(TermQuery::new(Term::from_text("field", "bar"))),
        1.5,
      )?,
      Occur::Should,
    )?
    .add(
      BoostQuery::new(
        ConstantScoreQuery::new(TermQuery::new(Term::from_text("field", "quux"))),
        0.1,
      )?,
      Occur::Should,
    )?;
  let query: Query = builder.build().into();

  let searcher = new_searcher_with_reader(reader)?;
  let weight = searcher.create_weight(searcher.rewrite(query)?, ScoreMode::TopScores, 1.0)?;

  let live_docs = BitsImpl::new();

  for &min_competitive_score in &[0.0f32, 1.0, 1.2, 2.0] {
    let context = &searcher.get_leaf_contexts()?[0];
    let mut bulk_scorer = weight
      .bulk_scorer(context, &searcher)?
      .expect("expected bulk scorer");

    let mut collector = LeafCollectorImpl7::new(min_competitive_score);

    bulk_scorer.score(&mut collector, Some(&live_docs), 0, NO_MORE_DOCS)?;
    collector.finish()?;
  }

  Ok(())
}

#[test]
fn test_partition() -> Result<()> {
  let mut random = random();
  let mut the = FakeScorer::new("the".to_string());
  the.cost = 9000;
  the.max_score = 0.1;
  the.doc_id = 4;
  the.max_score_up_to = 130;

  let mut quick = FakeScorer::new("quick".to_string());
  quick.cost = 1000;
  quick.max_score = 1.0;
  quick.doc_id = 4;
  quick.max_score_up_to = 999;

  let mut fox = FakeScorer::new("fox".to_string());
  fox.cost = 900;
  fox.max_score = 1.1;
  fox.doc_id = 10;
  fox.max_score_up_to = 1200;

  let scorers = vec![the, quick, fox];
  let mut scorer = MaxScoreBulkScorer::with_no_filter(10_000, scorers)?;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(0, scorer.first_essential_scorer);
  assert_eq!(3, scorer.first_required_scorer);

  // less than the minimum score of every clause
  scorer.scorable.min_competitive_score = 0.09;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(0, scorer.first_essential_scorer);
  assert_eq!(3, scorer.first_required_scorer);

  // equal to the maximum score of `the`
  scorer.scorable.min_competitive_score = 0.1;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(0, scorer.first_essential_scorer);
  assert_eq!(3, scorer.first_required_scorer);

  // gt than the minimum score of `the`
  scorer.scorable.min_competitive_score = 0.11;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(1, scorer.first_essential_scorer);
  assert_eq!(3, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]); // the

  // equal to the sum of the max scores of the and quick
  scorer.scorable.min_competitive_score = 1.1;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(1, scorer.first_essential_scorer);
  assert_eq!(3, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]); // the

  // greater than the sum of the max scores of the and quick
  scorer.scorable.min_competitive_score = 1.11;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(2, scorer.first_essential_scorer);
  assert_eq!(2, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]); // the
  assert_eq!(1, scorer.all_scorers_idx[1]); // quick
  assert_eq!(2, scorer.all_scorers_idx[2]); // fox

  // equal to the sum of the max scores of the and fox
  scorer.scorable.min_competitive_score = 1.2;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(2, scorer.first_essential_scorer);
  assert_eq!(2, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]);
  assert_eq!(1, scorer.all_scorers_idx[1]);
  assert_eq!(2, scorer.all_scorers_idx[2]);

  // greater than the sum of the max scores of the and fox
  scorer.scorable.min_competitive_score = 1.21;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(2, scorer.first_essential_scorer);
  assert_eq!(1, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]);
  assert_eq!(1, scorer.all_scorers_idx[1]);
  assert_eq!(2, scorer.all_scorers_idx[2]);

  // equal to the sum of the max scores of quick and fox
  scorer.scorable.min_competitive_score = 2.1;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(2, scorer.first_essential_scorer);
  assert_eq!(1, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]);
  assert_eq!(1, scorer.all_scorers_idx[1]);
  assert_eq!(2, scorer.all_scorers_idx[2]);

  // greater than the sum of the max scores of quick and fox
  scorer.scorable.min_competitive_score = 2.11;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(2, scorer.first_essential_scorer);
  assert_eq!(0, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]);
  assert_eq!(1, scorer.all_scorers_idx[1]);
  assert_eq!(2, scorer.all_scorers_idx[2]);

  // equal to the sum of the max scores of all terms
  scorer.scorable.min_competitive_score = 2.2;
  scorer.all_scorers_idx.shuffle(&mut random);
  scorer.update_max_window_scores(4, 100)?;
  assert!(scorer.partition_scorers()?);
  assert_eq!(2, scorer.first_essential_scorer);
  assert_eq!(0, scorer.first_required_scorer);
  assert_eq!(0, scorer.all_scorers_idx[0]);
  assert_eq!(1, scorer.all_scorers_idx[1]);
  assert_eq!(2, scorer.all_scorers_idx[2]);

  // greater than the sum of the max scores of all terms
  scorer.scorable.min_competitive_score = 2.21;
  scorer.update_max_window_scores(4, 100)?;
  assert!(!scorer.partition_scorers()?);

  Ok(())
}

struct FakeScorer {
  to_string: String,
  doc_id: i32,
  max_score_up_to: i32,
  max_score: f32,
  cost: i32,
  disi: AllDISI,
}
impl FakeScorer {
  fn new(to_string: String) -> Self {
    let cost = 10;
    let disi = AllDISI::new(cost);
    Self {
      to_string,
      doc_id: -1,
      max_score_up_to: NO_MORE_DOCS,
      max_score: 1.0,
      cost: 10,
      disi,
    }
  }
}

impl Scorable for FakeScorer {
  fn score(&mut self) -> Result<f32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl crate::core::search::scorable::FixedScore for FakeScorer {}

impl Scorer for FakeScorer {
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.doc_id)
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let FakeScorer { disi, .. } = *self;
    Box::new(disi)
  }

  fn advance_shallow(&mut self, _target: i32) -> Result<i32> {
    Ok(self.max_score_up_to)
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(self.max_score)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    No
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }
}

struct LeafCollectorImpl1 {
  i: i32,
}
impl LeafCollectorImpl1 {
  fn new() -> Self {
    Self { i: 0 }
  }
}

impl Display for LeafCollectorImpl1 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl1 {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let idx = self.i;
    self.i += 1;

    match idx {
      0 => {
        assert_eq!(0, doc);
        assert_eq!(3.0, scorer.score()?);
      },
      1 => {
        assert_eq!(4096, doc);
        assert_eq!(2.0, scorer.score()?);
      },
      2 => {
        assert_eq!(12288, doc);
        assert_eq!(3.0, scorer.score()?);
      },
      3 => {
        assert_eq!(16384, doc);
        assert_eq!(1.0, scorer.score()?);
      },
      4 => {
        assert_eq!(20480, doc);
        assert_eq!(1.0, scorer.score()?);
      },
      _ => {
        unreachable!("unexpected collect call");
      },
    }
    Ok(())
  }
}
struct LeafCollectorImpl2 {
  i: i32,
}

impl LeafCollectorImpl2 {
  fn new() -> Self {
    Self { i: 0 }
  }
}

impl Display for LeafCollectorImpl2 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl2 {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let idx = self.i;
    self.i += 1;

    match idx {
      0 => {
        assert_eq!(0, doc);
        assert_eq!(2.0, scorer.score()?);
      },
      1 => {
        assert_eq!(12288, doc);
        assert_eq!(3.0, scorer.score()?);
      },
      2 => {
        assert_eq!(20480, doc);
        assert_eq!(1.0, scorer.score()?);
      },
      _ => {
        unreachable!("unexpected collect call");
      },
    }

    Ok(())
  }
}
struct LeafCollectorImpl3 {
  i: i32,
}

impl LeafCollectorImpl3 {
  fn new() -> Self {
    Self { i: 0 }
  }
}

impl Display for LeafCollectorImpl3 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl3 {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let idx = self.i;
    self.i += 1;

    match idx {
      0 => {
        assert_eq!(0, doc);
        assert_eq!(2.0, scorer.score()?);
        scorer.set_min_competitive_score(2.0f32.next_up())?;
      },
      1 => {
        assert_eq!(12288, doc);
        assert_eq!(3.0, scorer.score()?);
        scorer.set_min_competitive_score(3.0f32.next_up())?;
      },
      _ => {
        println!("{}", self.i);
        unreachable!("unexpected collect call");
      },
    }

    Ok(())
  }
}
struct LeafCollectorImpl4 {
  i: i32,
}

impl LeafCollectorImpl4 {
  fn new() -> Self {
    Self { i: 0 }
  }
}

impl Display for LeafCollectorImpl4 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl4 {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let idx = self.i;
    self.i += 1;

    match idx {
      0 => {
        assert_eq!(0, doc);
        assert_eq!(3.0, scorer.score()?);
      },
      1 => {
        assert_eq!(4096, doc);
        assert_eq!(2.0, scorer.score()?);
        scorer.set_min_competitive_score(2.0f32.next_up())?;
      },
      2 => {
        assert_eq!(12288, doc);
        assert_eq!(3.0, scorer.score()?);
        scorer.set_min_competitive_score(3.0f32.next_up())?;
      },
      _ => {
        unreachable!("unexpected collect call");
      },
    }

    Ok(())
  }
}
struct LeafCollectorImpl5 {
  i: i32,
}

impl LeafCollectorImpl5 {
  fn new() -> Self {
    Self { i: 0 }
  }
}

impl Display for LeafCollectorImpl5 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl5 {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let idx = self.i;
    self.i += 1;

    match idx {
      0 => {
        assert_eq!(0, doc);
        assert_eq!(3.0, scorer.score()?);
      },
      1 => {
        assert_eq!(4096, doc);
        assert_eq!(2.0, scorer.score()?);
      },
      2 => {
        assert_eq!(12288, doc);
        assert_eq!(6.0, scorer.score()?);
      },
      3 => {
        assert_eq!(16384, doc);
        assert_eq!(1.0, scorer.score()?);
      },
      4 => {
        assert_eq!(20480, doc);
        assert_eq!(4.0, scorer.score()?);
      },
      _ => {
        unreachable!("unexpected collect call");
      },
    }

    Ok(())
  }
}
struct LeafCollectorImpl6 {
  i: i32,
}

impl LeafCollectorImpl6 {
  fn new() -> Self {
    Self { i: 0 }
  }
}

impl Display for LeafCollectorImpl6 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl6 {
  fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
    Ok(())
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let idx = self.i;
    self.i += 1;

    match idx {
      0 => {
        assert_eq!(0, doc);
        assert_eq!(3.0, scorer.score()?);
      },
      1 => {
        assert_eq!(4096, doc);
        assert_eq!(2.0, scorer.score()?);
        scorer.set_min_competitive_score(2.0f32.next_up())?;
      },
      2 => {
        assert_eq!(12288, doc);
        assert_eq!(6.0, scorer.score()?);
        scorer.set_min_competitive_score(3.0f32.next_up())?;
      },
      3 => {
        assert_eq!(20480, doc);
        assert_eq!(4.0, scorer.score()?);
        scorer.set_min_competitive_score(4.0f32.next_up())?;
      },
      _ => {
        unreachable!("unexpected collect call");
      },
    }

    Ok(())
  }
}
struct LeafCollectorImpl7 {
  i: i32,
  min_competitive_score: f32,
}

impl LeafCollectorImpl7 {
  fn new(min_competitive_score: f32) -> Self {
    Self {
      i: 0,
      min_competitive_score,
    }
  }
}

impl Display for LeafCollectorImpl7 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>(),)
  }
}

impl LeafCollector for LeafCollectorImpl7 {
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    scorer.set_min_competitive_score(self.min_competitive_score)?;
    Ok(())
  }

  fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    assert_eq!(1, doc);
    assert_eq!(0, self.i);
    self.i += 1;
    Ok(())
  }

  fn finish(&mut self) -> Result<()> {
    assert_eq!(1, self.i);
    Ok(())
  }
}

struct BitsImpl {
  id: Identity,
}
impl BitsImpl {
  fn new() -> Self {
    Self {
      id: Identity::new(),
    }
  }
}

impl HasIdentity for BitsImpl {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl Bits for BitsImpl {
  fn get(&self, index: usize) -> Result<bool> {
    Ok(index == 1)
  }

  fn length(&self) -> usize {
    3
  }
}
