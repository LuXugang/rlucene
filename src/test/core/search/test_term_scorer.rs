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
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::{IndexReader, IndexReaderBase, LeafReaderContextKind};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::{LeafPostingsEnum, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::knn_collector::KnnCollector;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::similarities_impl::classic_similarity;
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::weight::Weight;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::DefaultIndexSearchLR;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, is_night_mode, new_directory_shared, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_log_merge_policy, new_searcher_with_reader,
  new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[allow(dead_code)]
struct TestTermScorer;

const FIELD: &str = "field";

const VALUES: &[&str] = &["all", "dogs dogs", "like", "playing", "fetch", "all"];
fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchLR>
where
  R: Rng + ?Sized,
{
  let directory = new_directory_shared(random)?;

  let analyzer = MockAnalyzer::new(random);
  let mut iwc = new_index_writer_config_with_analyzer(random, analyzer)?;
  iwc.set_merge_policy(new_log_merge_policy(random)?);
  iwc.set_similarity(classic_similarity::new());

  let writer = RandomIndexWriter::with_config(random, directory.clone(), iwc);
  let mut field_to_type = HashMap::new();
  for value in VALUES.iter() {
    let mut doc = Document::new();
    doc.add(new_text_field(
      random,
      FIELD,
      *value,
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(random, doc)?;
  }

  writer.force_merge(random, 1)?;
  let reader = writer.get_reader(random)?;
  let index_reader = get_only_leaf_reader(&reader)?;
  writer.close(random)?;

  let mut index_searcher = new_searcher_with_reader(index_reader)?;
  index_searcher.set_similarity(classic_similarity::new());

  Ok(index_searcher)
}

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;

  let all_term = Term::from_text(FIELD, "all");
  let term_query = TermQuery::new(all_term);

  let weight = index_searcher.create_weight(term_query, ScoreMode::Complete, 1.0)?;
  let top_reader_context = index_searcher.get_top_reader_context();
  let mut ts = weight
    .bulk_scorer(top_reader_context, &index_searcher)?
    .unwrap();

  let mut collector = SimpleCollectorImpl::new();

  ts.score(&mut collector, None::<&dyn Bits>, 0, NO_MORE_DOCS)?;

  let docs = collector.docs;
  assert_eq!(2, docs.len(), "docs Size: {} is not: 2", docs.len());

  let doc0 = &docs[0];
  let doc5 = &docs[1];
  assert_eq!(
    doc0.score, doc5.score,
    "{} does not equal: {}",
    doc0.score, doc5.score
  );

  Ok(())
}
#[test]
fn test_next() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;

  let all_term = Term::from_text(FIELD, "all");
  let term_query = TermQuery::new(all_term);

  let weight = index_searcher.create_weight(term_query, ScoreMode::Complete, 1.0)?;
  let top_reader_context = index_searcher.get_top_reader_context();

  let mut ts = weight.scorer(top_reader_context, &index_searcher)?.unwrap();
  assert_ne!(
    ts.iterator_mut().next_doc()?,
    NO_MORE_DOCS,
    "next did not return a doc"
  );
  assert_ne!(
    ts.iterator_mut().next_doc()?,
    NO_MORE_DOCS,
    "next did not return a doc"
  );
  assert_eq!(
    ts.iterator_mut().next_doc()?,
    NO_MORE_DOCS,
    "next returned a doc and it should not have"
  );

  Ok(())
}

#[test]
fn test_advance() -> Result<()> {
  let mut random = random();
  let index_searcher = set_up(&mut random)?;

  let all_term = Term::from_text(FIELD, "all");
  let term_query = TermQuery::new(all_term);

  let weight = index_searcher.create_weight(term_query, ScoreMode::Complete, 1.0)?;
  let top_reader_context = index_searcher.get_top_reader_context();

  let mut ts = weight.scorer(top_reader_context, &index_searcher)?.unwrap();
  assert_ne!(ts.iterator_mut().advance(3)?, NO_MORE_DOCS, "Didn't skip");
  assert_eq!(5, ts.doc_id()?, "doc should be number 5");

  Ok(())
}

#[derive(Clone, PartialEq)]
struct TestHit {
  doc: i32,
  score: f32,
}

impl TestHit {
  fn new(doc: i32, score: f32) -> Self {
    Self { doc, score }
  }
}

impl Display for TestHit {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "TestHit{{doc={}, score={}}}", self.doc, self.score)
  }
}
#[test]
fn test_does_not_load_norms() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let all_term = Term::from_text(FIELD, "all");
  let term_query = TermQuery::new(all_term);
  let lr = searcher.get_index_reader().clone();
  let forbidden_norms = (LeafReaderImpl::new(lr)).get_context()?;
  let index_searcher = IndexSearcher::new(forbidden_norms)?;

  let weight = index_searcher.create_weight(term_query.clone(), ScoreMode::Complete, 1.0)?;
  let ctx = index_searcher.get_leaf_contexts()?;
  debug_assert!(ctx.len() == 1);
  let err = weight.scorer(&ctx[0], &index_searcher);
  assert!(matches!(err, Err(LuceneError::IllegalState(_))));

  let weight2 = index_searcher.create_weight(term_query, ScoreMode::CompleteNoScores, 1.0)?;
  let mut scorer = weight2.scorer(&ctx[0], &index_searcher)?.unwrap();
  scorer.iterator_mut().next_doc()?;

  Ok(())
}
#[test]
fn test_random_top_docs() -> Result<()> {
  let mut random = random();
  let _index_searcher = set_up(&mut random)?;
  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random)?;
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let num_docs = if is_night_mode() {
    at_least(&mut random, 128 * 8 * 8 * 3)
  } else {
    at_least(&mut random, 500)
  };

  for _ in 0..num_docs {
    let mut doc = Document::new();

    let shift = random.random_range(0..5);
    let num_values = random.random_range(0..(1 << shift));
    let start = random.random_range(0..10);

    for j in 0..num_values {
      let freq_shift = random.random_range(0..3);
      let freq = TestUtil::next_usize(&mut random, 1, 1 << freq_shift);

      for _ in 0..freq {
        doc.add(TextField::from_string(
          "foo",
          (start + j).to_string(),
          Store::No,
        )?);
      }
    }

    w.add_document(doc)?;
  }

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;
  let searcher = new_searcher_with_reader(reader)?;

  for iter in 0..15 {
    let query = TermQuery::new(Term::from_text("foo", iter.to_string()));

    let complete_manager = TopScoreDocCollectorManager::new(10, i32::MAX as usize)?;
    let top_scores_manager = TopScoreDocCollectorManager::new(10, 1)?;

    let complete = searcher.search_with_collector_manager(query.clone(), &complete_manager)?;
    let top_scores = searcher.search_with_collector_manager(query.clone(), &top_scores_manager)?;
    CheckHits::check_equal(
      &query.clone().into(),
      &complete.score_docs,
      &top_scores.score_docs,
    )?;

    let filter_term = random.random_range(0..15);
    let mut builder = Builder::new();
    builder.add(query.clone(), Occur::Must)?;
    builder.add(
      TermQuery::new(Term::from_text("foo", filter_term.to_string())),
      Occur::Filter,
    )?;
    let filtered_query = builder.build();

    let complete_manager = TopScoreDocCollectorManager::new(10, i32::MAX as usize)?;
    let top_scores_manager = TopScoreDocCollectorManager::new(10, 1)?;

    let complete =
      searcher.search_with_collector_manager(filtered_query.clone(), &complete_manager)?;
    let top_scores = searcher.search_with_collector_manager(filtered_query, &top_scores_manager)?;
    CheckHits::check_equal(&query.into(), &complete.score_docs, &top_scores.score_docs)?;
  }
  Ok(())
}

