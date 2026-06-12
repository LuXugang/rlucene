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
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, MAX_TERM_LENGTH};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::line_file_docs::LineFileDocs;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, random,
  random_from_seed, rarely,
};
use crate::test::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::Rng;
use rand::RngExt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
#[allow(dead_code)] // for quick search
struct TestIndexWriterWithThreads;

const SOFT_DELETES_FIELD: &str = "___soft_deletes";

#[test]
fn test_immediate_disk_full_with_threads() -> Result<()> {
  // TODO MockDirectoryWrapper未实现
  Ok(())
}

#[test]
fn test_close_with_threads() -> Result<()> {
  // TODO IndexerThread未实现
  Ok(())
}

#[test]
fn test_io_exception_during_abort() -> Result<()> {
  // TODO FailOnlyOnAbortOrFlush
  Ok(())
}

#[test]
fn test_io_exception_during_abort_only_once() -> Result<()> {
  // TODO FailOnlyOnAbortOrFlush
  Ok(())
}

#[test]
fn test_io_exception_during_abort_with_threads() -> Result<()> {
  // TODO FailOnlyOnAbortOrFlush
  Ok(())
}

#[test]
fn test_io_exception_during_abort_with_threads_only_once() -> Result<()> {
  // TODO FailOnlyOnAbortOrFlush
  Ok(())
}

#[test]
fn test_io_exception_during_write_segment() -> Result<()> {
  // TODO FailOnlyInWriteSegment未实现
  Ok(())
}

#[test]
fn test_io_exception_during_write_segment_only_once() -> Result<()> {
  // TODO FailOnlyInWriteSegment未实现
  Ok(())
}

#[test]
fn test_io_exception_during_write_segment_with_threads() -> Result<()> {
  // TODO FailOnlyInWriteSegment未实现
  Ok(())
}

#[test]
fn test_io_exception_during_write_segment_with_threads_only_once() -> Result<()> {
  // TODO FailOnlyInWriteSegment未实现
  Ok(())
}

#[test]
fn test_open_two_index_writers_on_different_threads() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let sync_start = Arc::new(Barrier::new(2));

  let results = thread::scope(|scope| {
    let mut handles = Vec::new();
    for thread_id in 0..2 {
      let seed = random.random();
      let dir = dir.clone();
      let sync_start = sync_start.clone();
      handles.push(scope.spawn(move || -> Result<()> {
        let mut random = random_from_seed(seed);
        let mut doc = Document::new();
        doc.add(TextField::from_string("field", "testData", Store::Yes)?);

        sync_start.wait();
        if thread_id == 1 && random.random_bool(0.5) {
          thread::sleep(Duration::from_millis(100));
        }
        let analyzer = MockAnalyzer::new(&mut random);
        let writer = IndexWriter::new(
          dir,
          new_index_writer_config_with_analyzer(&mut random, analyzer),
        )?;
        writer.add_document(doc)?;
        writer.close()
      }));
    }

    handles
      .into_iter()
      .map(|handle| handle.join().expect("thread panicked"))
      .collect::<Vec<_>>()
  });

  if results
    .iter()
    .any(|result| matches!(result, Err(LuceneError::LockObtainFailed(_))))
  {
    return Ok(());
  }

  for result in results {
    result?;
  }

  let reader = directory_reader::open(dir)?;
  assert_eq!(2, reader.num_docs()?);
  reader.close()?;
  Ok(())
}

