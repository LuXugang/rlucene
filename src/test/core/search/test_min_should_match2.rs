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
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{
  IRCLeafReader, IRCNormNDV, IRCSSDV, IndexReaderContext,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::term::Term;
use crate::core::index::term_states::build as build_term_states;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boolean_weight::BooleanWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::classic_similarity;
use crate::core::search::similarities_impl::similarities::{
  SimScorer, Similarity, SimilaritySimScorer,
};
use crate::core::search::term_query::TermQuery;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::bulk_scorer_wrapper_scorer::BulkScorerWrapperScorer;
use crate::test_framework::core::util::DefaultIndexSearchLR;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, new_directory_shared, new_searcher_with_leaf_reader, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::SliceRandom;
use rand::{Rng, RngExt};
use std::collections::HashSet;

#[allow(dead_code)] // for quick search
pub struct TestMinShouldMatch2;
const ALWAYS_TERMS: &[&str] = &["a"];
const COMMON_TERMS: &[&str] = &["b", "c", "d"];
const MEDIUM_TERMS: &[&str] = &["e", "f", "g"];
const RARE_TERMS: &[&str] = &[
  "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
];

enum Mode {
  Scorer,
  BulkScorer,
  DocValues,
}
fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchLR>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let iw = RandomIndexWriter::new(random, dir.clone())?;
  let num_docs = at_least(random, 300);

  for _ in 0..num_docs {
    let mut doc = Document::new();

    add_some(random, &mut doc, ALWAYS_TERMS)?;

    if random.random_range(0..100) < 90 {
      add_some(random, &mut doc, COMMON_TERMS)?;
    }
    if random.random_range(0..100) < 50 {
      add_some(random, &mut doc, MEDIUM_TERMS)?;
    }
    if random.random_range(0..100) < 10 {
      add_some(random, &mut doc, RARE_TERMS)?;
    }

    iw.add_document(random, doc)?;
  }

  iw.force_merge(random, 1)?;
  let reader = iw.get_reader(random)?;
  iw.close(random)?;

  let mut searcher = new_searcher_with_leaf_reader(get_only_leaf_reader(&reader)?)?;
  searcher.set_similarity(classic_similarity::new());
  Ok(searcher)
}
fn add_some<R>(random: &mut R, doc: &mut Document, values: &[&str]) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut list: Vec<&str> = values.to_vec();
  list.shuffle(random);
  let how_many = TestUtil::next_usize(random, 1, list.len());
  for value in list.iter().take(how_many) {
    doc.add(StringField::from_string(
      "field",
      (*value).to_string(),
      Store::No,
    )?);
    doc.add(SortedSetDocValuesField::new(
      "dv",
      BytesRef::from_string(value),
    ));
  }
  Ok(())
}
fn scorer<R, IRC>(
  random: &mut R,
  values: &[&str],
  min_should_match: i32,
  mode: Mode,
  searcher: &IndexSearcher<IRC>,
) -> Result<Option<Box<dyn Scorer>>>
where
  R: Rng + ?Sized,
  IRC: IndexReaderContext<IndexReader = IRCLeafReader<IRC>>,
{
  let mut bq = Builder::new();
  for value in values {
    bq.add(
      TermQuery::new(Term::from_text("field", value)),
      Occur::Should,
    )?;
  }
  bq.set_minimum_number_should_match(min_should_match);
  let query: Query = bq.build().into();

  let rewritten = searcher.rewrite(query)?;
  let mut weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

  let weight = weight
    .as_any()
    .downcast_ref::<BooleanWeight<IRC>>()
    .unwrap();
  match mode {
    Mode::DocValues => {
      let scorer = SlowMinShouldMatchScorer::new(weight, searcher)?;
      Ok(Some(Box::new(scorer)))
    },
    Mode::Scorer => {
      let ctx = &searcher.get_leaf_contexts()?[0];
      weight.scorer(ctx, searcher)
    },
    Mode::BulkScorer => {
      let ctx = &searcher.get_leaf_contexts()?[0];
      let mut ss = weight
        .scorer_supplier(ctx, searcher)?
        .ok_or_else(|| LuceneError::illegal_state("scorerSupplier is None"))?;

      let bulk_scorer = ss.bulk_scorer(ctx, searcher)?;
      if bulk_scorer.is_none() {
        if weight.scorer(ctx, searcher)?.is_some() {
          panic!("BooleanScorer should be applicable for this query");
        }
        return Ok(None);
      }

      let buffer_size = TestUtil::next_usize(random, 1, 100);

      let wrapper = BulkScorerWrapperScorer::new(bulk_scorer.unwrap(), buffer_size);

      Ok(Some(Box::new(wrapper)))
    },
  }
}
fn assert_next(expected: &mut impl Scorer, actual: Option<&mut impl Scorer>) -> Result<()> {
  if actual.is_none() {
    let mut expected_it = expected.iterator();
    assert_eq!(NO_MORE_DOCS, expected_it.next_doc()?);
    return Ok(());
  }

  let actual = actual.unwrap();

  loop {
    let doc = expected.iterator_mut().next_doc()?;
    if doc == NO_MORE_DOCS {
      break;
    }

    assert_eq!(doc, actual.iterator_mut().next_doc()?);

    let expected_score = expected.score()?;
    let actual_score = actual.score()?;

    assert_eq!(expected_score, actual_score);
  }

  assert_eq!(NO_MORE_DOCS, actual.iterator_mut().next_doc()?);

  Ok(())
}
fn assert_advance(
  expected: &mut impl Scorer,
  actual: Option<&mut impl Scorer>,
  amount: i32,
) -> Result<()> {
  if actual.is_none() {
    let mut expected_it = expected.iterator();
    assert_eq!(NO_MORE_DOCS, expected_it.next_doc()?);
    return Ok(());
  }

  let actual = actual.unwrap();

  let mut prev_doc = 0;

  loop {
    let doc = expected.iterator_mut().advance(prev_doc + amount)?;
    if doc == NO_MORE_DOCS {
      break;
    }

    assert_eq!(doc, actual.iterator_mut().advance(prev_doc + amount)?);

    let expected_score = expected.score()?;
    let actual_score = actual.score()?;

    assert_eq!(expected_score, actual_score);

    prev_doc = doc;
  }

  assert_eq!(
    NO_MORE_DOCS,
    actual.iterator_mut().advance(prev_doc + amount)?
  );

  Ok(())
}

