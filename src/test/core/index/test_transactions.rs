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
use crate::core::document::field_type::FieldType;
use crate::core::document::string_field::TYPE_NOT_STORED;
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::ByteBuffersDirectory;
use crate::core::store::directory::Directory;
use crate::core::store::single_instance_lock_factory::SingleInstanceLockFactory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::store::mock_directory_wrapper::{Failure, MockDirectoryWrapper};
use crate::test_framework::core::util::english::English;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_field, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, new_text_field, random,
};
use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::collections::HashMap;
use std::io::Error;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestTransactions;

static DO_FAIL: AtomicBool = AtomicBool::new(false);

struct RandomFailure {
  random: StdRng,
}

impl<D> Failure<D> for RandomFailure
where
  D: Directory,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if DO_FAIL.load(Ordering::SeqCst) && self.random.random_range(0..10) <= 3 {
      if cfg!(feature = "test_log_verbose") {
        println!(
          "{} TEST: now fail on purpose",
          thread::current().name().unwrap_or("unnamed")
        );
      }
      return Err(LuceneError::io(Error::other(
        "now failing randomly but on purpose",
      )));
    }
    Ok(())
  }

  fn set_do_fail(&mut self) {
    DO_FAIL.store(true, Ordering::SeqCst);
  }

  fn clear_do_fail(&mut self) {
    DO_FAIL.store(false, Ordering::SeqCst);
  }
}

#[derive(Clone)]
struct TimedThread {
  failed: Arc<AtomicBool>,
  max_iterations: i32,
  all_threads: Arc<Vec<Arc<AtomicBool>>>,
}

impl TimedThread {
  fn new(
    failed: Arc<AtomicBool>,
    max_iterations: i32,
    all_threads: Arc<Vec<Arc<AtomicBool>>>,
  ) -> Self {
    Self {
      failed,
      max_iterations,
      all_threads,
    }
  }

  fn run<F>(&self, mut do_work: F)
  where
    F: FnMut() -> Result<()>,
  {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      let mut iterations = 0;
      loop {
        iterations += 1;
        println!("{iterations}");
        if self.any_errors() {
          break;
        }
        do_work()?;
        if iterations >= self.max_iterations {
          break;
        }
      }
      Ok(())
    }));

    match result {
      Ok(Ok(())) => {},
      Ok(Err(error)) => {
        eprintln!("{:?}: exc", thread::current());
        eprintln!("{error:?}");
        self.failed.store(true, Ordering::SeqCst);
      },
      Err(_) => {
        eprintln!("{:?}: exc", thread::current());
        self.failed.store(true, Ordering::SeqCst);
      },
    }
  }

  fn any_errors(&self) -> bool {
    self
      .all_threads
      .iter()
      .any(|failed| failed.load(Ordering::SeqCst))
  }
}

type TestDirectory = MockDirectoryWrapper<ByteBuffersDirectory<SingleInstanceLockFactory>>;

struct IndexerThread {
  timed_thread: TimedThread,
  dir1: Arc<TestDirectory>,
  dir2: Arc<TestDirectory>,
  lock: Arc<Mutex<()>>,
  next_id: i32,
  random: StdRng,
  field_to_type: HashMap<String, FieldType>,
}

impl IndexerThread {
  fn run(mut self) {
    let timed_thread = self.timed_thread.clone();
    timed_thread.run(|| self.do_work());
  }

