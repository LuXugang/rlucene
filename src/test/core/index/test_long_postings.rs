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
use crate::core::analysis::analyzer::{Analyzer, REUSE_STRATEGY, ReuseStrategy};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Store::No;
use crate::core::document::field_type::FieldType;
use crate::core::document::text_field::text_field_type;
use crate::core::index::BytesRef;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_terms::get_term_postings_enum;
use crate::core::index::postings_enum::{FREQS, NONE, PostingsEnum};
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, create_temp_dir_with_prefix, new_field, new_fs_directory,
  new_index_writer_config_with_analyzer, new_log_merge_policy, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
pub struct TestLongPostings;

fn get_random_term<R>(random: &mut R, other: Option<&str>) -> Result<String>
where
  R: Rng + ?Sized,
{
  let a = MockAnalyzer::new(random);
  loop {
    let s = TestUtil::random_realistic_unicode_string(random);
    if other.is_some_and(|other| s == other) {
      continue;
    }
    let field_name = "foo";
    a.token_stream(field_name, &s)?;
    let unchanged_single_token = REUSE_STRATEGY.with(|reuse_strategy| {
      (|| -> Result<bool> {
        let mut reuse_strategy = reuse_strategy.borrow_mut();
        let ts = match reuse_strategy.as_mut() {
          Some(rs) => rs
            .get_reusable_components(field_name)?
            .expect("reuse strategy components must exist after token_stream")
            .get_token_stream(),
          None => panic!("reuse strategy is not initialized"),
        };
        ts.reset()?;

        let mut count = 0;
        let mut changed = false;

        while ts.increment_token()? {
          let term_bytes = ts.get_attribute_source_mut().get_bytes_ref()?;
          if count == 0 {
            if let Some(term_bytes) = term_bytes {
              if term_bytes.utf8_to_string()? != s {
                changed = true;
              }
            } else {
              changed = true;
            }
          }
          count += 1;
        }

        ts.end()?;
        Ok(!changed && count == 1)
      })()
    })?;
    if unchanged_single_token {
      return Ok(s);
    }
  }
}

// #[test]
fn test_long_postings() -> Result<()> {
  let mut random = random();
  let dir_suffix = random.random::<i64>();
  let dir = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix(format!("longpostings.{dir_suffix}"))?,
  )?;

  let num_docs = at_least(&mut random, 1000);

  let s1 = get_random_term(&mut random, None)?;
  let s2 = get_random_term(&mut random, Some(&s1))?;

  let mut is_s1 = FixedBitSet::new(num_docs as usize);
  for idx in 0..num_docs {
    if random.random_bool(0.5) {
      is_s1.set(idx as usize);
    }
  }

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_open_mode(OpenMode::Create);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  iwc.set_ram_buffer_size_mb(16.0 + 16.0 * random.random::<f64>());
  iwc.set_max_buffered_docs(-1);
  let riw = RandomIndexWriter::with_config(&mut random, dir, iwc);
  let mut field_types = HashMap::new();

  for idx in 0..num_docs {
    let mut doc = Document::new();
    let s = if is_s1.get(idx as usize)? { &s1 } else { &s2 };
    let f = new_text_field(&mut random, "field", s, No, &mut field_types)?;
    let count = TestUtil::next_int(&mut random, 1, 4);
    for _ in 0..count {
      doc.add(f.clone());
    }
    riw.add_document(doc)?;
  }

  let r = riw.get_reader()?;
  riw.close()?;

  assert_eq!(num_docs, r.num_docs()?);
  assert!(r.doc_freq(&Term::from_text("field", &s1))? > 0);
  assert!(r.doc_freq(&Term::from_text("field", &s2))? > 0);

  let num = at_least(&mut random, 1000);
  for _ in 0..num {
    let (term, do_s1) = if random.random_bool(0.5) {
      (&s1, true)
    } else {
      (&s2, false)
    };

    let mut postings = get_term_postings_enum(&r, "field", &BytesRef::from_string(term))?
      .expect("postings enum must exist");

    let mut doc_id = -1;
    while doc_id < NO_MORE_DOCS {
      let what = random.random_range(0..3);
      if what == 0 {
        let mut expected = doc_id + 1;
        loop {
          if expected == num_docs {
            expected = NO_MORE_DOCS;
            break;
          } else if is_s1.get(expected as usize)? == do_s1 {
            break;
          } else {
            expected += 1;
          }
        }

        doc_id = postings.next_doc()?;
        assert_eq!(expected, doc_id);
        if doc_id == NO_MORE_DOCS {
          break;
        }

        if random.random_range(0..6) == 3 {
          let freq = postings.freq()?;
          assert!((1..=4).contains(&freq));
          for pos in 0..freq {
            assert_eq!(pos, postings.next_position()?);
            if random.random_bool(0.5) {
              let _ = postings.get_payload()?;
              if random.random_bool(0.5) {
                let _ = postings.get_payload()?;
              }
            }
          }
        }
      } else {
        let target_doc_id = if doc_id == -1 {
          random.random_range(0..=num_docs)
        } else {
          doc_id + TestUtil::next_int(&mut random, 1, num_docs - doc_id)
        };

        let mut expected = target_doc_id;
        loop {
          if expected == num_docs {
            expected = NO_MORE_DOCS;
            break;
          } else if is_s1.get(expected as usize)? == do_s1 {
            break;
          } else {
            expected += 1;
          }
        }

        doc_id = postings.advance(target_doc_id)?;
        assert_eq!(expected, doc_id);
        if doc_id == NO_MORE_DOCS {
          break;
        }

        if random.random_range(0..6) == 3 {
          let freq = postings.freq()?;
          assert!((1..=4).contains(&freq));
          for pos in 0..freq {
            assert_eq!(pos, postings.next_position()?);
            if random.random_bool(0.5) {
              let _ = postings.get_payload()?;
              if random.random_bool(0.5) {
                let _ = postings.get_payload()?;
              }
            }
          }
        }
      }
    }
  }

  r.close()?;
  Ok(())
}

