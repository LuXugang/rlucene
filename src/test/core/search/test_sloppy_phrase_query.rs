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
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::document::text_field::text_field_type;
use crate::core::index::impacts_enum::ImpactsEnumEnum2;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LRNormNumericDocValues;
use crate::core::index::leaf_reader::{LRImpactsEnum, LRPosting};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::slow_impacts_enum::SlowImpactsEnum;
use crate::core::index::term::Term;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::phrase_matcher::PhraseMatcher;
use crate::core::search::phrase_query;
use crate::core::search::phrase_query::{Builder as PhraseQueryBuilder, PhraseQuery};
use crate::core::search::phrase_scorer::PhraseScorer;
use crate::core::search::phrase_weight::SimScorerType;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::DefaultIRCRC;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_field, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[allow(dead_code)]
pub struct TestSloppyPhraseQuery;
macro_rules! s_1 {
  () => {
    "A A A"
  };
}

macro_rules! s_2 {
  () => {
    "A 1 2 3 A 4 5 6 A"
  };
}
const DOC_1: &str = concat!("X ", s_1!(), " Y");
const DOC_2: &str = concat!("X ", s_2!(), " Y");
const DOC_3: &str = concat!("X ", s_1!(), " A Y");
const DOC_1_B: &str = concat!("X ", s_1!(), " Y N N N N ", s_1!(), " Z");
const DOC_2_B: &str = concat!("X ", s_2!(), " Y N N N N ", s_2!(), " Z");
const DOC_3_B: &str = concat!("X ", s_1!(), " A Y N N N N ", s_1!(), " A Y");
const DOC_4: &str = "A A X A X B A X B B A A X B A A";
const DOC_5_3: &str = "H H H X X X H H H X X X H H H";
const DOC_5_4: &str = "H H H H";

const QUERY_1: &str = s_1!();
const QUERY_2: &str = s_2!();
const QUERY_4: &str = "X A A";
const QUERY_5_4: &str = "H H H H";

#[test]
fn test_doc4_query4_all_slops_should_match() -> Result<()> {
  let mut random = random();
  for slop in 0..30 {
    check_phrase_query(
      &mut random,
      make_document(DOC_4)?,
      make_phrase_query(QUERY_4)?,
      slop,
      if slop < 1 { 0 } else { 1 },
    )?;
  }
  Ok(())
}

#[test]
fn test_doc1_query1_all_slops_should_match() -> Result<()> {
  let mut random = random();
  for slop in 0..30 {
    let freq1 = check_phrase_query(
      &mut random,
      make_document(DOC_1)?,
      make_phrase_query(QUERY_1)?,
      slop,
      1,
    )?;
    let freq2 = check_phrase_query(
      &mut random,
      make_document(DOC_1_B)?,
      make_phrase_query(QUERY_1)?,
      slop,
      1,
    )?;
    assert!(
      freq2 > freq1,
      "slop={slop} freq2={freq2} should be greater than freq1 {freq1}"
    );
  }
  Ok(())
}

#[test]
fn test_doc2_query1_slop_6_or_more_should_match() -> Result<()> {
  let mut random = random();
  for slop in 0..30 {
    let expected = if slop < 6 { 0 } else { 1 };
    let freq1 = check_phrase_query(
      &mut random,
      make_document(DOC_2)?,
      make_phrase_query(QUERY_1)?,
      slop,
      expected,
    )?;
    if expected > 0 {
      let freq2 = check_phrase_query(
        &mut random,
        make_document(DOC_2_B)?,
        make_phrase_query(QUERY_1)?,
        slop,
        1,
      )?;
      assert!(
        freq2 > freq1,
        "slop={slop} freq2={freq2} should be greater than freq1 {freq1}"
      );
    }
  }
  Ok(())
}

#[test]
fn test_doc2_query2_all_slops_should_match() -> Result<()> {
  let mut random = random();
  for slop in 0..30 {
    let freq1 = check_phrase_query(
      &mut random,
      make_document(DOC_2)?,
      make_phrase_query(QUERY_2)?,
      slop,
      1,
    )?;
    let freq2 = check_phrase_query(
      &mut random,
      make_document(DOC_2_B)?,
      make_phrase_query(QUERY_2)?,
      slop,
      1,
    )?;
    assert!(
      freq2 > freq1,
      "slop={slop} freq2={freq2} should be greater than freq1 {freq1}"
    );
  }
  Ok(())
}

