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
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::dummy_total_hit_count_collector::DummyTotalHitCountCollector;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config, new_searcher_with_reader,
  new_text_field, random,
};
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;

#[allow(dead_code)] // for quick search
pub struct TestSearchWithThreads;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let num_threads = if is_night_mode() { 5 } else { 4 };
  let num_searches = if is_night_mode() {
    at_least(&mut random, 2000)
  } else {
    at_least(&mut random, 500)
  };
  let num_docs = if is_night_mode() {
    at_least(&mut random, 10000)
  } else {
    at_least(&mut random, 200)
  };

  let dir = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random);
  config.set_max_buffered_docs(1000);
  let writer = RandomIndexWriter::with_config(&mut random, dir, config);
  let mut field_to_type = HashMap::new();

  for _ in 0..num_docs {
    let mut body = String::new();
    let num_terms = random.random_range(0..10);
    for _ in 0..num_terms {
      body.push_str(if random.random_bool(0.5) {
        "aaa"
      } else {
        "bbb"
      });
      body.push(' ');
    }

    let mut doc = Document::new();
    doc.add(new_text_field(
      &mut random,
      "body",
      body,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;
  }

  let reader = writer.get_reader()?;
  writer.close()?;
  let searcher = Arc::new(new_searcher_with_reader(reader)?);

  let failed = Arc::new(AtomicBool::new(false));
  let net_search = Arc::new(AtomicU64::new(0));
  let mut threads = Vec::with_capacity(num_threads as usize);

  let collector_manager = Arc::new(DummyTotalHitCountCollector::create_manager());

  for _ in 0..num_threads {
    let searcher = searcher.clone();
    let failed = failed.clone();
    let net_search = net_search.clone();
    let collector_manager = Arc::clone(&collector_manager);

    threads.push(thread::spawn(move || -> std::result::Result<(), String> {
      let result: Result<()> = (|| {
        let mut tot_hits = 0i64;
        let mut tot_search = 0i64;

        while tot_search < num_searches as i64 && !failed.load(Ordering::Relaxed) {
          tot_hits += searcher.search_with_collector_manager(
            TermQuery::new(Term::from_text("body", "aaa")),
            collector_manager.as_ref(),
          )? as i64;

          tot_hits += searcher.search_with_collector_manager(
            TermQuery::new(Term::from_text("body", "bbb")),
            collector_manager.as_ref(),
          )? as i64;

          tot_search += 1;
        }

        assert!(tot_search > 0 && tot_hits > 0);

        net_search.fetch_add(tot_search as u64, Ordering::Relaxed);

        Ok(())
      })();

      match result {
        Ok(()) => Ok(()),
        Err(e) => {
          failed.store(true, Ordering::Relaxed);
          Err(e.to_string())
        },
      }
    }));
  }

  for handle in threads {
    match handle.join() {
      Ok(result) => {
        result.map_err(crate::core::util::error::lucene_error::LuceneError::illegal_state)?
      },
      Err(_) => {
        return Err(
          crate::core::util::error::lucene_error::LuceneError::illegal_state(
            "search thread panicked",
          ),
        );
      },
    }
  }

  assert!(!failed.load(Ordering::Relaxed));
  assert!(net_search.load(Ordering::Relaxed) > 0);
  Ok(())
}
