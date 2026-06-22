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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::doc_helper::DocHelper;
use crate::test::core::index::test_index_writer_reader;
use crate::test::core::util::lucene_test_case::{
  ensure_sane_iwc_on_nightly, is_night_mode, new_directory_shared,
  new_index_writer_config_with_analyzer, new_log_merge_policy_with_merge_factor_cfs, random,
  random_from_seed,
};
use rand::RngExt;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestNRTReaderWithThreads;

#[test]
fn test_indexing() -> Result<()> {
  let mut random = random();
  let main_dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
  config.set_max_buffered_docs(10);
  config.set_merge_policy(new_log_merge_policy_with_merge_factor_cfs(
    &mut random,
    false,
    2,
  )?);
  ensure_sane_iwc_on_nightly(&mut config)?;
  let writer = IndexWriter::new(main_dir, config)?;

  let reader = directory_reader::open_from_writer(&writer)?; // start pooling readers
  reader.close()?;

  let num_threads = if is_night_mode() { 4 } else { 2 };
  let num_iterations = if is_night_mode() { 2000 } else { 50 };
  let seq = AtomicI32::new(1);

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();
    for x in 0..num_threads {
      let writer = &writer;
      let seq = &seq;
      let seed = random.random();
      handles.push(
        thread::Builder::new()
          .name(format!("Thread {x}"))
          .spawn_scoped(scope, move || -> Result<()> {
            let type_ = x % 2;
            let mut del_count = 0;
            let mut add_count = 0;
            let mut r = random_from_seed(seed);

            for _ in 0..num_iterations {
              if type_ == 0 {
                let i = seq.fetch_add(1, Ordering::SeqCst) + 1;
                let doc = DocHelper::create_document(i, "index1", 10);
                writer.add_document(doc)?;
                add_count += 1;
              } else if type_ == 1 {
                let reader = directory_reader::open_from_writer(writer)?;
                let id = r.random_range(0..seq.load(Ordering::SeqCst));
                let term = Term::from_text("id", id.to_string());
                let count = test_index_writer_reader::count(&mut r, &term, &reader)?;
                writer.delete_documents_with_terms(vec![term])?;
                reader.close()?;
                del_count += count;
              }
            }

            let _ = (del_count, add_count);
            Ok(())
          })?,
      );
    }

    for handle in handles {
      handle
        .join()
        .map_err(|_| LuceneError::illegal_state("index thread panicked"))??;
    }
    Ok(())
  })?;

  writer.close()?;

  Ok(())
}