struct LeafReaderImpl<LR>
where
  LR: LeafReader,
{
  in_: LR,
}
impl<LR> LeafReaderImpl<LR>
where
  LR: LeafReader,
{
  fn new(in_: LR) -> Self {
    Self { in_ }
  }
}

impl<LR> IndexReader for LeafReaderImpl<LR>
where
  LR: LeafReader,
{
  type ContextKind = LeafReaderContextKind;

  type TermVectors = LR::TermVectors;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    self.in_.term_vectors()
  }

  fn max_doc(&self) -> Result<i32> {
    self.in_.max_doc()
  }

  fn num_docs(&self) -> Result<i32> {
    self.in_.num_docs()
  }

  fn num_deleted_docs(&self) -> Result<i32> {
    self.in_.num_deleted_docs()
  }

  fn inc_ref(&self) -> Result<()> {
    self.in_.inc_ref()
  }

  fn dec_ref(&self) -> Result<()> {
    self.in_.dec_ref()
  }

  fn ensure_open(&self) -> Result<()> {
    self.in_.ensure_open()
  }

  type StoredFields = LR::StoredFields;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    self.in_.stored_fields()
  }

  fn has_deletions(&self) -> Result<bool> {
    self.in_.has_deletions()
  }

  fn close(&self) -> Result<()> {
    self.in_.close()
  }

  fn do_close(&self) -> Result<()> {
    self.in_.do_close()
  }

  type ReaderCacheHelper = LR::ReaderCacheHelper;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    self.in_.get_reader_cache_helper()
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    IndexReader::doc_freq(&self.in_, term)
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    self.in_.total_term_freq(term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_doc_freq(&self.in_, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    IndexReader::get_doc_count(&self.in_, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    IndexReader::get_sum_total_term_freq(&self.in_, field)
  }

  fn index_base(&self) -> &IndexReaderBase {
    self.in_.index_base()
  }

  fn try_inc_ref(&self) -> bool {
    self.in_.try_inc_ref()
  }

  fn get_ref_count(&self) -> i32 {
    self.in_.get_ref_count()
  }
}

impl<LR> Display for LeafReaderImpl<LR>
where
  LR: LeafReader,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "LeafReaderImpl")
  }
}

