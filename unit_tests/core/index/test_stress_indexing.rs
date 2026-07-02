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
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::english::English;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, random as new_random,
};
use rand::RngExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
#[allow(dead_code)] // for quick search
struct TestStressIndexing;

struct TimedThread {
  failed: AtomicBool,
}

impl TimedThread {
  fn new() -> Self {
    Self {
      failed: AtomicBool::new(false),
    }
  }
}

/*
  Run one indexer and 2 searchers against single index as
  stress test.
*/
fn run_stress_test(directory: Arc<DirEnum>, merge_scheduler: MergeSchedulerEnum) -> Result<()> {
  let mut random = new_random();
  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  config.set_open_mode(OpenMode::Create);
  config.set_max_buffered_docs(10);
  config.set_merge_scheduler(merge_scheduler);
  let modifier = IndexWriter::new(directory.clone(), config)?;
  modifier.commit()?;

  let run_iterations = if is_night_mode() {
    at_least(&mut random, 100)
  } else {
    at_least(&mut random, 20)
  };
  let threads = Arc::new([
    TimedThread::new(),
    TimedThread::new(),
    TimedThread::new(),
    TimedThread::new(),
  ]);

  let thread_results = thread::scope(|scope| {
    let mut handles = Vec::new();

    for thread_id in 0..2 {
      let modifier = &modifier;
      let all_threads = threads.clone();
      handles.push(scope.spawn(move || {
        let mut next_id = 0;
        let mut random = new_random();
        let thread_state = &all_threads[thread_id];
        let mut iterations = 0;
        let result = (|| -> Result<()> {
          loop {
            if all_threads
              .iter()
              .any(|thread| thread.failed.load(Ordering::Acquire))
            {
              break;
            }

            // Add 10 docs:
            for _ in 0..10 {
              let mut d = Document::new();
              let n = random.random::<i32>();
              d.add(StringField::from_string(
                "id",
                next_id.to_string(),
                Store::Yes,
              )?);
              next_id += 1;
              d.add(TextField::from_string(
                "contents",
                English::int_to_english(n),
                Store::No,
              )?);
              modifier.add_document(d)?;
            }

            // Delete 5 docs:
            let mut delete_id = next_id - 1;
            for _ in 0..5 {
              modifier
                .delete_documents_with_terms(vec![Term::from_text("id", delete_id.to_string())])?;
              delete_id -= 2;
            }

            iterations += 1;
            if iterations >= run_iterations {
              break;
            }
          }
          Ok(())
        })();
        if result.is_err() {
          thread_state.failed.store(true, Ordering::Release);
        }
        result
      }));
    }

    for thread_id in 2..4 {
      let directory = directory.clone();
      let all_threads = threads.clone();
      handles.push(scope.spawn(move || {
        let thread_state = &all_threads[thread_id];
        let mut iterations = 0;
        let result = (|| -> Result<()> {
          loop {
            if all_threads
              .iter()
              .any(|thread| thread.failed.load(Ordering::Acquire))
            {
              break;
            }

            for _ in 0..100 {
              let ir = directory_reader::open(directory.clone())?;
              let _searcher = new_searcher_with_reader(ir)?;
            }

            iterations += 1;
            if iterations >= run_iterations {
              break;
            }
          }
          Ok(())
        })();
        if result.is_err() {
          thread_state.failed.store(true, Ordering::Release);
        }
        result
      }));
    }

    let mut results = Vec::new();
    for handle in handles {
      results.push(handle.join());
    }
    results
  });

  for thread_result in thread_results {
    match thread_result {
      Ok(Ok(())) => {},
      Ok(Err(err)) => return Err(err),
      Err(_) => return Err(LuceneError::illegal_state("thread hit exception")),
    }
  }

  modifier.close()?;

  for thread_state in threads.iter() {
    assert!(!thread_state.failed.load(Ordering::Acquire));
  }

  Ok(())
}

#[test]
fn test_stress_index_and_searching() -> Result<()> {
  let mut random = new_random();
  let directory = new_directory_shared(&mut random)?;

  run_stress_test(
    directory,
    MergeSchedulerEnum::from(SerialMergeScheduler::new()),
  )
}