#[test]
fn test_doc3_query1_all_slops_should_match() -> Result<()> {
  let mut random = random();
  for slop in 0..30 {
    let freq1 = check_phrase_query(
      &mut random,
      make_document(DOC_3)?,
      make_phrase_query(QUERY_1)?,
      slop,
      1,
    )?;
    let freq2 = check_phrase_query(
      &mut random,
      make_document(DOC_3_B)?,
      make_phrase_query(QUERY_1)?,
      slop,
      1,
    )?;
    assert!(
      freq2 > freq1,
      "slop={slop} freq2={freq2} should be greater than freq1 {freq1}"
    );
  }
  Ok(())
}

#[test]
fn test_doc5_query5_any_slop_should_be_consistent() -> Result<()> {
  let mut random = random();
  let n_repeats = 5;
  for slop in 0..3 {
    for _ in 0..n_repeats {
      check_phrase_query(
        &mut random,
        make_document(DOC_5_4)?,
        make_phrase_query(QUERY_5_4)?,
        slop,
        1,
      )?;
    }
    for _ in 0..n_repeats {
      check_phrase_query(
        &mut random,
        make_document(DOC_5_3)?,
        make_phrase_query(QUERY_5_4)?,
        slop,
        0,
      )?;
    }
  }
  Ok(())
}
fn check_phrase_query<R>(
  random: &mut R,
  doc: Document,
  query: PhraseQuery,
  slop: usize,
  expected_num_results: i32,
) -> Result<f32>
where
  R: Rng + ?Sized,
{
  let mut builder = PhraseQueryBuilder::new();
  for (term, position) in query.get_terms().iter().zip(query.get_positions()) {
    builder.add(term.clone(), *position)?;
  }
  builder.set_slop(slop);
  let query = builder.build()?;

  let dir = new_directory_shared(random)?;
  let analyzer = MockAnalyzer::with_automaton(random, mock_tokenizer::WHITESPACE.clone(), false);
  let config = new_index_writer_config_with_analyzer(random, analyzer);
  let writer = RandomIndexWriter::with_config(random, dir.clone(), config);
  writer.add_document(doc)?;
  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  let result = searcher.search_with_collector_manager(query.clone(), &MaxFreqCollectorManager)?;
  assert_eq!(
    expected_num_results, result.total_hits,
    "slop: {slop} query: {query:?} Wrong number of hits"
  );

  writer.close()?;
  Ok(result.max)
}

fn assert_sane_scoring<R>(
  random: &mut R,
  pq: PhraseQuery,
  searcher: &crate::test::core::util::DefaultIndexSearchCR,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  searcher.search_with_collector_manager(pq.clone(), &SaneScoringCollectorManager)?;
  QueryUtils::check_from_searcher(random, pq, searcher)?;
  Ok(())
}

fn make_document(doc_text: &str) -> Result<Document> {
  let mut doc = Document::new();
  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type.set_omit_norms(true)?;
  let f = Field::new("f", doc_text, custom_type);
  doc.add(f);
  Ok(doc)
}

fn make_phrase_query(terms: &str) -> Result<PhraseQuery> {
  let terms = terms.split_whitespace().collect::<Vec<_>>();
  PhraseQuery::from_terms(0, "f", &terms)
}

#[derive(Default)]
struct QueryResult {
  max: f32,
  total_hits: i32,
}

struct MaxFreqCollectorManager;

impl CollectorManager for MaxFreqCollectorManager {
  type C = MaxFreqCollector;
  type T = QueryResult;

  fn new_collector(&self) -> Result<Self::C> {
    Ok(MaxFreqCollector::default())
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    let mut result = QueryResult::default();
    for collector in collectors {
      result.max = result.max.max(collector.max);
      result.total_hits += collector.total_hits;
    }
    Ok(result)
  }
}

#[derive(Default)]
struct MaxFreqCollector {
  max: f32,
  total_hits: i32,
}

impl Collector for MaxFreqCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::do_set_next_reader(self, context)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl LeafCollector for MaxFreqCollector {
  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    self.total_hits += 1;
    let ps = scorer
      .as_any()
      .downcast_mut::<PhraseScorer<
        ImpactsEnumEnum2<
          LRImpactsEnum<IRCLeafReader<DefaultIRCRC>>,
          SlowImpactsEnum<LRPosting<IRCLeafReader<DefaultIRCRC>>>,
        >,
        Arc<SimScorerType>,
        LRNormNumericDocValues<IRCLeafReader<DefaultIRCRC>>,
      >>()
      .unwrap();
    let matcher = &mut ps.disi.two_phase_iterator.matcher;
    let mut freq = matcher.sloppy_weight();
    loop {
      if !matcher.next_match()? {
        break;
      }
      freq += matcher.sloppy_weight();
    }
    self.max = self.max.max(freq);
    Ok(())
  }
}