#[test]
fn test_rollback_and_commit_with_threads() -> Result<()> {
  let mut rng = random();
  let dir = new_directory_shared(&mut rng)?;
  let thread_count = TestUtil::next_int(&mut rng, 2, 6) as usize;

  let mut analyzer = MockAnalyzer::new(&mut rng);
  analyzer.set_max_token_length(TestUtil::next_int(&mut rng, 1, MAX_TERM_LENGTH));
  let writer = Arc::new(IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut rng, analyzer),
  )?);
  writer.commit()?;

  let writer_ref = Arc::new(Mutex::new(writer));
  let failed = Arc::new(AtomicBool::new(false));
  let rollback_lock = Arc::new(Mutex::new(()));
  let commit_lock = Arc::new(Mutex::new(()));
  let docs = Arc::new(Mutex::new(LineFileDocs::new(&mut rng)?));
  let iters = at_least(&mut rng, 100);

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();
    for _ in 0..thread_count {
      let seed = rng.random();
      let dir = dir.clone();
      let writer_ref = writer_ref.clone();
      let failed = failed.clone();
      let rollback_lock = rollback_lock.clone();
      let commit_lock = commit_lock.clone();
      let docs = docs.clone();
      handles.push(scope.spawn(move || -> Result<()> {
        let mut random = random_from_seed(seed);

        for _ in 0..iters {
          if failed.load(Ordering::SeqCst) {
            break;
          }

          let result = match random.random_range(0..3) {
            0 => {
              let _rollback_guard = rollback_lock.lock();
              let writer = writer_ref.lock().clone();
              writer.rollback()?;

              let analyzer = MockAnalyzer::new(&mut random);
              let new_writer = Arc::new(IndexWriter::new(
                dir.clone(),
                new_index_writer_config_with_analyzer(&mut random, analyzer),
              )?);
              *writer_ref.lock() = new_writer;
              Ok(())
            },
            1 => {
              let _commit_guard = commit_lock.lock();
              let writer = writer_ref.lock().clone();
              let result = (|| -> Result<()> {
                if random.random_bool(0.5) {
                  writer.prepare_commit()?;
                }
                writer.commit()?;
                Ok(())
              })();
              match result {
                Ok(()) | Err(LuceneError::AlreadyClosed(_)) => Ok(()),
                Err(error) => Err(error),
              }
            },
            2 => {
              let writer = writer_ref.lock().clone();
              let doc = docs.lock().next_doc()?;
              match writer.add_document(doc) {
                Ok(_) | Err(LuceneError::AlreadyClosed(_)) => Ok(()),
                Err(error) => Err(error),
              }
            },
            _ => unreachable!(),
          };

          if let Err(error) = result {
            failed.store(true, Ordering::SeqCst);
            return Err(error);
          }
        }
        Ok(())
      }));
    }

    for handle in handles {
      handle.join().expect("thread panicked")?;
    }
    Ok(())
  })?;

  assert!(!failed.load(Ordering::SeqCst));
  writer_ref.lock().close()?;
  Ok(())
}

fn test_update_single_doc_with_threads() -> Result<()> {
  let mut random = random();
  let force_merge = rarely(&mut random);
  stress_update_single_doc_with_threads(&mut random, false, force_merge)
}
fn test_soft_update_single_doc_with_threads() -> Result<()> {
  let mut random = random();
  let force_merge = rarely(&mut random);
  stress_update_single_doc_with_threads(&mut random, true, force_merge)
}

fn stress_update_single_doc_with_threads<R>(
  random: &mut R,
  use_soft_deletes: bool,
  force_merge: bool,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let analyzer = MockAnalyzer::new(random);
  let mut config = new_index_writer_config_with_analyzer(random, analyzer);
  config
    .set_max_buffered_docs(-1)
    .set_ram_buffer_size_mb(0.00001);
  let writer = Arc::new(RandomIndexWriter::with_soft_deletes(
    random,
    dir,
    config,
    use_soft_deletes,
  ));
  let num_threads = if is_night_mode() {
    3 + random.random_range(0..3)
  } else {
    3
  };
  let done = Arc::new(AtomicUsize::new(0));
  let barrier = Arc::new(Barrier::new(num_threads + 1));

  let mut doc = Document::new();
  doc.add(StringField::from_string("id", "1", Store::No)?);
  writer.update_document_with_term(Term::from_text("id", "1"), doc)?;

  let iters_per_thread = 100 + random.random_range(0..2000);
  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();
    for _ in 0..num_threads {
      let writer = writer.clone();
      let done = done.clone();
      let barrier = barrier.clone();
      handles.push(scope.spawn(move || -> Result<()> {
        barrier.wait();
        let result = (|| -> Result<()> {
          for _ in 0..iters_per_thread {
            let mut d = Document::new();
            d.add(StringField::from_string("id", "1", Store::No)?);
            writer.update_document_with_term(Term::from_text("id", "1"), d)?;
          }
          Ok(())
        })();
        done.fetch_add(1, Ordering::SeqCst);
        result
      }));
    }

    let mut open = writer.get_reader()?;
    assert_eq!(1, open.num_docs()?);
    barrier.wait();
    while done.load(Ordering::SeqCst) < num_threads {
      if force_merge && random.random_bool(0.5) {
        writer.force_merge(1)?;
      }
      if let Some(new_open) = directory_reader::open_if_changed(&open, &writer.w)? {
        open.close()?;
        open = new_open;
      }
      assert_eq!(1, open.num_docs()?);
    }
    open.close()?;

    for handle in handles {
      handle.join().expect("thread panicked")?;
    }
    Ok(())
  })?;

  writer.close()?;
  Ok(())
}
