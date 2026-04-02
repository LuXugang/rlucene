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
use crate::core::search::doc_id_set_iterator::{
  DocIdSetIterator, DocIdSetIteratorEnum2, EmptyDISI,
};
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
#[cfg(test)]
use crate::core::search::scorer::ScorerKind;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::{
  EmptyTPI, TwoPhaseIterator, TwoPhaseIteratorAsDocIdSetIterator, TwoPhaseIteratorEnum2,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// A constant-scoring Scorer.
pub struct ConstantScoreScorer<DISI, TPI>
where
  DISI: DocIdSetIterator,
  TPI: TwoPhaseIterator,
{
  score: f32,
  score_mode: ScoreMode,
  disi: ConstantDISI_<DISI, TPI>,
  tpi_state: TwoPhaseState,
}
impl<DISI> ConstantScoreScorer<DISI, DummyTwoPhaseIterator>
where
  DISI: DocIdSetIterator,
{
  /// Constructor based on a [`DocIdSetIterator`] used to drive iteration. Two-phase
  /// iteration is not supported.
  ///
  /// # Parameters
  /// - `score`: the score to return on each document.
  /// - `score_mode`: the score mode.
  /// - `disi`: the iterator that defines matching documents.
  pub fn from_disi(score: f32, score_mode: ScoreMode, disi: DISI) -> Self {
    let approximation = match score_mode {
      ScoreMode::TopScores => {
        ConstantDISI::A(DocIdSetIteratorWrapper::new(DelegateEnum::Disi(disi)))
      },
      _ => ConstantDISI::B(disi),
    };
    Self {
      score,
      score_mode,
      disi: DocIdSetIteratorEnum2::A(approximation),
      tpi_state: TwoPhaseState::No,
    }
  }
}
impl<TPI> ConstantScoreScorer<DummyDISI, TPI>
where
  TPI: TwoPhaseIterator,
{
  /// Constructor based on a [`TwoPhaseIterator`]. In this case the `Scorer` will
  /// support two-phase iteration.
  ///
  /// # Parameters
  /// - `score`: the score to return on each document.
  /// - `score_mode`: the score mode.
  /// - `two_phase_iterator`: the iterator that defines matching documents.
  pub fn from_tpi(score: f32, score_mode: ScoreMode, two_phase_iterator: TPI) -> Self {
    let two_phase_iterator = match score_mode {
      ScoreMode::TopScores => {
        let v: DocIdSetIteratorWrapper<TPI, DummyDISI> =
          DocIdSetIteratorWrapper::new(DelegateEnum::TPI(two_phase_iterator));
        ConstantTPI::A(TwoPhaseIteratorImpl::new(v))
      },
      _ => ConstantTPI::B(two_phase_iterator),
    };
    Self {
      score,
      score_mode,
      disi: DocIdSetIteratorEnum2::B(TwoPhaseIteratorAsDocIdSetIterator::new(two_phase_iterator)),
      tpi_state: TwoPhaseState::Yes,
    }
  }
}

impl<DISI, TPI> Scorable for ConstantScoreScorer<DISI, TPI>
where
  DISI: DocIdSetIterator + 'static,
  TPI: TwoPhaseIterator + 'static,
{
  fn score(&mut self) -> Result<f32> {
    Ok(self.score)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    if min_score > self.score && matches!(self.score_mode, ScoreMode::TopScores) {
      match self.disi {
        ConstantDISI_::A(ref mut v) => match v {
          DocIdSetIteratorEnum2::A(v) => {
            v.delegate = DelegateEnum::EmptyDisi(EmptyDISI::new());
          },
          DocIdSetIteratorEnum2::B(_) => {
            return Err(LuceneError::illegal_state("TopScores: should not be here"));
          },
        },
        ConstantDISI_::B(ref mut v) => match v.two_phase_iterator {
          TwoPhaseIteratorEnum2::A(ref mut wrapper) => {
            wrapper.approximation.delegate = DelegateEnum::EmptyTPI(EmptyTPI);
          },
          TwoPhaseIteratorEnum2::B(_) => {
            return Err(LuceneError::illegal_state("TopScores: should not be here"));
          },
        },
      }
    }
    Ok(())
  }

  fn cost(&self) -> Result<i64> {
    self.iterator().cost()
  }
}

impl<DISI, TPI> crate::core::search::scorable::FixedScore for ConstantScoreScorer<DISI, TPI>
where
  DISI: DocIdSetIterator + 'static,
  TPI: TwoPhaseIterator + 'static,
{
}

impl<DISI, TPI> Scorer for ConstantScoreScorer<DISI, TPI>
where
  DISI: DocIdSetIterator + 'static,
  TPI: TwoPhaseIterator + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.doc_id())
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    match &self.disi {
      ConstantDISI_::A(v) => match v {
        DocIdSetIteratorEnum2::A(wrapper) => Box::new(wrapper),
        DocIdSetIteratorEnum2::B(disi) => Box::new(disi),
      },
      ConstantDISI_::B(v) => Box::new(v),
    }
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match &mut self.disi {
      ConstantDISI_::A(v) => match v {
        DocIdSetIteratorEnum2::A(wrapper) => Box::new(wrapper),
        DocIdSetIteratorEnum2::B(disi) => Box::new(disi),
      },
      ConstantDISI_::B(v) => Box::new(v),
    }
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let ConstantScoreScorer { disi, .. } = *self;
    match disi {
      ConstantDISI_::A(v) => match v {
        DocIdSetIteratorEnum2::A(wrapper) => Box::new(wrapper),
        DocIdSetIteratorEnum2::B(disi) => Box::new(disi),
      },
      ConstantDISI_::B(v) => Box::new(v),
    }
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match self.disi {
        ConstantDISI_::A(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        ConstantDISI_::B(ref v) => Some(Box::new(&v.two_phase_iterator)),
      },
    }
  }

  fn two_phase_iterator_mut(&mut self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    match self.tpi_state {
      TwoPhaseState::No => None,
      _ => match self.disi {
        ConstantDISI_::A(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        ConstantDISI_::B(ref mut v) => Some(Box::new(&mut v.two_phase_iterator)),
      },
    }
  }

  fn take_two_phase_iterator(self: Box<Self>) -> Option<Box<dyn TwoPhaseIterator>> {
    let ConstantScoreScorer {
      disi, tpi_state, ..
    } = *self;
    match tpi_state {
      TwoPhaseState::No => None,
      _ => match disi {
        ConstantDISI_::A(_) => {
          debug_assert!(false, "should not be here");
          None
        },
        ConstantDISI_::B(wrapper) => Some(Box::new(wrapper.two_phase_iterator)),
      },
    }
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(self.score)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    self.tpi_state
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator_mut(),
      _ => match self.disi {
        ConstantDISI_::A(_) => self.iterator_mut(),
        ConstantDISI_::B(ref mut v) => v.two_phase_iterator.approximation_mut(),
      },
    }
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    match self.tpi_state {
      TwoPhaseState::No => self.iterator(),
      _ => match self.disi {
        ConstantDISI_::A(_) => self.iterator(),
        ConstantDISI_::B(ref v) => v.two_phase_iterator.approximation(),
      },
    }
  }
  #[cfg(test)]
  fn kind(&self) -> ScorerKind {
    ScorerKind::ConstantScore
  }
}

