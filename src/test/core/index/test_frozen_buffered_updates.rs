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
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::frozen_buffered_updates::{TermDocsIterator, TermsProviderImpl2};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{AtomicCounter, BytesRefArray, Natural, SortableBytesRefArray};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_string_field_binary, random, rarely,
};
use rand::RngExt;
use rand::prelude::IndexedRandom;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestFrozenBufferedUpdates;

#[test]
fn test_term_docs_iterator() -> Result<()> {
  for _ in 0..5 {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let iwc = new_index_writer_config(&mut random)?;
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let duplicates = random.random_bool(0.5);
    let non_matches = random.random_bool(0.5);

    let mut array = BytesRefArray::new(Arc::new(AtomicCounter::new()))?;
    let num_docs = 10 + random.random_range(0..1000);
    let mut random_ids = HashSet::new();
    for _ in 0..num_docs {
      loop {
        let s = TestUtil::random_realistic_unicode_string(&mut random);
        let id = BytesRef::from_string(&s);
        if random_ids.insert(id) {
          break;
        }
      }
    }

    let as_list: Vec<BytesRef<Vec<u8>>> = random_ids.iter().cloned().collect();
    let mut field_to_type = HashMap::new();

    for ref_ in &random_ids {
      let mut doc = Document::new();
      doc.add(new_string_field_binary(
        &mut random,
        "field",
        ref_.clone(),
        Store::No,
        &mut field_to_type,
      )?);

      array.append(ref_)?;

      if duplicates && rarely(&mut random) {
        let picked = as_list.choose(&mut random).unwrap();
        array.append(picked)?;
      }

      if non_matches && rarely(&mut random) {
        let id = loop {
          let s = TestUtil::random_realistic_unicode_string(&mut random);
          let id = BytesRef::from_string(&s);
          if !random_ids.contains(&id) {
            break id;
          }
        };
        array.append(&id)?;
      }

      writer.add_document(doc)?;
    }

    writer.force_merge(1)?;
    writer.commit()?;

    let reader = directory_reader::open(dir.clone())?;
    let irc = (&reader).get_context()?;
    let leaves = irc.leaves()?;
    assert_eq!(1, leaves.len());

    let sorted = random.random_bool(0.5);
    let mut values = if sorted {
      SortableBytesRefArray::iterator(&array, Natural::default())?
    } else {
      array.iterator()
    };
    let leaf = leaves[0].reader();
    let mut iterator = TermDocsIterator::new(TermsProviderImpl2::new(leaf), sorted);
    let mut bit_set = FixedBitSet::new(reader.max_doc()? as usize);

    while let Some(ref_) = values.next()? {
      let mut doc_id_set_iterator = iterator.next_term("field", &ref_)?;

      if !non_matches {
        assert!(doc_id_set_iterator.is_some());
      }

      if let Some(ref mut disi) = doc_id_set_iterator {
        loop {
          let doc = disi.next_doc()?;
          if doc == NO_MORE_DOCS {
            break;
          }
          if !duplicates {
            assert!(!bit_set.get(doc as usize)?);
          }
          bit_set.set(doc as usize)?;
        }
      }
    }

    assert_eq!(reader.max_doc()? as usize, bit_set.cardinality());
    writer.close()?;
  }

  Ok(())
}