impl<LR> LeafReader for LeafReaderImpl<LR>
where
  LR: LeafReader,
{
  type CacheHelper = LR::CacheHelper;

  fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
    self.in_.get_core_cache_helper()
  }

  fn doc_freq(&self, term: &Term) -> Result<i32>
  where
    Self: Sized,
  {
    LeafReader::doc_freq(&self.in_, term)
  }

  fn get_total_term_freq(&self, term: &Term) -> Result<i64>
  where
    Self: Sized,
  {
    self.in_.get_total_term_freq(term)
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64>
  where
    Self: Sized,
  {
    LeafReader::get_sum_doc_freq(&self.in_, field)
  }

  fn get_doc_count(&self, field: &str) -> Result<i32>
  where
    Self: Sized,
  {
    LeafReader::get_doc_count(&self.in_, field)
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64>
  where
    Self: Sized,
  {
    LeafReader::get_sum_total_term_freq(&self.in_, field)
  }

  type Terms = LR::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    self.in_.terms(field)
  }

  fn postings_with_flag(
    &self,
    term: &Term,
    flags: i32,
  ) -> Result<Option<LeafPostingsEnum<Self::Terms>>>
  where
    Self: Sized,
  {
    self.in_.postings_with_flag(term, flags)
  }

  fn postings(&self, term: &Term) -> Result<Option<LeafPostingsEnum<Self::Terms>>>
  where
    Self: Sized,
  {
    self.in_.postings(term)
  }

  type NumericDocValues = LR::NumericDocValues;

  fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
    self.in_.get_numeric_doc_values(field)
  }

  type BinaryDocValues = LR::BinaryDocValues;

  fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
    self.in_.get_binary_doc_values(field)
  }

  type SortedDocValues = LR::SortedDocValues;

  fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
    self.in_.get_sorted_doc_values(field)
  }

  type SortedNumericDocValues = LR::SortedNumericDocValues;

  fn get_sorted_numeric_doc_values(
    &self,
    field: &str,
  ) -> Result<Option<Self::SortedNumericDocValues>> {
    self.in_.get_sorted_numeric_doc_values(field)
  }

  type SortedSetDocValues = LR::SortedSetDocValues;

  fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
    self.in_.get_sorted_set_doc_values(field)
  }

  type NormNumericDocValues = LR::NormNumericDocValues;

  fn get_norm_values(&self, _field: &str) -> Result<Option<Self::NormNumericDocValues>> {
    Err(LuceneError::illegal_state("Norms are not available"))
  }

  type DocValuesSkipper = LR::DocValuesSkipper;

  fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
    self.in_.get_doc_values_skipper(field)
  }

  type FloatVectorValues = LR::FloatVectorValues;

  fn get_float_vector_values(&self, field: &str) -> Result<Option<Self::FloatVectorValues>> {
    self.in_.get_float_vector_values(field)
  }

  type ByteVectorValues = LR::ByteVectorValues;

  fn get_byte_vector_values(&self, field: &str) -> Result<Option<Self::ByteVectorValues>> {
    self.in_.get_byte_vector_values(field)
  }

  fn search_nearest_vectors_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    self
      .in_
      .search_nearest_vectors_f32(field, target, knn_collector, accept_docs)
  }

  fn search_nearest_vectors_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: KnnCollector,
  {
    self
      .in_
      .search_nearest_vectors_u8(field, target, knn_collector, accept_docs)
  }

  fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
    self.in_.get_field_infos()
  }

  type Bits = LR::Bits;

  fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
    self.in_.get_live_docs()
  }

  type PointValues = LR::PointValues;

  fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
    self.in_.get_point_values(field)
  }

  fn check_integrity(&self) -> Result<()> {
    self.in_.check_integrity()
  }

  fn get_metadata(&self) -> Result<&LeafMetaData> {
    self.in_.get_metadata()
  }
}
struct SimpleCollectorImpl {
  base: i32,
  docs: Vec<TestHit>,
}
impl SimpleCollectorImpl {
  fn new() -> Self {
    Self {
      base: 0,
      docs: Vec::new(),
    }
  }
}

impl Collector for SimpleCollectorImpl {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext + 'a;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::get_leaf_collector(self, context, weight)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl LeafCollector for SimpleCollectorImpl {
  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;
    let doc = doc + self.base;
    self.docs.push(TestHit::new(doc, score));

    assert!(score > 0.0, "score {} is not greater than 0", score);
    assert!(
      doc == 0 || doc == 5,
      "Doc: {} does not equal 0 or doc does not equal 5",
      doc
    );

    Ok(())
  }
}

impl Display for SimpleCollectorImpl {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl SimpleCollector for SimpleCollectorImpl {
  fn do_set_next_reader<LR>(&mut self, context: &LeafReaderContext<LR>) -> Result<()>
  where
    LR: LeafReader,
  {
    self.base = context.doc_base as i32;
    Ok(())
  }
}
