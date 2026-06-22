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
use crate::core::analysis::analyzer::AnalyzerEnum;
use crate::core::document::fields::Fields;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_writer::{
  DocStats, IndexCommitWrapper, IndexWriter, IndexWriterHooks, IndexWriterHooksEnum,
};
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::query::Query;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStream, InfoStreamEnum, InfoStreamMT};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::new_index_writer_config_with_analyzer;
use crate::test::core::util::null_info_stream::NullInfoStream;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::sync::Arc;
use std::thread;

pub struct RandomIndexWriter<D>
where
  D: Directory,
{
  pub(crate) w: IndexWriter<D>,
}

impl<D> RandomIndexWriter<D>
where
  D: Directory,
{
  pub fn new<R>(r: &mut R, dir: Arc<D>) -> Self
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    let a = MockAnalyzer::new(r);
    let config = new_index_writer_config_with_analyzer(r, a);
    Self {
      w: IndexWriter::new(dir, config).expect("should not fail"),
    }
  }
  pub fn with_analyzer<R, T>(r: &mut R, dir: Arc<D>, analyzer: T) -> Self
  where
    R: Rng + ?Sized,
    D: Directory,
    T: Into<AnalyzerEnum>,
  {
    let config = new_index_writer_config_with_analyzer(r, analyzer);
    Self::with_config(r, dir, config)
  }
  pub fn with_config<R>(_r: &mut R, dir: Arc<D>, config: IndexWriterConfig) -> Self
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    Self {
      w: IndexWriter::new(dir, config).expect("should not fail"),
    }
  }
  pub fn with_soft_deletes<R>(
    _r: &mut R,
    dir: Arc<D>,
    config: IndexWriterConfig,
    _use_soft_deletes: bool,
  ) -> Self
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    Self {
      w: IndexWriter::new(dir, config).expect("should not fail"),
    }
  }
  /// Returns an indexwriter that randomly mixes up thread scheduling (by yielding at test points).
  pub fn mock_index_writer<R>(
    dir: Arc<D>,
    conf: IndexWriterConfig,
    r: &mut R,
  ) -> Result<IndexWriter<D>>
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    // Randomly calls Thread.yield so we mixup thread scheduling.
    let random = StdRng::seed_from_u64(r.random());
    Self::mock_index_writer_with_test_point(r, dir, conf, YieldTestPoint::new(random))
  }

  /// Returns an indexwriter that enables the specified test point.
  pub fn mock_index_writer_with_test_point<R, TP>(
    r: &mut R,
    dir: Arc<D>,
    mut conf: IndexWriterConfig,
    test_point: TP,
  ) -> Result<IndexWriter<D>>
  where
    R: Rng + ?Sized,
    D: Directory,
    TP: TestPoint + 'static,
  {
    let info_stream = conf.get_info_stream();
    conf.set_info_stream(InfoStreamEnum::Custom(Box::new(TestPointInfoStream::new(
      info_stream,
      test_point,
    ))));

    if r.random()
      && directory_reader::index_exists(dir.as_ref())?
      && *conf.get_open_mode() != OpenMode::Create
    {
      if cfg!(feature = "test_log_verbose") {
        println!("RIW: open writer from reader");
      }
      let reader = directory_reader::open(dir.clone())?;
      let commit = reader.get_index_commit()?;
      IndexWriter::with_index_commit_and_hook(
        dir,
        conf,
        Some(IndexWriterHooksEnum::custom(TestPointsIndexWriterHooks)),
        IndexCommitWrapper::new(Some(commit), Some(reader), None)?,
      )
    } else {
      IndexWriter::with_hooks(
        dir,
        conf,
        Some(IndexWriterHooksEnum::custom(TestPointsIndexWriterHooks)),
      )
    }
  }

  pub fn get_reader(&self) -> Result<StandardDirectoryReaderType<D>> {
    self.w.get_reader(true, false)
  }
  pub fn add_document<DF>(&self, doc: DF) -> Result<i64>
  where
    DF: IntoIterator<Item = Fields>,
  {
    self.w.add_document(doc)
  }
  pub fn delete_documents_with_terms(&self, terms: Vec<Term>) -> Result<i64> {
    self.w.delete_documents_with_terms(terms)
  }
  pub fn delete_documents_with_query(&self, terms: Vec<Query>) -> Result<i64> {
    self.w.delete_documents_with_queries(terms)
  }
  pub fn delete_all(&self) -> Result<i64> {
    self.w.delete_all()
  }

  pub fn close(&self) -> Result<()> {
    self.w.close()
  }
  pub fn flush(&self) -> Result<()> {
    self.w.flush()
  }
  pub fn commit(&self) -> Result<i64> {
    self.w.commit()
  }
  pub fn force_merge(&self, max_num_segments: i32) -> Result<()> {
    self.w.force_merge(max_num_segments)
  }
  pub fn update_numeric_doc_value<T, F>(&self, term: T, field: F, value: i64) -> Result<i64>
  where
    T: Into<Arc<Term>>,
    F: Into<String>,
  {
    self.w.update_numeric_doc_value(term, field, value)
  }
  pub fn update_binary_doc_value<T, F>(
    &self,
    term: T,
    field: F,
    value: BytesRef<Vec<u8>>,
  ) -> Result<i64>
  where
    T: Into<Arc<Term>>,
    F: Into<String>,
  {
    self.w.update_binary_doc_value(term, field, value)
  }
  pub fn get_doc_stats(&self) -> Result<DocStats> {
    self.w.get_doc_stats()
  }

  pub fn set_do_random_force_merge(&mut self, _v: bool) {}
  pub fn update_document_with_term<T, DF>(&self, del_term: T, docs: DF) -> Result<i64>
  where
    T: Into<Option<Term>>,
    DF: IntoIterator<Item = Fields>,
  {
    self.w.update_document_with_term(del_term, docs)
  }
  pub fn update_documents_with_term<T, DI, DF>(&self, del_term: T, docs: DI) -> Result<i64>
  where
    T: Into<Option<Term>>,
    DI: IntoIterator<Item = DF>,
    DF: IntoIterator<Item = Fields>,
  {
    self.w.update_documents_with_term(del_term, docs)
  }
  pub fn add_indexes_from_dir(&self, dirs: &[Arc<D>]) -> Result<i64> {
    self.w.add_indexes_from_dir(dirs)
  }
}

