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
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::explanation::Explanation;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::tf_idf_similarity::{
  TFIDFSimilarity, TFIDFSimilarityBase, TFIDFSubEnum,
};
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::term_query::TermQuery;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::weight::Weight;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_field, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, new_searcher_with_reader, random,
};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
pub struct TestOmitTF;

#[derive(Clone, Default)]
pub struct SimpleSimilarity1;

pub fn new_simple_similarity1() -> TFIDFSimilarity {
  let v = TFIDFSubEnum::Simple1(SimpleSimilarity1);
  TFIDFSimilarity::new(v)
}

impl TFIDFSimilarityBase for SimpleSimilarity1 {
  fn tf(&self, freq: f32) -> f32 {
    freq
  }

  fn idf_explain(
    &self,
    _collection_stats: &CollectionStatistics,
    _term_stats: &TermStatistics,
  ) -> Explanation {
    Explanation::match_(1.0f32, "Inexplicable".to_string(), vec![])
  }

  fn idf(&self, _doc_freq: i64, _doc_count: i64) -> f32 {
    1.0f32
  }

  fn length_norm(&self, _length: i32) -> f32 {
    1.0f32
  }
}
static OMIT_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  let mut field_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)
    .expect("failed to create OMIT_TYPE");
  field_type
    .set_index_options(IndexOptions::Docs)
    .expect("failed to set index options for OMIT_TYPE");
  field_type
});

static NORMAL_TYPE: LazyLock<FieldType> = LazyLock::new(|| {
  FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)
    .expect("failed to create NORMAL_TYPE")
});
// Make sure first adding docs that do not omitTermFreqAndPositions for
// field X, then adding docs that do omitTermFreqAndPositions for that same
// field,
#[test]
fn test_mixed_ram() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(10);
  let mut mp = new_log_merge_policy_with_merge_factor(&mut random, 2)?;
  mp.get_base_mut().set_no_cfs_ratio(0.0)?;
  iwc.set_merge_policy(mp);
  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut d = Document::new();
  let mut field_to_type = HashMap::new();

  // this field will have Tf
  let f1 = new_field(
    &mut random,
    "f1",
    "This field has term freqs",
    &NORMAL_TYPE,
    &mut field_to_type,
  )?;
  d.add(f1);

  // this field will NOT have Tf
  let f2 = new_field(
    &mut random,
    "f2",
    "This field has NO Tf in all docs",
    &OMIT_TYPE,
    &mut field_to_type,
  )?;
  d.add(f2);

  for _ in 0..5 {
    writer.add_document(d.clone())?;
  }

  for _ in 0..20 {
    writer.add_document(d.clone())?;
  }

  // force merge
  writer.force_merge(1)?;

  // flush
  writer.close()?;

  let reader = directory_reader::open(ram.clone())?;
  let leaf = get_only_leaf_reader(&reader)?;
  let fi = leaf.get_field_infos()?;

  assert_eq!(
    IndexOptions::DocsAndFreqsAndPositions,
    *fi
      .field_info_by_name("f1")
      .ok_or_else(|| LuceneError::illegal_state("field info for f1 is None"))?
      .get_index_options(),
    "OmitTermFreqAndPositions field bit should not be set."
  );
  assert_eq!(
    IndexOptions::Docs,
    *fi
      .field_info_by_name("f2")
      .ok_or_else(|| LuceneError::illegal_state("field info for f2 is None"))?
      .get_index_options(),
    "OmitTermFreqAndPositions field bit should be set."
  );

  Ok(())
}
fn assert_no_prx(dir: &DirEnum) -> Result<()> {
  let files = dir.list_all()?;
  for file in files {
    assert!(!file.ends_with(".prx"));
    assert!(!file.ends_with(".pos"));
  }
  Ok(())
}