/// test advance with giant bq of all terms with varying minShouldMatch
#[test]
fn test_advance_all_terms() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut terms_list = Vec::new();
  terms_list.extend(COMMON_TERMS.iter().cloned());
  terms_list.extend(MEDIUM_TERMS.iter().cloned());
  terms_list.extend(RARE_TERMS.iter().cloned());

  let terms = &terms_list[..];

  for amount in (25..200).step_by(25) {
    for min_nr_should_match in 1..terms.len() {
      let mut expected = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::DocValues,
        &searcher,
      )?;
      let mut actual = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::Scorer,
        &searcher,
      )?;

      assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;

      let mut expected = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::DocValues,
        &searcher,
      )?;
      let mut actual = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::BulkScorer,
        &searcher,
      )?;

      assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;
    }
  }

  Ok(())
}
/// simple test for next(): minShouldMatch=2 on 3 terms (one common, one medium, one rare)
#[test]
fn test_next_cmr2() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  for common in COMMON_TERMS {
    for medium in MEDIUM_TERMS {
      for rare in RARE_TERMS {
        let terms = [*common, *medium, *rare];

        let mut expected = scorer(&mut random, &terms, 2, Mode::DocValues, &searcher)?;
        let mut actual = scorer(&mut random, &terms, 2, Mode::Scorer, &searcher)?;

        assert_next(expected.as_mut().unwrap(), actual.as_mut())?;

        let mut expected = scorer(&mut random, &terms, 2, Mode::DocValues, &searcher)?;
        let mut actual = scorer(&mut random, &terms, 2, Mode::BulkScorer, &searcher)?;

        assert_next(expected.as_mut().unwrap(), actual.as_mut())?;
      }
    }
  }

  Ok(())
}
/// simple test for advance(): minShouldMatch=2 on 3 terms (one common, one medium, one rare)
#[test]
fn test_advance_cmr2() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  for amount in (25..200).step_by(25) {
    for common in COMMON_TERMS {
      for medium in MEDIUM_TERMS {
        for rare in RARE_TERMS {
          let terms = [*common, *medium, *rare];

          let mut expected = scorer(&mut random, &terms, 2, Mode::DocValues, &searcher)?;
          let mut actual = scorer(&mut random, &terms, 2, Mode::Scorer, &searcher)?;

          assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;

          let mut expected = scorer(&mut random, &terms, 2, Mode::DocValues, &searcher)?;
          let mut actual = scorer(&mut random, &terms, 2, Mode::BulkScorer, &searcher)?;

          assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;
        }
      }
    }
  }

  Ok(())
}
/// test next with giant bq of all terms with varying minShouldMatch
#[test]
fn test_next_all_terms() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut terms_list = Vec::new();
  terms_list.extend(COMMON_TERMS.iter().cloned());
  terms_list.extend(MEDIUM_TERMS.iter().cloned());
  terms_list.extend(RARE_TERMS.iter().cloned());

  let terms = &terms_list[..];

  for min_nr_should_match in 1..terms.len() {
    let mut expected = scorer(
      &mut random,
      terms,
      min_nr_should_match as i32,
      Mode::DocValues,
      &searcher,
    )?;
    let mut actual = scorer(
      &mut random,
      terms,
      min_nr_should_match as i32,
      Mode::Scorer,
      &searcher,
    )?;

    assert_next(expected.as_mut().unwrap(), actual.as_mut())?;

    let mut expected = scorer(
      &mut random,
      terms,
      min_nr_should_match as i32,
      Mode::DocValues,
      &searcher,
    )?;
    let mut actual = scorer(
      &mut random,
      terms,
      min_nr_should_match as i32,
      Mode::BulkScorer,
      &searcher,
    )?;

    assert_next(expected.as_mut().unwrap(), actual.as_mut())?;
  }

  Ok(())
}