pub struct TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  approximation: DocIdSetIteratorWrapper<TPI, DummyDISI>,
}
impl<TPI> TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  pub fn new(approximation: DocIdSetIteratorWrapper<TPI, DummyDISI>) -> Self {
    Self { approximation }
  }
}
impl<TPI> TwoPhaseIterator for TwoPhaseIteratorImpl<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.approximation)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.approximation)
  }

  fn matches(&mut self) -> Result<bool> {
    match self.approximation.delegate {
      DelegateEnum::TPI(ref mut t) => t.matches(),
      DelegateEnum::EmptyTPI(ref mut t) => t.matches(),
      _ => unreachable!("should not be here"),
    }
  }

  fn match_cost(&self) -> f32 {
    match self.approximation.delegate {
      DelegateEnum::TPI(ref t) => t.match_cost(),
      DelegateEnum::EmptyTPI(ref t) => t.match_cost(),
      _ => unreachable!("should not be here"),
    }
  }
}

// used for Constructor from DISI
pub type ConstantDISI<DISI> =
  DocIdSetIteratorEnum2<DocIdSetIteratorWrapper<DummyTwoPhaseIterator, DISI>, DISI>;
// used Constructor from TwoPhaseIterator
pub type ConstantTPI<TPI> = TwoPhaseIteratorEnum2<TwoPhaseIteratorImpl<TPI>, TPI>;

