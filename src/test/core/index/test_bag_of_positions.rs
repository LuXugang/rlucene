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
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::composite_reader::get_context;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, create_temp_dir_with_prefix, new_fs_directory, new_index_writer_config_with_analyzer,
  random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::VecDeque;
use std::sync::{Barrier, Mutex};
use std::thread;

/// Simple test that adds numeric terms, where each term has the totalTermFreq of its integer
/// value, and checks that the totalTermFreq is correct.
#[allow(dead_code)] // for quick search
pub struct TestBagOfPositions;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let mut postings_list: Vec<String> = Vec::new();
  let num_terms = at_least(&mut random, 100);
  let max_terms_per_doc = TestUtil::next_int(&mut random, 10, 20);

  // Build postings list: term "i" appears i times
  for i in 0..num_terms {
    let term = i.to_string();
    for _ in 0..i {
      postings_list.push(term.clone());
    }
  }

  // Shuffle
  postings_list.shuffle(&mut random);

  let postings = Mutex::new(VecDeque::from(postings_list));

  let dir = new_fs_directory(&mut random, create_temp_dir_with_prefix("bagofpositions")?)?;

  let a = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, a);
  let iw = RandomIndexWriter::with_config(&mut random, dir, iwc);

  let thread_count = TestUtil::next_int(&mut random, 1, 5);

  // Build field type
  let mut field_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  if random.random_bool(0.5) {
    field_type.set_omit_norms(true)?;
  }
  let options = random.random_range(0..3);
  if options == 0 {
    field_type.set_index_options(IndexOptions::DocsAndFreqs)?;
    // we dont actually need positions, but enforce term vectors when we do this so we check
    // SOMETHING
    field_type.set_store_term_vectors(true)?;
  } else if options == 1 {
    field_type.set_index_options(IndexOptions::DocsAndFreqsAndPositionsAndOffsets)?;
  }
  // else just positions (default)

  let barrier = Barrier::new(thread_count as usize + 1);

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();

    for _thread_id in 0..thread_count {
      let postings = &postings;
      let iw = &iw;
      let barrier = &barrier;
      let thread_seed = random.next_u64();
      let field_type = field_type.clone();

      handles.push(scope.spawn(move || -> Result<()> {
        let mut thread_random = StdRng::seed_from_u64(thread_seed);

        let _ = barrier.wait();

        loop {
          // Check if the queue is empty (equivalent to Java's ConcurrentLinkedQueue.isEmpty())
          {
            let queue = postings.lock().unwrap();
            if queue.is_empty() {
              break;
            }
          }

          // Build text from queue
          let mut text = String::new();
          let num_terms_in_doc = thread_random.random_range(0..max_terms_per_doc);
          for _ in 0..num_terms_in_doc {
            let token = {
              let mut queue = postings.lock().unwrap();
              queue.pop_front()
            };
            let token = match token {
              Some(t) => t,
              None => break,
            };
            text.push(' ');
            text.push_str(&token);
          }

          // Create document and add field
          let mut doc = Document::new();
          doc.add(Field::new("field", text.as_str(), field_type.clone()));
          iw.add_document(doc)?;
        }

        Ok(())
      }));
    }

    let _ = barrier.wait();

    for handle in handles {
      handle.join().expect("thread panicked")?;
    }

    Ok(())
  })?;

  iw.force_merge(1)?;
  let ir = iw.get_reader()?;
  let top_reader_context = get_context(&ir)?;
  let leaves = top_reader_context.leaves()?;
  assert_eq!(1, leaves.len());
  let leaf_reader = leaves[0].reader();
  let terms_opt = leaf_reader.terms("field")?;
  let terms = terms_opt.expect("terms must exist");
  // numTerms-1 because there cannot be a term 0 with 0 postings:
  assert_eq!((num_terms - 1) as i64, terms.size()?);
  let mut terms_enum = terms.iterator()?;
  loop {
    let term = terms_enum.next()?;
    let term = match term {
      Some(t) => t,
      None => break,
    };
    let value: i32 = term.utf8_to_string()?.parse()?;
    assert_eq!(value as i64, terms_enum.total_term_freq()?);
  }

  drop(ir);
  iw.close()?;

  Ok(())
}