/// test advance with giant bq of all terms with varying minShouldMatch
#[test]
fn test_advance_all_terms_again() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut terms_list = Vec::new();
  terms_list.extend(COMMON_TERMS.iter().cloned());
  terms_list.extend(MEDIUM_TERMS.iter().cloned());
  terms_list.extend(RARE_TERMS.iter().cloned());

  let terms = &terms_list[..];

  for amount in (25..200).step_by(25) {
    for min_nr_should_match in 1..terms.len() {
      let mut expected = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::DocValues,
        &searcher,
      )?;
      let mut actual = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::Scorer,
        &searcher,
      )?;

      assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;

      let mut expected = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::DocValues,
        &searcher,
      )?;
      let mut actual = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::BulkScorer,
        &searcher,
      )?;

      assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;
    }
  }

  Ok(())
}

/// test next with varying numbers of terms with varying minShouldMatch
#[test]
fn test_next_varying_number_of_terms() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let mut terms_list = Vec::new();
  terms_list.extend(COMMON_TERMS.iter().cloned());
  terms_list.extend(MEDIUM_TERMS.iter().cloned());
  terms_list.extend(RARE_TERMS.iter().cloned());

  terms_list.shuffle(&mut random);

  for num_terms in 2..=terms_list.len() {
    let terms = &terms_list[0..num_terms];

    for min_nr_should_match in 1..terms.len() {
      let mut expected = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::DocValues,
        &searcher,
      )?;
      let mut actual = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::Scorer,
        &searcher,
      )?;

      assert_next(expected.as_mut().unwrap(), actual.as_mut())?;

      let mut expected = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::DocValues,
        &searcher,
      )?;
      let mut actual = scorer(
        &mut random,
        terms,
        min_nr_should_match as i32,
        Mode::BulkScorer,
        &searcher,
      )?;

      assert_next(expected.as_mut().unwrap(), actual.as_mut())?;
    }
  }

  Ok(())
}
/// test advance with varying numbers of terms with varying minShouldMatch
#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_advance_varying_number_of_terms() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;
  let mut terms_list = Vec::new();
  terms_list.extend(COMMON_TERMS.iter().cloned());
  terms_list.extend(MEDIUM_TERMS.iter().cloned());
  terms_list.extend(RARE_TERMS.iter().cloned());

  terms_list.shuffle(&mut random);

  for amount in (25..200).step_by(25) {
    for num_terms in 2..=terms_list.len() {
      let terms = &terms_list[0..num_terms];

      for min_nr_should_match in 1..terms.len() {
        let mut expected = scorer(
          &mut random,
          terms,
          min_nr_should_match as i32,
          Mode::DocValues,
          &searcher,
        )?;
        let mut actual = scorer(
          &mut random,
          terms,
          min_nr_should_match as i32,
          Mode::Scorer,
          &searcher,
        )?;

        assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;

        let mut expected = scorer(
          &mut random,
          terms,
          min_nr_should_match as i32,
          Mode::DocValues,
          &searcher,
        )?;
        let mut actual = scorer(
          &mut random,
          terms,
          min_nr_should_match as i32,
          Mode::Scorer,
          &searcher,
        )?;

        assert_advance(expected.as_mut().unwrap(), actual.as_mut(), amount)?;
      }
    }
  }

  Ok(())
}
#[allow(dead_code)] // for quick search
pub struct SlowMinShouldMatchScorer<IRC>
where
  IRC: IndexReaderContext,
{
  disi: DocIdSetIteratorImpl<IRC>,
}