pub type ConstantDISI_<DISI, TPI> =
  DocIdSetIteratorEnum2<ConstantDISI<DISI>, TwoPhaseIteratorAsDocIdSetIterator<ConstantTPI<TPI>>>;

pub enum DelegateEnum<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
  TPI(T),
  EmptyTPI(EmptyTPI),
  Disi(D),
  EmptyDisi(EmptyDISI),
}
impl<T, D> DocIdSetIterator for DelegateEnum<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    match self {
      DelegateEnum::TPI(t) => t.approximation().doc_id(),
      DelegateEnum::EmptyTPI(t) => t.approximation().doc_id(),
      DelegateEnum::Disi(d) => d.doc_id(),
      DelegateEnum::EmptyDisi(e) => e.doc_id(),
    }
  }

  fn next_doc(&mut self) -> Result<i32> {
    match self {
      DelegateEnum::TPI(t) => t.approximation_mut().next_doc(),
      DelegateEnum::EmptyTPI(t) => t.approximation_mut().next_doc(),
      DelegateEnum::Disi(d) => d.next_doc(),
      DelegateEnum::EmptyDisi(e) => e.next_doc(),
    }
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    match self {
      DelegateEnum::TPI(t) => t.approximation_mut().advance(_target),
      DelegateEnum::EmptyTPI(t) => t.approximation_mut().advance(_target),
      DelegateEnum::Disi(d) => d.advance(_target),
      DelegateEnum::EmptyDisi(e) => e.advance(_target),
    }
  }

  fn slow_advance(&mut self, target: i32) -> Result<i32> {
    match self {
      DelegateEnum::TPI(t) => t.approximation_mut().slow_advance(target),
      DelegateEnum::EmptyTPI(t) => t.approximation_mut().slow_advance(target),
      DelegateEnum::Disi(d) => d.slow_advance(target),
      DelegateEnum::EmptyDisi(e) => e.slow_advance(target),
    }
  }

  fn cost(&self) -> Result<i64> {
    match self {
      DelegateEnum::TPI(t) => t.approximation().cost(),
      DelegateEnum::EmptyTPI(t) => t.approximation().cost(),
      DelegateEnum::Disi(d) => d.cost(),
      DelegateEnum::EmptyDisi(e) => e.cost(),
    }
  }
}

pub struct DocIdSetIteratorWrapper<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
  doc: i32,
  delegate: DelegateEnum<T, D>,
}

impl<T, D> DocIdSetIteratorWrapper<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
  pub fn new(delegate: DelegateEnum<T, D>) -> Self {
    Self { doc: -1, delegate }
  }
}

