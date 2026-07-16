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
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::threaded_indexing_and_searching_test_case::{
  ThreadedIndexSearcher, ThreadedIndexingAndSearchingTestCase,
  ThreadedIndexingAndSearchingTestCaseState,
};
use crate::test_framework::core::store::mock_directory_wrapper::MockDirectoryWrapper;
use crate::test_framework::core::util::lucene_test_case::random;
use parking_lot::RwLock;
use rand::RngExt;
use rand::prelude::StdRng;
use std::sync::Arc;
use std::sync::atomic::Ordering;

// TODO
//   - mix in forceMerge, addIndexes
//   - randomly mix in non-congruent docs

type TestDirectory = MockDirectoryWrapper<DirEnum>;
type TestDirectoryReader = StandardDirectoryReader<TestDirectory>;
type TestIndexSearcher = ThreadedIndexSearcher<TestDirectoryReader>;

#[allow(dead_code)] // for quick search
struct TestNRTThreads {
  state: ThreadedIndexingAndSearchingTestCaseState<TestDirectory>,
  use_non_nrt_readers: bool,
  fixed_searcher: RwLock<Option<Arc<TestIndexSearcher>>>,
}

impl TestNRTThreads {
  fn new(use_non_nrt_readers: bool) -> Self {
    Self {
      state: ThreadedIndexingAndSearchingTestCaseState::new(),
      use_non_nrt_readers,
      fixed_searcher: RwLock::new(None),
    }
  }
}

impl ThreadedIndexingAndSearchingTestCase for TestNRTThreads {
  type Directory = TestDirectory;
  type Reader = TestDirectoryReader;

  fn state(&self) -> &ThreadedIndexingAndSearchingTestCaseState<Self::Directory> {
    &self.state
  }

  fn do_searching(&self, random: &mut StdRng, max_iterations: i32) -> Result<()> {
    let mut any_open_del_files = false;

    let writer = self.state.writer();
    let mut reader = Arc::new(directory_reader::open_from_writer(&writer)?);
    let mut iterations = 0;
    while {
      iterations += 1;
      iterations < max_iterations && !self.state.failed.load(Ordering::SeqCst)
    } {
      if random.random_bool(0.5) {
        if let Some(new_reader) = directory_reader::open_if_changed(reader.as_ref())? {
          *self.fixed_searcher.write() = None;
          reader.close()?;
          reader = Arc::new(new_reader);
        }
      } else {
        *self.fixed_searcher.write() = None;
        reader.close()?;
        writer.commit()?;
        let open_deleted_files = self.state.directory().get_open_deleted_files();
        if !open_deleted_files.is_empty() {
          eprintln!("OBD files: {open_deleted_files:?}");
        }
        any_open_del_files |= !open_deleted_files.is_empty();
        // assertEquals("open but deleted: " + openDeletedFiles, 0, openDeletedFiles.size());
        reader = Arc::new(directory_reader::open_from_writer(&writer)?);
      }

      // System.out.println("numDocs=" + r.numDocs() + "
      // openDelFileCount=" + dir.openDeleteFileCount());
      if reader.num_docs()? > 0 {
        let searcher = Arc::new(IndexSearcher::new(reader.clone().get_context()?)?);
        *self.fixed_searcher.write() = Some(searcher.clone());
        self.smoke_test_searcher(searcher.as_ref())?;
        self.run_search_threads(random, 100)?;
      }
    }
    *self.fixed_searcher.write() = None;
    reader.close()?;

    // System.out.println("numDocs=" + r.numDocs() + " openDelFileCount=" +
    // dir.openDeleteFileCount());
    let open_deleted_files = self.state.directory().get_open_deleted_files();
    if !open_deleted_files.is_empty() {
      eprintln!("OBD files: {open_deleted_files:?}");
    }
    any_open_del_files |= !open_deleted_files.is_empty();

    assert!(!any_open_del_files, "saw non-zero open-but-deleted count");
    Ok(())
  }

  fn get_directory(&self, directory: Arc<Self::Directory>) -> Arc<Self::Directory> {
    if !self.use_non_nrt_readers {
      directory.set_assert_no_delete_open_file(true);
    }
    directory
  }

  fn do_after_writer(&self, _random: &mut StdRng, _search_threads: Option<usize>) -> Result<()> {
    // Force writer to do reader pooling, always, so that
    // all merged segments, even for merges before
    // doSearching is called, are warmed:
    directory_reader::open_from_writer(&self.state.writer())?.close()
  }

  fn get_current_searcher(
    &self,
    _random: &mut StdRng,
  ) -> Result<Arc<ThreadedIndexSearcher<Self::Reader>>> {
    Ok(
      self
        .fixed_searcher
        .read()
        .as_ref()
        .expect("fixed searcher has not been initialized")
        .clone(),
    )
  }

  fn release_searcher(&self, searcher: Arc<ThreadedIndexSearcher<Self::Reader>>) -> Result<()> {
    if self
      .fixed_searcher
      .read()
      .as_ref()
      .is_some_and(|fixed_searcher| Arc::ptr_eq(fixed_searcher, &searcher))
    {
      Ok(())
    } else {
      searcher.get_index_reader().close()
    }
  }

  fn get_final_searcher(
    &self,
    random: &mut StdRng,
  ) -> Result<Arc<ThreadedIndexSearcher<Self::Reader>>> {
    let reader = if self.use_non_nrt_readers {
      if random.random_bool(0.5) {
        directory_reader::open_from_writer(&self.state.writer())?
      } else {
        self.state.writer().commit()?;
        directory_reader::open(self.state.directory())?
      }
    } else {
      directory_reader::open_from_writer(&self.state.writer())?
    };
    Ok(Arc::new(IndexSearcher::new(
      Arc::new(reader).get_context()?,
    )?))
  }
}

#[test]
fn test_nrt_threads() -> Result<()> {
  let mut random = random();
  let test = TestNRTThreads::new(random.random_bool(0.5));
  test.run_test(&mut random, "TestNRTThreads")
}
