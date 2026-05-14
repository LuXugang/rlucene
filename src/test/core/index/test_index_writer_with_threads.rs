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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, random, rarely,
};
use rand::Rng;
use rand::RngExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

pub struct TestIndexWriterWithThreads;

const SOFT_DELETES_FIELD: &str = "___soft_deletes";
/// TODO IMPORTANT openIfChanged未实现
fn test_update_single_doc_with_threads() -> Result<()> {
  let mut random = random();
  let force_merge = rarely(&mut random);
  stress_update_single_doc_with_threads(&mut random, false, force_merge)
}
/// TODO IMPORTANT openIfChanged未实现
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
  // let num_threads = if is_night_mode() {
  //   3 + random.random_range(0..3)
  // } else {
  //   3
  // };
  let num_threads = 2;
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

    let open = writer.get_reader()?;
    assert_eq!(1, open.num_docs()?);
    barrier.wait();
    while done.load(Ordering::SeqCst) < num_threads {
      if force_merge && random.random_bool(0.5) {
        writer.force_merge(1)?;
      }
      // TODO IMPORTANT 这里没有用openIfChanged
      let open = writer.get_reader()?;
      assert_eq!(1, open.num_docs()?);
    }

    for handle in handles {
      handle.join().expect("thread panicked")?;
    }
    Ok(())
  })?;

  writer.close()?;
  Ok(())
}