// Verifies no *.prx exists when all fields omit term freq:
#[test]
fn test_no_prx_file() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(3);
  iwc.set_max_buffered_docs(10);
  let mut mp = new_log_merge_policy_with_merge_factor(&mut random, 2)?;
  mp.get_base_mut().set_no_cfs_ratio(0.0)?;
  iwc.set_merge_policy(mp);
  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut d = Document::new();
  let mut field_to_type = HashMap::new();

  let f1 = new_field(
    &mut random,
    "f1",
    "This field has term freqs",
    &OMIT_TYPE,
    &mut field_to_type,
  )?;
  d.add(f1);

  for _ in 0..30 {
    writer.add_document(d.clone())?;
  }

  writer.commit()?;

  assert_no_prx(ram.as_ref())?;

  writer.close()?;
  Ok(())
}
#[test]
fn test_basic() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(2);
  iwc.set_similarity(new_simple_similarity1());
  iwc.set_max_buffered_docs(10);
  let mut mp = new_log_merge_policy_with_merge_factor(&mut random, 2)?;
  mp.get_base_mut().set_no_cfs_ratio(0.0)?;
  iwc.set_merge_policy(mp);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let mut sb = String::with_capacity(265);
  let term = "term";
  let mut field_to_type = HashMap::new();

  for i in 0..30 {
    let mut d = Document::new();
    sb.push_str(term);
    sb.push(' ');
    let content = sb.clone();

    let no_tf = new_field(
      &mut random,
      "noTf",
      format!("{}{}", content, if i % 2 == 0 { "" } else { " notf" }),
      &OMIT_TYPE,
      &mut field_to_type,
    )?;
    d.add(no_tf);

    let tf = new_field(
      &mut random,
      "tf",
      format!("{}{}", content, if i % 2 == 0 { " tf" } else { "" }),
      &NORMAL_TYPE,
      &mut field_to_type,
    )?;
    d.add(tf);

    writer.add_document(d)?;
  }

  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_similarity(new_simple_similarity1());

  let a = Term::from_text("noTf", term);
  let b = Term::from_text("tf", term);
  let c = Term::from_text("noTf", "notf");
  let d = Term::from_text("tf", "tf");

  let q1 = TermQuery::new(a.clone());
  let q2 = TermQuery::new(b.clone());
  let q3 = TermQuery::new(c.clone());
  let q4 = TermQuery::new(d.clone());

  let pq = PhraseQuery::from_bytes_no_slop(a.field(), vec![a.bytes().clone(), c.bytes().clone()])?;
  let err = searcher.search(pq, 10);
  debug_assert!(err.is_err());

  let collector_manager = CollectorManagerImpl;
  searcher.search_with_collector_manager(q1.clone(), &collector_manager)?;

  let collector_manager = CollectorManagerImpl1;
  searcher.search_with_collector_manager(q2, &collector_manager)?;

  let collector_manager = CollectorManagerImpl2;
  searcher.search_with_collector_manager(q3, &collector_manager)?;

  let collector_manager = CollectorManagerImpl3;
  searcher.search_with_collector_manager(q4.clone(), &collector_manager)?;

  let mut bq = Builder::new();
  bq.add(q1, Occur::Must)?;
  bq.add(q4, Occur::Must)?;

  let count = searcher.count(bq.build())?;
  assert_eq!(15, count);

  Ok(())
}
#[test]
fn test_stats() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let mut doc = Document::new();
  let mut ft = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  ft.set_index_options(IndexOptions::Docs)?;
  ft.freeze();

  let mut field_to_type = HashMap::new();
  let f = new_field(&mut random, "foo", "bar", &ft, &mut field_to_type)?;
  doc.add(f);

  iw.add_document(&mut random, doc)?;
  let ir = iw.get_reader(&mut random)?;
  iw.close(&mut random)?;

  assert_eq!(
    ir.doc_freq(&Term::new("foo", BytesRef::from_string("bar")))? as i64,
    ir.total_term_freq(&Term::new("foo", BytesRef::from_string("bar")))?
  );
  assert_eq!(
    ir.get_sum_doc_freq("foo")?,
    ir.get_sum_total_term_freq("foo")?
  );

  Ok(())
}

struct CollectorManagerImpl;

impl CollectorManager for CollectorManagerImpl {
  type C = ScoreAssertingCollectorImpl;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ScoreAssertingCollectorImpl::new())
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct ScoreAssertingCollectorImpl;

impl ScoreAssertingCollectorImpl {
  fn new() -> Self {
    Self
  }
}

impl Collector for ScoreAssertingCollectorImpl {
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

impl LeafCollector for ScoreAssertingCollectorImpl {
  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;
    assert!((score - 1.0f32).abs() <= 0.00001f32, "got score={}", score);
    Ok(())
  }
}

impl SimpleCollector for ScoreAssertingCollectorImpl {}

impl Display for ScoreAssertingCollectorImpl {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

struct CollectorManagerImpl1;

impl CollectorManager for CollectorManagerImpl1 {
  type C = ScoreAssertingCollectorImpl1;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ScoreAssertingCollectorImpl1::new())
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct ScoreAssertingCollectorImpl1;

impl ScoreAssertingCollectorImpl1 {
  fn new() -> Self {
    Self
  }
}

impl Collector for ScoreAssertingCollectorImpl1 {
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

impl LeafCollector for ScoreAssertingCollectorImpl1 {
  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;
    let expected = 1.0f32 + doc as f32;
    assert!(
      (expected - score).abs() <= 0.00001f32,
      "expected score={}, actual score={}",
      expected,
      score
    );
    Ok(())
  }
}

impl SimpleCollector for ScoreAssertingCollectorImpl1 {}

impl Display for ScoreAssertingCollectorImpl1 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

struct CollectorManagerImpl2;

impl CollectorManager for CollectorManagerImpl2 {
  type C = ScoreAssertingCollectorImpl2;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ScoreAssertingCollectorImpl2::new())
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct ScoreAssertingCollectorImpl2;

impl ScoreAssertingCollectorImpl2 {
  fn new() -> Self {
    Self
  }
}

impl Collector for ScoreAssertingCollectorImpl2 {
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

impl LeafCollector for ScoreAssertingCollectorImpl2 {
  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;
    assert!((score - 1.0f32).abs() <= 0.00001f32);
    assert_ne!(doc % 2, 0);
    Ok(())
  }
}

impl SimpleCollector for ScoreAssertingCollectorImpl2 {}

impl Display for ScoreAssertingCollectorImpl2 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
struct CollectorManagerImpl3;

impl CollectorManager for CollectorManagerImpl3 {
  type C = ScoreAssertingCollectorImpl3;
  type T = ();

  fn new_collector(&self) -> Result<Self::C> {
    Ok(ScoreAssertingCollectorImpl3::new())
  }

  fn reduce(&self, _collectors: Vec<Self::C>) -> Result<Self::T> {
    Ok(())
  }
}

struct ScoreAssertingCollectorImpl3;

impl ScoreAssertingCollectorImpl3 {
  fn new() -> Self {
    Self
  }
}

impl Collector for ScoreAssertingCollectorImpl3 {
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

impl LeafCollector for ScoreAssertingCollectorImpl3 {
  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    let score = scorer.score()?;
    assert!((score - 1.0f32).abs() <= 0.00001f32);
    assert!(doc % 2 == 0);
    Ok(())
  }
}

impl SimpleCollector for ScoreAssertingCollectorImpl3 {}

impl Display for ScoreAssertingCollectorImpl3 {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