static TEST_POINT_COMPONENT: &str = "TP";

struct TestPointInfoStream {
  delegate: InfoStreamMT,
  test_point: Arc<dyn TestPoint>,
}

impl TestPointInfoStream {
  fn new<TP>(delegate: InfoStreamMT, test_point: TP) -> Self
  where
    TP: TestPoint + 'static,
  {
    let delegate = if matches!(delegate.as_ref(), InfoStreamEnum::NoOutput(_)) {
      Arc::new(NullInfoStream.into())
    } else {
      delegate
    };
    Self {
      delegate,
      test_point: Arc::new(test_point),
    }
  }
}

impl InfoStream for TestPointInfoStream {
  fn close(&self) -> Result<()> {
    self.delegate.close()
  }

  fn message(&self, component: &str, message: &str) -> Result<()> {
    if component == TEST_POINT_COMPONENT {
      self.test_point.apply(message)?;
    }
    if self.delegate.is_enabled(component) {
      self.delegate.message(component, message)?;
    }
    Ok(())
  }

  fn is_enabled(&self, component: &str) -> bool {
    component == TEST_POINT_COMPONENT || self.delegate.is_enabled(component)
  }
}

struct TestPointsIndexWriterHooks;

impl IndexWriterHooks for TestPointsIndexWriterHooks {
  fn is_enable_test_points(&self) -> bool {
    true
  }
}

struct YieldTestPoint {
  random: Mutex<StdRng>,
}

impl YieldTestPoint {
  fn new(random: StdRng) -> Self {
    Self {
      random: Mutex::new(random),
    }
  }
}

impl TestPoint for YieldTestPoint {
  fn apply(&self, _message: &str) -> Result<()> {
    if self.random.lock().random_range(0..4) == 2 {
      thread::yield_now();
    }
    Ok(())
  }
}

/// Simple trait that is executed for each `TP` [`InfoStream`] component message.
/// See also [`RandomIndexWriter::mock_index_writer_with_test_point`].
pub trait TestPoint: Send + Sync {
  fn apply(&self, message: &str) -> Result<()>;
}

impl<T> TestPoint for Arc<T>
where
  T: TestPoint + ?Sized,
{
  fn apply(&self, message: &str) -> Result<()> {
    self.as_ref().apply(message)
  }
}
