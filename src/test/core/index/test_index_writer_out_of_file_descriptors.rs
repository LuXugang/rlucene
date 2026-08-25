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
use crate::core::index::CODEC_FILE_PATTERN;
use crate::core::index::directory_reader;
use crate::core::index::index_file_names::IndexFileNames;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, MAX_TERM_LENGTH};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::{Directory, MockDirWrapper};
use crate::core::store::io_context::IOContext;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::InfoStreamEnum;
use crate::core::util::print_stream_info_stream::PrintStreamInfoStream;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::line_file_docs::LineFileDocs;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, create_temp_dir_with_prefix, new_index_writer_config_with_analyzer,
  new_mock_fs_directory, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestIndexWriterOutOfFileDescriptors;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(new_mock_fs_directory(
    &mut random,
    create_temp_dir_with_prefix("TestIndexWriterOutOfFileDescriptors")?,
  )?);
  let rate = random.random::<f64>() * 0.01;
  // println!("rate={rate}");
  dir.set_random_io_exception_rate_on_open(rate);
  let iters = at_least(&mut random, 20);
  let mut docs = LineFileDocs::new(&mut random)?;
  let mut reader = None;
  let mut reader2 = None;
  let mut any = false;
  let mut dir_copy: Option<Arc<MockDirWrapper>> = None;
  let mut last_num_docs = 0;
  for iter in 0..iters {
    let mut writer = None;
    if cfg!(feature = "test_log_verbose") {
      println!("TEST: iter={iter}");
    }

    let iteration_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      let mut analyzer = MockAnalyzer::new(&mut random);
      analyzer.set_max_token_length(TestUtil::next_int(&mut random, 1, MAX_TERM_LENGTH));
      let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer)?;

      if cfg!(feature = "test_log_verbose") {
        // Do this ourselves instead of relying on LTC so
        // we see incrementing messageID:
        iwc.set_info_stream(Arc::new(InfoStreamEnum::from(
          PrintStreamInfoStream::stdout(),
        )));
      }
      let merge_scheduler = match iwc.get_merge_scheduler() {
        MergeSchedulerEnum::Concurrent(ms) => {
          ms.set_suppress_exceptions();
          Some(ms.clone())
        },
        _ => None,
      };
      let w = IndexWriter::new(dir.clone(), iwc)?;
      writer = Some(w.clone());
      if let Some(reader) = reader.as_ref()
        && random.random_range(0..5) == 3
      {
        if random.random_bool(0.5) {
          if cfg!(feature = "test_log_verbose") {
            println!("TEST: addIndexes LR[]");
          }
          TestUtil::add_indexes_slowly(&w, &[reader])?;
        } else {
          if cfg!(feature = "test_log_verbose") {
            println!("TEST: addIndexes Directory[]");
          }
          if let Some(dir_copy) = dir_copy.as_ref() {
            w.add_indexes_from_directory(std::slice::from_ref(dir_copy))?;
          } else {
            return Err(LuceneError::illegal_state(
              "index copy is missing while its reader is open",
            ));
          }
        }
      } else {
        if cfg!(feature = "test_log_verbose") {
          println!("TEST: addDocument");
        }
        w.add_document(docs.next_doc()?)?;
      }
      dir.set_random_io_exception_rate_on_open(0.0);
      if let Some(merge_scheduler) = merge_scheduler {
        merge_scheduler.sync()?;
      }
      // If exc hit CMS then writer will be tragically closed:
      if w.get_tragic_exception().get().is_none() {
        w.close()?;
      }
      writer = None;

      // NOTE: This is O(N^2)!  Only enable for temporary debugging:
      // dir.set_random_io_exception_rate_on_open(0.0);
      // TestUtil::check_index(dir.as_ref())?;
      // dir.set_random_io_exception_rate_on_open(rate);

      // Verify numDocs only increases, to catch IndexWriter
      // accidentally deleting the index:
      dir.set_random_io_exception_rate_on_open(0.0);
      assert!(
        directory_reader::index_exists(dir.as_ref())?,
        "index does not exist"
      );
      if reader2.is_none() {
        reader2 = Some(directory_reader::open(dir.clone())?);
      } else if let Some(current_reader) = reader2.as_ref()
        && let Some(new_reader) = directory_reader::open_if_changed(current_reader)?
      {
        current_reader.close()?;
        reader2 = Some(new_reader);
      }
      if let Some(reader2) = reader2.as_ref() {
        let num_docs = reader2.num_docs()?;
        assert!(
          num_docs >= last_num_docs,
          "before={last_num_docs} after={num_docs}"
        );
        last_num_docs = num_docs;
      }
      // println!("numDocs={last_num_docs}");
      dir.set_random_io_exception_rate_on_open(rate);

      any = true;
      if cfg!(feature = "test_log_verbose") {
        println!("TEST: iter={iter}: success");
      }
      Ok(())
    }));

    let iteration_result = match iteration_result {
      Ok(result) => result,
      Err(payload) => {
        let message = LuceneError::panic_payload_message(payload.as_ref());
        if message == "index does not exist" || message.starts_with("before=") {
          Err(LuceneError::illegal_state(message))
        } else {
          resume_unwind(payload)
        }
      },
    };

    if let Err(error) = iteration_result {
      if error.is_io_error()
        || matches!(
          error,
          LuceneError::IllegalState(_) | LuceneError::AlreadyClosed(_)
        )
      {
        if cfg!(feature = "test_log_verbose") {
          println!("TEST: iter={iter}: error");
          eprintln!("{error:?}");
        }
        if let Some(w) = writer {
          // NOTE: leave random IO errors enabled here,
          // to verify that rollback does not try to write
          // anything:
          w.rollback()?;
        }
      } else {
        return Err(error);
      }
    }

    if any && reader.is_none() && random.random_bool(0.5) {
      // Make a copy of a non-empty index so we can use
      // it to addIndexes later:
      dir.set_random_io_exception_rate_on_open(0.0);
      reader = Some(directory_reader::open(dir.clone())?);
      let copy = Arc::new(new_mock_fs_directory(
        &mut random,
        create_temp_dir_with_prefix("TestIndexWriterOutOfFileDescriptors.copy")?,
      )?);
      let mut files = HashSet::new();
      let io_context = IOContext::default_io_context()?;
      for file in dir.list_all()? {
        if file.starts_with(IndexFileNames::SEGMENTS) || CODEC_FILE_PATTERN.is_match(&file) {
          copy.copy_from(dir.as_ref(), &file, &file, &io_context)?;
          files.insert(file);
        }
      }
      let files_to_sync = files.into_iter().collect::<Vec<_>>();
      copy.sync(&files_to_sync)?;
      // Have IW kiss the dir so we remove any leftover
      // files ... we can easily have leftover files at
      // the time we take a copy because we are holding
      // open a reader:
      let analyzer = MockAnalyzer::new(&mut random);
      let copy_writer = IndexWriter::new(
        copy.clone(),
        new_index_writer_config_with_analyzer(&mut random, analyzer)?,
      )?;
      copy_writer.close()?;
      copy.set_random_io_exception_rate(rate);
      dir_copy = Some(copy);
      dir.set_random_io_exception_rate_on_open(rate);
    }
  }

  if let Some(reader2) = reader2 {
    reader2.close()?;
  }
  if let Some(reader) = reader {
    reader.close()?;
    if let Some(dir_copy) = dir_copy {
      dir_copy.as_ref().close()?;
    } else {
      return Err(LuceneError::illegal_state(
        "index copy is missing while its reader is open",
      ));
    }
  }
  dir.as_ref().close()
}
