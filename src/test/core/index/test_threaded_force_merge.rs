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
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{DefaultIndexWriterType, IndexWriter};
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicyEnum;
use crate::core::index::term::Term;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer;
use crate::test::core::util::english::English;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, random,
};
use rand::Rng;
use std::sync::Arc;
use std::thread;

pub struct TestThreadedForceMerge {
  failed: bool,
}

const NUM_THREADS: i32 = 3;
const NUM_ITER: i32 = 1;
const NUM_ITER2: i32 = 1;

impl TestThreadedForceMerge {
  fn new() -> Self {
    Self { failed: false }
  }

  fn set_failed(&mut self) {
    self.failed = true;
  }

  fn set_merge_factor(
    writer: &mut DefaultIndexWriterType<DirEnum>,
    merge_factor: usize,
  ) -> Result<()> {
    match writer.get_config_mut().get_merge_policy_mut() {
      MergePolicyEnum::LogDoc(mp) => mp.set_merge_factor(merge_factor),
      MergePolicyEnum::LogBytesSize(mp) => mp.set_merge_factor(merge_factor),
      _ => Err(LuceneError::illegal_state(
        "expected LogMergePolicy variant",
      )),
    }
  }

  fn run_test<R>(&mut self, random: &mut R, directory: Arc<DirEnum>) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let analyzer = MockAnalyzer::with_automaton(random, mock_tokenizer::SIMPLE.clone(), true);
    let mut config = new_index_writer_config_with_analyzer(random, analyzer);
    config.set_open_mode(OpenMode::Create);
    config.set_max_buffered_docs(2);
    config.set_merge_policy(new_log_merge_policy_with_merge_factor(random, 1000)?);
    let mut writer = IndexWriter::new(directory.clone(), config)?;

    for iter in 0..NUM_ITER {
      Self::set_merge_factor(&mut writer, 1000)?;

      for i in 0..200 {
        let mut doc = Document::new();
        doc.add(StringField::from_string("id", i.to_string(), Store::Yes)?);
        doc.add(StringField::from_string(
          "contents",
          English::int_to_english(i),
          Store::Yes,
        )?);
        writer.add_document(doc)?;
      }

      Self::set_merge_factor(&mut writer, 4)?;

      let thread_results = thread::scope(|scope| {
        let mut handles = Vec::new();
        for i in 0..NUM_THREADS {
          let writer_ref = &writer;
          handles.push(scope.spawn(move || -> Result<()> {
            for j in 0..NUM_ITER2 {
              writer_ref.force_merge_with_wait(1, false)?;
              for k in 0..17 * (1 + i) {
                let mut doc = Document::new();
                doc.add(StringField::from_string(
                  "id",
                  format!("{iter}_{i}_{j}_{k}"),
                  Store::Yes,
                )?);
                doc.add(StringField::from_string(
                  "contents",
                  English::int_to_english(i + k),
                  Store::Yes,
                )?);
                writer_ref.add_document(doc)?;
              }
              for k in 0..9 * (1 + i) {
                writer_ref.delete_documents_with_terms(vec![Term::from_text(
                  "id",
                  format!("{iter}_{i}_{j}_{k}"),
                )])?;
              }
              writer_ref.force_merge(1)?;
            }
            Ok(())
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
          Ok(Err(err)) => {
            self.set_failed();
            return Err(err);
          },
          Err(_) => {
            self.set_failed();
            return Err(LuceneError::illegal_state("thread hit exception"));
          },
        }
      }

      assert!(!self.failed);

      let expected_doc_count = ((1.0 + f64::from(iter))
        * (200.0
          + 8.0
            * f64::from(NUM_ITER2)
            * (f64::from(NUM_THREADS) / 2.0)
            * (1.0 + f64::from(NUM_THREADS)))) as i32;
      let stats = writer.get_doc_stats()?;
      assert_eq!(expected_doc_count, stats.num_docs);
      assert_eq!(expected_doc_count, stats.max_doc);

      writer.close()?;
      drop(writer);

      let analyzer = MockAnalyzer::with_automaton(random, mock_tokenizer::SIMPLE.clone(), true);
      let mut append_config = new_index_writer_config_with_analyzer(random, analyzer);
      append_config.set_open_mode(OpenMode::Append);
      append_config.set_max_buffered_docs(2);
      writer = IndexWriter::new(directory.clone(), append_config)?;

      let reader = directory_reader::open(directory.clone())?;
      let top_reader_context = get_context(&reader)?;
      assert_eq!(1, top_reader_context.leaves()?.len());
      assert_eq!(expected_doc_count, reader.num_docs()?);
    }

    writer.close()?;
    Ok(())
  }
}

// TODO IMPORTANT 测试未通过 会卡死
fn test_threaded_force_merge() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  TestThreadedForceMerge::new().run_test(&mut random, directory)
}