impl<IRC> SlowMinShouldMatchScorer<IRC>
where
  IRC: IndexReaderContext,
{
  pub fn new(weight: &BooleanWeight<IRC>, searcher: &IndexSearcher<IRC>) -> Result<Self>
  where
    IRC: IndexReaderContext<IndexReader = IRCLeafReader<IRC>>,
  {
    let reader = searcher.get_index_reader();
    let mut dv = reader.get_sorted_set_doc_values("dv")?.unwrap();
    let max_doc = reader.max_doc()?;
    let bq = &weight.query;
    let min_nr_should_match = bq.get_minimum_number_should_match();

    let value_count = dv.get_value_count()? as usize;
    let mut ords = HashSet::new();
    let mut sims: Vec<Option<SimilaritySimScorer>> = (0..value_count).map(|_| None).collect();

    for clause in bq.clauses() {
      debug_assert!(!clause.is_prohibited());
      debug_assert!(!clause.is_required());

      let Query::Term(term_query) = &clause.query else {
        panic!("SlowMinShouldMatchScorer only supports TermQuery clauses");
      };

      let term = term_query.get_term();
      let ord = dv.lookup_term(term.bytes())?;
      if ord < 0 {
        continue;
      }

      let success = ords.insert(ord);
      debug_assert!(success);

      let ts = build_term_states(searcher, term.clone(), true)?;
      let collection_stats = searcher.collection_statistics("field")?.unwrap();
      let term_stats = searcher.term_statistics(term, ts.doc_freq()?, ts.total_term_freq()?)?;

      sims[ord as usize] = Some(
        weight
          .similarity
          .scorer(1.0, &collection_stats, &[term_stats])?,
      );
    }

    let norms = reader.get_norm_values("field")?;

    Ok(Self {
      disi: DocIdSetIteratorImpl::new(dv, max_doc, ords, sims, norms, min_nr_should_match),
    })
  }
}

impl<IRC> Scorable for SlowMinShouldMatchScorer<IRC>
where
  IRC: IndexReaderContext,
{
  fn score(&mut self) -> Result<f32> {
    debug_assert!(self.disi.score != 0.0, "{}", self.disi.current_matched);
    Ok(self.disi.score as f32)
  }
}

impl<IRC> FixedScore for SlowMinShouldMatchScorer<IRC> where IRC: IndexReaderContext {}

impl<IRC> Scorer for SlowMinShouldMatchScorer<IRC>
where
  IRC: IndexReaderContext + 'static,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.disi.current_doc)
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let SlowMinShouldMatchScorer { disi } = *self;
    Box::new(disi)
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(f32::INFINITY)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.disi)
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.disi)
  }
}

struct DocIdSetIteratorImpl<IRC>
where
  IRC: IndexReaderContext,
{
  current_doc: i32,
  current_matched: i32,

  dv: IRCSSDV<IRC>,
  max_doc: i32,

  ords: HashSet<i64>,
  sims: Vec<Option<SimilaritySimScorer>>,
  norms: Option<IRCNormNDV<IRC>>,
  min_nr_should_match: i32,

  score: f64,
}

impl<IRC> DocIdSetIteratorImpl<IRC>
where
  IRC: IndexReaderContext,
{
  fn new(
    dv: IRCSSDV<IRC>,
    max_doc: i32,
    ords: HashSet<i64>,
    sims: Vec<Option<SimilaritySimScorer>>,
    norms: Option<IRCNormNDV<IRC>>,
    min_nr_should_match: i32,
  ) -> Self {
    Self {
      current_doc: -1,
      current_matched: -1,
      dv,
      max_doc,
      ords,
      sims,
      norms,
      min_nr_should_match,
      score: f64::NAN,
    }
  }
}

impl<IRC> DocIdSetIterator for DocIdSetIteratorImpl<IRC>
where
  IRC: IndexReaderContext,
{
  fn doc_id(&self) -> i32 {
    self.current_doc
  }

  fn next_doc(&mut self) -> Result<i32> {
    debug_assert!(self.current_doc != NO_MORE_DOCS);

    for doc in (self.current_doc + 1)..self.max_doc {
      self.current_doc = doc;
      self.current_matched = 0;
      self.score = 0.0;

      if self.current_doc > self.dv.doc_id() {
        self.dv.advance(self.current_doc)?;
      }
      if self.current_doc != self.dv.doc_id() {
        continue;
      }

      let mut norm = 1_i64;
      if let Some(ref mut norms) = self.norms
        && norms.advance_exact(self.current_doc)?
      {
        norm = norms.long_value()?;
      }

      let count = self.dv.doc_value_count()?;
      for _ in 0..count {
        let ord = self.dv.next_ord()?;
        if self.ords.contains(&ord) {
          self.current_matched += 1;
          if let Some(sim) = &self.sims[ord as usize] {
            self.score += sim.score(1.0, norm) as f64;
          }
        }
      }

      if self.current_matched >= self.min_nr_should_match {
        return Ok(self.current_doc);
      }
    }

    self.current_doc = NO_MORE_DOCS;
    Ok(NO_MORE_DOCS)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let mut doc = self.next_doc()?;
    while doc < target {
      doc = self.next_doc()?;
    }
    Ok(doc)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.max_doc as i64)
  }
}
