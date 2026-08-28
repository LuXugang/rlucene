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
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::reference_manager::RefreshListener;
use crate::core::search::searcher_factory::{SearcherFactory, SearcherFactoryHook};
use crate::core::search::searcher_manager::SearcherManager;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::test_searcher_manager::EvilSearcherFactory;
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use rand::{Rng, RngExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(dead_code)] // for quick search
struct TestControlledRealTimeReopenThread;

struct AfterRefreshCalled {
  called: Arc<AtomicBool>,
}

impl RefreshListener for AfterRefreshCalled {
  fn before_refresh(&self) -> Result<()> {
    Ok(())
  }

  fn after_refresh(&self, did_refresh: bool) -> Result<()> {
    if did_refresh {
      self.called.store(true, Ordering::SeqCst);
    }
    Ok(())
  }
}

#[test]
fn test_evil_searcher_factory() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, directory.clone())?;
  writer.commit(&mut random)?;

  let other = Arc::new(directory_reader::open(directory.clone())?);

  let result = SearcherManager::with_writer_deletes(
    &writer.w,
    false,
    false,
    Some(SearcherFactory::with_hook(SearcherFactoryHook::Evil(
      EvilSearcherFactory::new(other.clone(), random.random()),
    ))),
  );
  assert!(matches!(
    result,
    Err(error) if error.is_illegal_state_error()
  ));

  writer.close(&mut random)?;
  other.close()?;
  directory.close()?;
  Ok(())
}

#[test]
fn test_listener_called() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(directory.clone(), IndexWriterConfig::new()?)?;
  let after_refresh_called = Arc::new(AtomicBool::new(false));
  let sm =
    SearcherManager::with_writer_deletes(&writer, false, false, Some(SearcherFactory::new()))?;
  sm.add_listener(Arc::new(AfterRefreshCalled {
    called: after_refresh_called.clone(),
  }));
  writer.add_document(Document::new())?;
  writer.commit()?;
  assert!(!after_refresh_called.load(Ordering::SeqCst));
  sm.maybe_refresh_blocking()?;
  assert!(after_refresh_called.load(Ordering::SeqCst));
  sm.close()?;
  writer.close()?;
  directory.close()?;
  Ok(())
}
