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
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::MAX_TERM_LENGTH;
use crate::core::index::multi_terms;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::term_query::TermQuery;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::line_file_docs::LineFileDocs;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, is_night_mode, new_directory_shared, new_searcher_with_reader, random, random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestSameScoresWithThreads;

// TODO IMPORTANT 多线程查询 BUG
fn test() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut analyzer = MockAnalyzer::new(&mut random);
  analyzer.set_max_token_length(TestUtil::next_int(&mut random, 1, MAX_TERM_LENGTH));
  let w = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), analyzer);
  let mut docs = LineFileDocs::new(&mut random)?;
  let chars_to_index = at_least(&mut random, 100000);
  let mut chars_indexed = 0;
  while chars_indexed < chars_to_index {
    let doc = docs.next_doc()?;
    chars_indexed += doc.get("body")?.unwrap().len() as i32;
    w.add_document(doc)?;
  }
  let r = Arc::new(w.get_reader()?);
  w.close()?;

  let s = new_searcher_with_reader(r.clone())?;
  let terms = multi_terms::get_terms(&r, "body")?.unwrap();
  let mut term_count = 0;
  let mut terms_enum = terms.iterator()?;
  while terms_enum.next()?.is_some() {
    term_count += 1;
  }
  assert!(term_count > 0);

  // Target ~10 terms to search:
  let chance = 10.0 / term_count as f64;
  terms_enum = terms.iterator()?;
  let mut answers = HashMap::new();
  while terms_enum.next()?.is_some() {
    if random.random::<f64>() <= chance {
      let term = BytesRef::deep_copy_of(terms_enum.term()?.as_ref());
      answers.insert(
        term.clone(),
        s.search(TermQuery::new(Term::new("body", term)), 100)?,
      );
    }
  }

  if !answers.is_empty() {
    let num_threads = if is_night_mode() {
      TestUtil::next_int(&mut random, 2, 5)
    } else {
      2
    } as usize;
    let starting_gun = Barrier::new(num_threads + 1);
    thread::scope(|scope| {
      let mut threads = Vec::new();
      for _ in 0..num_threads {
        let seed = random.random();
        let starting_gun = &starting_gun;
        let answers = &answers;
        let s = &s;
        threads.push(scope.spawn(move || -> Result<()> {
          let mut random = random_from_seed(seed);
          starting_gun.wait();
          for _ in 0..20 {
            let mut shuffled = answers.iter().collect::<Vec<_>>();
            shuffled.shuffle(&mut random);
            for (term, expected) in shuffled {
              let actual = s.search(TermQuery::new(Term::new("body", term.clone())), 100)?;
              assert_eq!(expected.total_hits.value(), actual.total_hits.value());
              assert_eq!(
                expected.score_docs.len(),
                actual.score_docs.len(),
                "query={}",
                term.utf8_to_string()?
              );
              for hit in 0..expected.score_docs.len() {
                assert_eq!(expected.score_docs[hit].doc, actual.score_docs[hit].doc);
                // Floats really should be identical:
                assert!(expected.score_docs[hit].score == actual.score_docs[hit].score);
              }
            }
          }
          Ok(())
        }));
      }
      starting_gun.wait();
      for thread in threads {
        thread.join().expect("thread panicked")?;
      }
      Ok::<(), crate::core::util::error::lucene_error::LuceneError>(())
    })?;
  }
  docs.close();
  r.close()?;
  Ok(())
}