// #[test]
fn test_long_postings_no_positions() -> Result<()> {
  do_test_long_postings_no_positions(IndexOptions::Docs)?;
  do_test_long_postings_no_positions(IndexOptions::DocsAndFreqs)?;
  Ok(())
}

fn do_test_long_postings_no_positions(options: IndexOptions) -> Result<()> {
  let mut random = random();
  let dir_suffix = random.random::<i64>();
  let dir = new_fs_directory(
    &mut random,
    create_temp_dir_with_prefix(format!("longpostings.{dir_suffix}"))?,
  )?;

  let num_docs = at_least(&mut random, 1000);

  let s1 = get_random_term(&mut random, None)?;
  let s2 = get_random_term(&mut random, Some(&s1))?;

  let mut is_s1 = FixedBitSet::new(num_docs as usize);
  for idx in 0..num_docs {
    if random.random_bool(0.5) {
      is_s1.set(idx as usize);
    }
  }

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_open_mode(OpenMode::Create);
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  iwc.set_ram_buffer_size_mb(16.0 + 16.0 * random.random::<f64>());
  iwc.set_max_buffered_docs(-1);
  let riw = RandomIndexWriter::with_config(&mut random, dir, iwc);

  let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  ft.set_index_options(options)?;
  let mut field_types = HashMap::new();
  for idx in 0..num_docs {
    let mut doc = Document::new();
    let s = if is_s1.get(idx as usize)? { &s1 } else { &s2 };
    let f = new_field(&mut random, "field", s.as_str(), &ft, &mut field_types)?;
    let count = TestUtil::next_int(&mut random, 1, 4);
    for _ in 0..count {
      doc.add(f.clone());
    }
    riw.add_document(doc)?;
  }

  let r = riw.get_reader()?;
  riw.close()?;

  assert_eq!(num_docs, r.num_docs()?);
  assert!(r.doc_freq(&Term::from_text("field", &s1))? > 0);
  assert!(r.doc_freq(&Term::from_text("field", &s2))? > 0);

  let num = at_least(&mut random, 1000);
  for _ in 0..num {
    let (term, do_s1) = if random.random_bool(0.5) {
      (&s1, true)
    } else {
      (&s2, false)
    };

    let flags = if options == IndexOptions::Docs {
      NONE as i32
    } else {
      FREQS as i32
    };
    let mut docs = TestUtil::docs_with_reader(
      &mut random,
      &r,
      "field",
      &BytesRef::from_string(term),
      None,
      flags,
    )?
    .expect("docs enum must exist");

    let mut doc_id = -1;
    while doc_id < NO_MORE_DOCS {
      let what = random.random_range(0..3);
      if what == 0 {
        let mut expected = doc_id + 1;
        loop {
          if expected == num_docs {
            expected = NO_MORE_DOCS;
            break;
          } else if is_s1.get(expected as usize)? == do_s1 {
            break;
          } else {
            expected += 1;
          }
        }

        doc_id = docs.next_doc()?;
        assert_eq!(expected, doc_id);
        if doc_id == NO_MORE_DOCS {
          break;
        }

        if random.random_range(0..6) == 3 && options != IndexOptions::Docs {
          let freq = docs.freq()?;
          assert!((1..=4).contains(&freq));
        }
      } else {
        let target_doc_id = if doc_id == -1 {
          random.random_range(0..=num_docs)
        } else {
          doc_id + TestUtil::next_int(&mut random, 1, num_docs - doc_id)
        };

        let mut expected = target_doc_id;
        loop {
          if expected == num_docs {
            expected = NO_MORE_DOCS;
            break;
          } else if is_s1.get(expected as usize)? == do_s1 {
            break;
          } else {
            expected += 1;
          }
        }

        doc_id = docs.advance(target_doc_id)?;
        assert_eq!(expected, doc_id);
        if doc_id == NO_MORE_DOCS {
          break;
        }

        if random.random_range(0..6) == 3 && options != IndexOptions::Docs {
          let freq = docs.freq()?;
          assert!((1..=4).contains(&freq), "got invalid freq={freq}");
        }
      }
    }
  }

  r.close()?;
  Ok(())
}
