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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::term::Term;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  random, random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestStressDeletes;

/**
 * Make sure that order of adds/deletes across threads is respected as long as each ID is only
 * changed by one thread at a time.
 */
// TODO IMPORTANT 多线程 BUG
fn test() -> Result<()> {
  let mut random = random();
  let num_ids = at_least(&mut random, 100);
  let locks: Vec<Mutex<()>> = (0..num_ids).map(|_| Mutex::new(())).collect();

  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  let w = IndexWriter::new(dir.clone(), iwc)?;
  let iters = at_least(&mut random, 2000);
  let exists = Mutex::new(HashMap::new());
  let num_threads = TestUtil::next_int(&mut random, 2, 6);
  let starting_gun = Arc::new(Barrier::new(num_threads as usize + 1));
  let delete_mode = random.random_range(0..3);

  let thread_results = thread::scope(|scope| {
    let mut handles = Vec::new();
    for _ in 0..num_threads {
      let seed = random.random();
      let starting_gun = starting_gun.clone();
      let w = &w;
      let locks = &locks;
      let exists = &exists;
      handles.push(scope.spawn(move || -> Result<()> {
        let mut random = random_from_seed(seed);
        starting_gun.wait();
        for _ in 0..iters {
          let id = random.random_range(0..num_ids);
          {
            let _id_lock = locks[id as usize].lock().unwrap();
            let mut exists = exists.lock().unwrap();
            let v = exists.get(&id).copied().unwrap_or(false);
            if !v {
              let mut doc = Document::new();
              doc.add(StringField::from_string("id", id.to_string(), Store::No)?);
              w.add_document(doc)?;
              exists.insert(id, true);
            } else {
              if delete_mode == 0 {
                w.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
              } else if delete_mode == 1 {
                w.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
                // TODO delete by query 未实现
                // w.delete_documents_with_queries(vec![Query::from(TermQuery::new(
                //   Term::from_text("id", id.to_string()),
                // ))])?;
              } else if random.random_bool(0.5) {
                w.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
              } else {
                w.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
                // w.delete_documents_with_queries(vec![Query::from(TermQuery::new(
                // TODO delete by query 未实现
                //   Term::from_text("id", id.to_string()),
                // ))])?;
              }
              exists.insert(id, false);
            }
          }
          if random.random_range(0..500) == 2 {
            directory_reader::open_with_writer_deletes(w, random.random_bool(0.5), false)?
              .close()?;
          }
          if random.random_range(0..500) == 2 {
            w.commit()?;
          }
        }
        Ok(())
      }));
    }

    starting_gun.wait();
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

  let r = directory_reader::open_with_writer_deletes(&w, true, false)?;
  let s = new_searcher_with_reader(r)?;
  for (id, value) in exists.lock().unwrap().iter() {
    let hits = s.search(TermQuery::new(Term::from_text("id", id.to_string())), 1)?;
    if *value {
      assert_eq!(1, hits.total_hits.value());
    } else {
      assert_eq!(0, hits.total_hits.value());
    }
  }
  w.close()?;
  Ok(())
}