  fn do_work(&mut self) -> Result<()> {
    let analyzer1 = MockAnalyzer::new(&mut self.random);
    let mut config1: IndexWriterConfig<TestDirectory> =
      new_index_writer_config_with_analyzer(&mut self.random, analyzer1)?;
    config1.set_max_buffered_docs(3);
    let merge_scheduler1 = ConcurrentMergeScheduler::new();
    merge_scheduler1.set_suppress_exceptions();
    config1.set_merge_scheduler(merge_scheduler1);
    config1.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut self.random, 2)?);
    let writer1 = IndexWriter::new(self.dir1.clone(), config1)?;

    // Intentionally use different params so flush/merge
    // happen @ different times
    let analyzer2 = MockAnalyzer::new(&mut self.random);
    let mut config2: IndexWriterConfig<TestDirectory> =
      new_index_writer_config_with_analyzer(&mut self.random, analyzer2)?;
    config2.set_max_buffered_docs(2);
    let merge_scheduler2 = ConcurrentMergeScheduler::new();
    merge_scheduler2.set_suppress_exceptions();
    config2.set_merge_scheduler(merge_scheduler2);
    config2.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut self.random, 3)?);
    let writer2 = IndexWriter::new(self.dir2.clone(), config2)?;

    self.update(writer1.as_ref())?;
    self.update(writer2.as_ref())?;

    DO_FAIL.store(true, Ordering::SeqCst);
    let commit_result = catch_unwind(AssertUnwindSafe(|| -> Result<bool> {
      let _guard = self.lock.lock();
      let prepare1 = catch_unwind(AssertUnwindSafe(|| writer1.prepare_commit()));
      if !matches!(prepare1, Ok(Ok(_))) {
        // release resources
        let _ = catch_unwind(AssertUnwindSafe(|| writer1.rollback()));
        let _ = catch_unwind(AssertUnwindSafe(|| writer2.rollback()));
        return Ok(false);
      }

      let prepare2 = catch_unwind(AssertUnwindSafe(|| writer2.prepare_commit()));
      if !matches!(prepare2, Ok(Ok(_))) {
        // release resources
        let _ = catch_unwind(AssertUnwindSafe(|| writer1.rollback()));
        let _ = catch_unwind(AssertUnwindSafe(|| writer2.rollback()));
        return Ok(false);
      }

      writer1.commit()?;
      writer2.commit()?;
      Ok(true)
    }));
    DO_FAIL.store(false, Ordering::SeqCst);

    let committed = match commit_result {
      Ok(result) => result?,
      Err(payload) => std::panic::resume_unwind(payload),
    };
    if !committed {
      return Ok(());
    }

    writer1.close()?;
    writer2.close()?;
    Ok(())
  }

  fn update(&mut self, writer: &IndexWriter<TestDirectory>) -> Result<()> {
    // Add 10 docs:
    let mut custom_type = FieldType::from_ref(&*TYPE_NOT_STORED)?;
    custom_type.set_store_term_vectors(true)?;
    for _ in 0..10 {
      let mut d = Document::new();
      let n = self.random.random::<i32>();
      d.add(new_field(
        &mut self.random,
        "id",
        self.next_id.to_string(),
        &custom_type,
        &mut self.field_to_type,
      )?);
      self.next_id += 1;
      d.add(new_text_field(
        &mut self.random,
        "contents",
        English::int_to_english(n),
        Store::No,
        &mut self.field_to_type,
      )?);
      writer.add_document(d)?;
    }

    // Delete 5 docs:
    let mut delete_id = self.next_id - 1;
    for _ in 0..5 {
      writer.delete_documents_with_terms(vec![Term::from_text("id", delete_id.to_string())])?;
      delete_id -= 2;
    }
    Ok(())
  }
}

struct SearcherThread {
  timed_thread: TimedThread,
  dir1: Arc<TestDirectory>,
  dir2: Arc<TestDirectory>,
  lock: Arc<Mutex<()>>,
}

impl SearcherThread {
  fn run(mut self) {
    let timed_thread = self.timed_thread.clone();
    timed_thread.run(|| self.do_work());
  }

  fn do_work(&mut self) -> Result<()> {
    let mut r1 = None;
    let mut r2 = None;
    {
      let _guard = self.lock.lock();
      let open_result = (|| -> Result<()> {
        r1 = Some(directory_reader::open(self.dir1.clone())?);
        r2 = Some(directory_reader::open(self.dir2.clone())?);
        Ok(())
      })();
      if let Err(error) = open_result {
        // can be rethrown as RuntimeException if it happens during a close listener
        if !error.to_string().contains("on purpose") {
          return Err(error);
        }
        // release resources
        IOUtils::close_while_handling_exception_with(
          [r1.as_ref(), r2.as_ref()].into_iter().flatten(),
          IndexReader::close,
        )?;
        return Ok(());
      }
    }

    let r1 = r1.unwrap();
    let r2 = r2.unwrap();
    if r1.num_docs()? != r2.num_docs()? {
      return Err(LuceneError::illegal_state(format!(
        "doc counts differ: r1={} r2={}",
        r1.num_docs()?,
        r2.num_docs()?
      )));
    }
    IOUtils::close_while_handling_exception_with([&r1, &r2], IndexReader::close)?;
    Ok(())
  }
}