impl SimpleCollector for MaxFreqCollector {}

impl Display for MaxFreqCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

struct SaneScoringCollectorManager;

impl CollectorManager for SaneScoringCollectorManager {
  type C = SaneScoringCollector;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(SaneScoringCollector)
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct SaneScoringCollector;

impl Collector for SaneScoringCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::do_set_next_reader(self, context)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl LeafCollector for SaneScoringCollector {
  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    assert!(!scorer.score()?.is_infinite());
    Ok(())
  }
}

impl SimpleCollector for SaneScoringCollector {}

impl Display for SaneScoringCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
#[test]
fn test_slop_with_holes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type.set_omit_norms(true)?;
  let mut f = Field::new("lyrics", "", custom_type);
  let mut doc = Document::new();
  doc.add(f.clone());
  writer.add_document(doc)?;

  f.set_string_value("drug drug")?;
  doc = Document::new();
  doc.add(f.clone());
  writer.add_document(doc)?;

  f.set_string_value("drug druggy drug")?;
  doc = Document::new();
  doc.add(f.clone());
  writer.add_document(doc)?;

  f.set_string_value("drug druggy druggy drug")?;
  doc = Document::new();
  doc.add(f.clone());
  writer.add_document(doc)?;

  f.set_string_value("drug druggy drug druggy drug")?;
  doc = Document::new();
  doc.add(f.clone());
  writer.add_document(doc)?;

  let reader = writer.get_reader()?;
  writer.close()?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut builder = PhraseQueryBuilder::new();
  builder.add(Term::from_text("lyrics", "drug"), 1)?;
  builder.add(Term::from_text("lyrics", "drug"), 4)?;
  let pq = builder.clone().build()?;
  assert_eq!(1, searcher.search(pq, 4)?.total_hits().value());
  builder.set_slop(1);
  let pq = builder.clone().build()?;
  assert_eq!(3, searcher.search(pq, 4)?.total_hits().value());
  builder.set_slop(2);
  let pq = builder.build()?;
  assert_eq!(4, searcher.search(pq, 4)?.total_hits().value());

  Ok(())
}

#[test]
fn test_infinite_freq1() -> Result<()> {
  let mut random = random();
  let document = "drug druggy drug drug drug";

  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());
  let mut doc = Document::new();
  let field_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  let mut field_to_type = HashMap::new();
  doc.add(new_field(
    &mut random,
    "lyrics",
    document,
    &field_type,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;
  let ir = iw.get_reader()?;
  iw.close()?;

  let searcher = new_searcher_with_reader(ir)?;

  let mut builder = PhraseQueryBuilder::new();
  builder.add(Term::from_text("lyrics", "drug"), 1)?;
  builder.add(Term::from_text("lyrics", "drug"), 3)?;
  builder.set_slop(1);
  let pq = builder.build()?;

  assert_sane_scoring(&mut random, pq, &searcher)?;

  Ok(())
}
#[test]
fn test_infinite_freq2() -> Result<()> {
  let mut random = random();
  let document = "So much fun to be had in my head \
                No more sunshine \
                So much fun just lying in my bed \
                No more sunshine \
                I can't face the sunlight and the dirt outside \
                Wanna stay in 666 where this darkness don't lie \
                Drug drug druggy \
                Got a feeling sweet like honey \
                Drug drug druggy \
                Need sensation like my baby \
                Show me your scars you're so aware \
                I'm not barbaric I just care \
                Drug drug drug \
                I need a reflection to prove I exist \
                No more sunshine \
                I am a victim of designer blitz \
                No more sunshine \
                Dance like a robot when you're chained at the knee \
                The C.I.A say you're all they'll ever need \
                Drug drug druggy \
                Got a feeling sweet like honey \
                Drug drug druggy \
                Need sensation like my baby \
                Snort your lines you're so aware \
                I'm not barbaric I just care \
                Drug drug druggy \
                Got a feeling sweet like honey \
                Drug drug druggy \
                Need sensation like my baby";
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());

  let mut field_to_type = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_field(
    &mut random,
    "lyrics",
    document,
    &FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?,
    &mut field_to_type,
  )?);
  iw.add_document(doc)?;

  let ir = iw.get_reader()?;
  iw.close()?;

  let searcher = new_searcher_with_reader(ir)?;

  let mut builder = phrase_query::Builder::new();
  builder.add(Term::from_text("lyrics", "drug"), 1)?;
  builder.add(Term::from_text("lyrics", "drug"), 4)?;
  builder.set_slop(5);
  let pq = builder.build()?;

  // "drug the drug"~5
  assert_sane_scoring(&mut random, pq, &searcher)?;

  Ok(())
}
