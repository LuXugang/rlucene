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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::{IndexWriter, MAX_DOCS, set_max_docs};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config, new_string_field, random,
};
use rand::RngExt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestIndexTooManyDocs;

/*
 * This test produces a boat load of very small segments with lot of deletes which are likely deleting
 * the entire segment. see https://issues.apache.org/jira/browse/LUCENE-8043
 */
// TODO IMPORTANT 多线程索引 BUG
fn test_index_too_many_docs() -> Result<()> {
  let mut rng = random();
  let dir = new_directory_shared(&mut rng)?;
  let num_max_doc = 25;
  let mut config = new_index_writer_config(&mut rng);
  config.set_ram_buffer_size_mb(0.000001);
  let writer = IndexWriter::new(dir.clone(), config)?;

  set_max_docs(num_max_doc)?;
  let result = (|| -> Result<()> {
    let num_threads = 5 + rng.random_range(0..5);
    let latch = Arc::new(Barrier::new(num_threads));
    let indexing_done = Arc::new((Mutex::new(num_threads - 2), Condvar::new()));
    let done = Arc::new(AtomicBool::new(false));

    thread::scope(|scope| -> Result<()> {
      let mut threads = Vec::new();
      for i in 0..num_threads {
        if i >= 2 {
          let latch = latch.clone();
          let indexing_done = indexing_done.clone();
          let writer = &writer;
          threads.push(scope.spawn(move || -> Result<()> {
            set_max_docs(num_max_doc)?;
            let result = (|| -> Result<()> {
              let mut random = random();
              let mut field_types = HashMap::new();
              latch.wait();
              for _d in 0..100 {
                let mut doc = Document::new();
                let id = random.random_range(0..num_max_doc * 2).to_string();
                doc.add(new_string_field(
                  &mut random,
                  "id",
                  id.clone(),
                  Store::No,
                  &mut field_types,
                )?);
                let t = Term::from_text("id", id);
                if random.random_range(0..5) == 0 {
                  writer.delete_documents_with_queries(vec![TermQuery::new(t.clone()).into()])?;
                }
                match writer.update_document_with_term(t, doc) {
                  Ok(_) => {},
                  Err(LuceneError::IllegalArgument(message)) => {
                    assert!(
                      message
                        .message
                        .starts_with("number of documents in the index cannot exceed ")
                    );
                    assert!(message.message.contains(&num_max_doc.to_string()));
                  },
                  Err(e) => return Err(e),
                }
              }
              Ok(())
            })();
            let (lock, cvar) = &*indexing_done;
            let mut count = lock.lock().expect("indexingDone mutex poisoned");
            *count -= 1;
            cvar.notify_all();
            result
          }));
        } else {
          let latch = latch.clone();
          let done = done.clone();
          let writer = &writer;
          threads.push(scope.spawn(move || -> Result<()> {
            set_max_docs(num_max_doc)?;
            latch.wait();
            let mut open = directory_reader::open_from_writer(writer)?;
            while !done.load(Ordering::SeqCst) {
              // TODO IMPORTANT openIfChanged未实现
              let directory_reader = directory_reader::open_from_writer(writer)?;
              open.close()?;
              open = directory_reader;
            }
            open.close()?;
            Ok(())
          }));
        }
      }

      let (lock, cvar) = &*indexing_done;
      let mut count = lock.lock().expect("indexingDone mutex poisoned");
      while *count > 0 {
        count = cvar.wait(count).expect("indexingDone mutex poisoned");
      }
      done.store(true, Ordering::SeqCst);

      for thread in threads {
        thread.join().expect("thread panicked")?;
      }
      Ok(())
    })?;

    writer.close()?;
    Ok(())
  })();

  set_max_docs(MAX_DOCS)?;
  result
}