impl<T, D> DocIdSetIterator for DocIdSetIteratorWrapper<T, D>
where
  T: TwoPhaseIterator,
  D: DocIdSetIterator,
{
  fn doc_id(&self) -> i32 {
    self.doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    self.doc = self.delegate.next_doc()?;
    Ok(self.doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc = self.delegate.advance(target)?;
    Ok(self.doc)
  }

  fn cost(&self) -> Result<i64> {
    self.delegate.cost()
  }
}
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::{BooleanQuery, Builder};
  use crate::core::search::constant_score_query::ConstantScoreQuery;
  use crate::core::search::constant_score_scorer::ConstantScoreScorer;
  use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
  use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
  use crate::core::search::dummy::dummy_disi::DummyDISI;
  use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
  use crate::core::search::phrase_query::PhraseQuery;
  use crate::core::search::query::Query;
  use crate::core::search::scorable::Scorable;
  use crate::core::search::score_mode::ScoreMode;
  use crate::core::search::scorer::{Scorer, ScorerEnum2, TwoPhaseState};
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
  use crate::core::search::total_hits::Relation::GreaterThanOrEqualTo;
  use crate::core::search::two_phase_iterator::TwoPhaseIterator;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use rand::Rng;
  use std::collections::HashMap;
  use std::sync::LazyLock;

  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
    new_log_merge_policy, new_searcher_with_reader, new_text_field, random,
  };

  #[allow(dead_code)] // for quick search
  struct TestConstantScoreScorer;
  pub static FIELD: &str = "f";

  pub static VALUES: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
      "foo".to_string(),
      "bar".to_string(),
      "foo bar".to_string(),
      "bar foo".to_string(),
      "foo not bar".to_string(),
      "bar foo bar".to_string(),
      "azerty".to_string(),
    ]
  });

  pub static TERM_QUERY: LazyLock<BooleanQuery> = LazyLock::new(|| {
    let mut builder = Builder::new();
    builder
      .add(TermQuery::new(Term::from_text(FIELD, "foo")), Occur::Must)
      .unwrap();
    builder
      .add(TermQuery::new(Term::from_text(FIELD, "bar")), Occur::Must)
      .unwrap();
    builder.build()
  });

  pub static PHRASE_QUERY: LazyLock<PhraseQuery> =
    LazyLock::new(|| PhraseQuery::from_terms_no_slop(FIELD, &["foo", "bar"]).unwrap());
  #[test]
  fn test_matching_score_mode_complete() -> Result<()> {
    let mut random = random();
    test_matching(&mut random, ScoreMode::Complete)
  }

  #[test]
  fn test_matching_score_mode_complete_no_scores() -> Result<()> {
    let mut random = random();
    test_matching(&mut random, ScoreMode::CompleteNoScores)
  }
  fn test_matching<R: Rng + ?Sized>(random: &mut R, score_mode: ScoreMode) -> Result<()> {
    let mut scorer = constant_score_scorer(random, TERM_QUERY.clone(), 1.0, score_mode)?;

    let mut doc;

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(2, doc);
    assert!((scorer.score()? - 1.0).abs() <= 0.0);

    scorer.set_min_competitive_score(2.0)?;
    assert_eq!(doc, scorer.doc_id()?);
    assert_eq!(doc, scorer.iterator().doc_id());
    assert!((scorer.score()? - 1.0).abs() <= 0.0);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(3, doc);
    assert!((scorer.score()? - 1.0).abs() <= 0.0);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(4, doc);
    assert!((scorer.score()? - 1.0).abs() <= 0.0);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(5, doc);
    assert!((scorer.score()? - 1.0).abs() <= 0.0);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(NO_MORE_DOCS, doc);

    Ok(())
  }

  #[test]
  fn test_matching_score_mode_top_scores() -> Result<()> {
    let mut random = random();

    let mut scorer =
      constant_score_scorer(&mut random, TERM_QUERY.clone(), 1.0, ScoreMode::TopScores)?;

    let mut doc;

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(2, doc);
    assert_eq!(1.0, scorer.score()?);

    scorer.set_min_competitive_score(2.0)?;
    assert_eq!(doc, scorer.doc_id()?);
    assert_eq!(doc, scorer.iterator().doc_id());
    assert_eq!(1.0, scorer.score()?);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(NO_MORE_DOCS, doc);

    Ok(())
  }
  #[test]
  fn test_two_phase_matching_score_mode_complete() -> Result<()> {
    let mut random = random();
    test_two_phase_matching(&mut random, ScoreMode::Complete)
  }

  #[test]
  fn test_two_phase_matching_score_mode_complete_no_scores() -> Result<()> {
    let mut random = random();
    test_two_phase_matching(&mut random, ScoreMode::CompleteNoScores)
  }

  fn test_two_phase_matching<R: Rng + ?Sized>(random: &mut R, score_mode: ScoreMode) -> Result<()> {
    let mut scorer = constant_score_scorer(random, PHRASE_QUERY.clone(), 1.0, score_mode)?;

    let mut doc;

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(2, doc);
    assert_eq!(1.0, scorer.score()?);

    scorer.set_min_competitive_score(2.0)?;
    assert_eq!(doc, scorer.doc_id()?);
    assert_eq!(doc, scorer.iterator().doc_id());
    assert_eq!(1.0, scorer.score()?);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(5, doc);
    assert_eq!(1.0, scorer.score()?);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(NO_MORE_DOCS, doc);

    Ok(())
  }
  #[test]
  fn test_two_phase_matching_score_mode_top_scores() -> Result<()> {
    let mut random = random();

    let mut scorer =
      constant_score_scorer(&mut random, PHRASE_QUERY.clone(), 1.0, ScoreMode::TopScores)?;

    let mut doc;

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(2, doc);
    assert_eq!(1.0, scorer.score()?);

    scorer.set_min_competitive_score(2.0)?;
    assert_eq!(doc, scorer.doc_id()?);
    assert_eq!(doc, scorer.iterator().doc_id());
    assert_eq!(1.0, scorer.score()?);

    doc = scorer.iterator_mut().next_doc()?;
    assert_eq!(NO_MORE_DOCS, doc);

    Ok(())
  }
  fn constant_score_scorer<R: Rng + ?Sized, T: Into<Query>>(
    random: &mut R,
    query: T,
    score: f32,
    score_mode: ScoreMode,
  ) -> Result<Scorers> {
    let query = query.into();
    let directory = new_directory_shared(random)?;

    let mut iwc = new_index_writer_config(random);
    iwc.set_merge_policy(new_log_merge_policy(random)?);

    let writer = RandomIndexWriter::with_config(random, directory.clone(), iwc);
    let mut field_to_type = HashMap::new();

    for value in VALUES.iter() {
      let mut doc = Document::new();
      doc.add(new_text_field(
        random,
        FIELD,
        value,
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;
    }

    writer.force_merge(1)?;
    let reader = writer.get_reader()?;
    writer.close()?;
    let searcher = new_searcher_with_reader(reader)?;
    let weight = searcher.create_weight(ConstantScoreQuery::new(query), score_mode, 1.0)?;

    let leaves = searcher.get_top_reader_context().leaves()?;
    assert_eq!(1, leaves.len());

    let context = &leaves[0];
    let scorer = weight
      .scorer(context, &searcher)?
      .ok_or_else(|| LuceneError::illegal_state("scorer is None"))?;
    let has_tpi = scorer.has_two_phase_iterator() == TwoPhaseState::Yes;
    let v = if has_tpi {
      ScorerEnum2::A(ConstantScoreScorer::from_tpi(
        score,
        score_mode,
        scorer.take_two_phase_iterator().unwrap(),
      ))
    } else {
      ScorerEnum2::B(ConstantScoreScorer::from_disi(
        score,
        score_mode,
        scorer.take_iterator(),
      ))
    };
    Ok(v)
  }
  #[test]
  fn test_early_termination() -> Result<()> {
    let mut random = random();

    let analyzer = MockAnalyzer::new(&mut random);
    let dir = new_directory_shared(&mut random)?;

    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    iwc.set_max_buffered_docs(2);
    iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

    let iw = IndexWriter::new(dir.clone(), iwc)?;

    let num_docs = 50;
    let mut field_to_type = HashMap::new();

    for i in 0..num_docs {
      let mut doc = Document::new();
      let value = if i % 2 == 0 { "foo bar" } else { "baz" };
      doc.add(new_text_field(
        &mut random,
        "key",
        value,
        Store::Yes,
        &mut field_to_type,
      )?);
      iw.add_document(doc)?;
    }

    let ir = directory_reader_util::open_from_writer(&iw)?;

    let is = new_searcher_with_reader(ir)?;

    let mut c = TopScoreDocCollectorManager::new(10, 10)?;
    let top_docs = is.search_with_collector_manager(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("key", "foo"))),
      &c,
    )?;
    assert_eq!(11, top_docs.total_hits.value());
    assert_eq!(GreaterThanOrEqualTo, top_docs.total_hits.relation());

    c = TopScoreDocCollectorManager::new(10, 10)?;
    let mut builder = Builder::new();
    builder.add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("key", "foo"))),
      Occur::Should,
    )?;
    builder.add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("key", "bar"))),
      Occur::Filter,
    )?;
    let query = builder.build();

    let top_docs = is.search_with_collector_manager(query, &c)?;
    assert_eq!(11, top_docs.total_hits.value());
    assert_eq!(GreaterThanOrEqualTo, top_docs.total_hits.relation());

    iw.close()?;
    Ok(())
  }
  type Scorers = ScorerEnum2<
    ConstantScoreScorer<DummyDISI, Box<dyn TwoPhaseIterator>>,
    ConstantScoreScorer<Box<dyn DocIdSetIterator>, DummyTwoPhaseIterator>,
  >;
}