fn init_index(
  random: &mut StdRng,
  dir: Arc<TestDirectory>,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()> {
  let analyzer = MockAnalyzer::new(random);
  let config = new_index_writer_config_with_analyzer(random, analyzer)?;
  let writer = IndexWriter::new(dir, config)?;
  for _ in 0..7 {
    let mut d = Document::new();
    let n = random.random::<i32>();
    d.add(new_text_field(
      random,
      "contents",
      English::int_to_english(n),
      Store::No,
      field_to_type,
    )?);
    writer.add_document(d)?;
  }
  writer.close()
}

#[test]
fn test_transactions() -> Result<()> {
  let mut random = random();

  // we cant use non-ramdir on windows, because this test needs to double-write.
  let dir1 = Arc::new(MockDirectoryWrapper::new(
    &mut random,
    ByteBuffersDirectory::new(),
  ));
  let dir2 = Arc::new(MockDirectoryWrapper::new(
    &mut random,
    ByteBuffersDirectory::new(),
  ));
  dir1.fail_on(Box::new(RandomFailure {
    random: StdRng::seed_from_u64(random.random()),
  }));
  dir2.fail_on(Box::new(RandomFailure {
    random: StdRng::seed_from_u64(random.random()),
  }));
  dir1.set_fail_on_open_input(false);
  dir2.set_fail_on_open_input(false);

  // We throw exceptions in deleteFile, which creates
  // leftover files:
  dir1.set_assert_no_unrefenced_files_on_close(false);
  dir2.set_assert_no_unrefenced_files_on_close(false);

  let mut field_to_type = HashMap::new();
  init_index(&mut random, dir1.clone(), &mut field_to_type)?;
  init_index(&mut random, dir2.clone(), &mut field_to_type)?;

  let max_iterations = at_least(&mut random, 100);
  let failed = Arc::new(vec![
    Arc::new(AtomicBool::new(false)),
    Arc::new(AtomicBool::new(false)),
    Arc::new(AtomicBool::new(false)),
  ]);
  let lock = Arc::new(Mutex::new(()));

  let indexer_thread = IndexerThread {
    timed_thread: TimedThread::new(failed[0].clone(), max_iterations, failed.clone()),
    dir1: dir1.clone(),
    dir2: dir2.clone(),
    lock: lock.clone(),
    next_id: 0,
    random: StdRng::seed_from_u64(random.random()),
    field_to_type,
  };
  let indexer_handle = thread::Builder::new()
    .name("indexer".to_string())
    .spawn(move || indexer_thread.run())
    .map_err(LuceneError::io)?;

  let searcher_thread1 = SearcherThread {
    timed_thread: TimedThread::new(failed[1].clone(), max_iterations, failed.clone()),
    dir1: dir1.clone(),
    dir2: dir2.clone(),
    lock: lock.clone(),
  };
  let searcher_handle1 = thread::Builder::new()
    .name("searcher-1".to_string())
    .spawn(move || searcher_thread1.run())
    .map_err(LuceneError::io)?;

  let searcher_thread2 = SearcherThread {
    timed_thread: TimedThread::new(failed[2].clone(), max_iterations, failed.clone()),
    dir1: dir1.clone(),
    dir2: dir2.clone(),
    lock,
  };
  let searcher_handle2 = thread::Builder::new()
    .name("searcher-2".to_string())
    .spawn(move || searcher_thread2.run())
    .map_err(LuceneError::io)?;

  indexer_handle.join().unwrap();
  searcher_handle1.join().unwrap();
  searcher_handle2.join().unwrap();

  for thread_failed in failed.iter() {
    assert!(!thread_failed.load(Ordering::SeqCst));
  }
  dir1.close()?;
  dir2.close()?;
  Ok(())
}
