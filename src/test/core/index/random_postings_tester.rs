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
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::ToInt;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::random_from_seed;
use crate::test::core::util::test_util::TestUtil;
use rand::rngs::StdRng;
use rand::{Rng, RngExt};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Which features to test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Option_ {
  /// Sometimes use `.advance()`.
  Skipping,

  /// Sometimes reuse the `PostingsEnum` across terms.
  ReuseEnums,

  /// Sometimes pass non-null live docs.
  LiveDocs,

  /// Sometimes seek to term using previously saved `TermState`.
  TermState,

  /// Sometimes don't fully consume docs from the enum.
  PartialDocConsume,

  /// Sometimes don't fully consume positions at each doc.
  PartialPosConsume,

  /// Sometimes check payloads.
  Payloads,

  /// Test w/ multiple threads.
  Threads,
}
pub struct RandomPostingsTester {
  fields: HashMap<String, BTreeMap<BytesRef<Vec<u8>>, SeedAndOrd>>,
  field_infos: Arc<FieldInfos>,
  all_terms: Vec<FieldAndTerm>,
  max_doc: i32,
  random: u64,
}
struct SeedAndOrd {
  seed: i64,
  ord: i64,
}

impl SeedAndOrd {
  fn new(seed: i64) -> Self {
    Self { seed, ord: 0 }
  }
}

// pub struct SeedFields;
// impl Fields for SeedFields {
//   type FieldIter<'a>
//   where
//     Self: 'a,
//   = DummyFields;
//
//   fn iterator(&self) -> Result<Self::FieldIter<'_>> {
//     todo!()
//   }
//
//   type Terms = DummyTerms;
//
//   fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
//     todo!()
//   }
//
//   fn size(&self) -> Result<i32> {
//     todo!()
//   }
// }
//
// pub fn get_seed_postings(
//   term: &str,
//   seed: u64,
//   options: IndexOptions,
//   allow_payloads: bool,
// ) -> SeedPostings {
//   let random_multiplier = random_multiplier();
//   let (min_doc_freq, max_doc_freq) = if term.starts_with("big_") {
//     (random_multiplier * 50000, random_multiplier * 70000)
//   } else if term.starts_with("medium_") {
//     (random_multiplier * 3000, random_multiplier * 6000)
//   } else if term.starts_with("low_") {
//     (random_multiplier, random_multiplier * 40)
//   } else {
//     (1, 3)
//   };
//
//   SeedPostings::new(seed, min_doc_freq, max_doc_freq, options, allow_payloads)
// }
pub struct SeedPostings {
  // Used only to generate docIDs; this way if you pull w/
  // or w/o positions you get the same docID sequence:
  doc_random: StdRng,
  random: StdRng,
  pub doc_freq: i32,
  max_doc_spacing: i32,
  payload_size: i32,
  fixed_payloads: bool,
  payload: BytesRef<Vec<u8>>,
  do_positions: bool,
  allow_payloads: bool,

  doc_id: i32,
  freq: i32,
  pub upto: i32,

  pos: i32,
  offset: i32,
  start_offset: i32,
  end_offset: i32,
  pos_spacing: i32,
  pos_upto: i32,
}

impl SeedPostings {
  pub fn new(
    seed: u64,
    min_doc_freq: i32,
    max_doc_freq: i32,
    options: IndexOptions,
    allow_payloads: bool,
  ) -> Self {
    let mut random = random_from_seed(seed);
    let doc_random_seed = random.next_u64();
    let doc_random = random_from_seed(doc_random_seed);
    let doc_freq = TestUtil::next_int(&mut random, min_doc_freq, max_doc_freq);
    let max_doc_spacing = TestUtil::next_int(&mut random, 1, 100);

    let payload_size = if random.random_range(0..10) == 7 {
      1 + random.random_range(0..3)
    } else {
      1 + random.random_range(0..1)
    };

    let fixed_payloads = random.random_bool(0.5);
    let payload_bytes = vec![0u8; payload_size as usize];
    let payload = BytesRef::from_bytes(payload_bytes);
    let do_positions = IndexOptions::DocsAndFreqsAndPositions
      .cmp(&options)
      .to_int()
      <= 0;

    Self {
      doc_random,
      random,
      doc_freq,
      max_doc_spacing,
      payload_size,
      fixed_payloads,
      payload,
      do_positions,
      allow_payloads,
      doc_id: -1,
      freq: 0,
      upto: 0,
      pos: 0,
      offset: 0,
      start_offset: 0,
      end_offset: 0,
      pos_spacing: 0,
      pos_upto: 0,
    }
  }
}

impl SeedPostings {
  fn _next_doc(&mut self) -> Result<i32> {
    if self.doc_id == -1 {
      self.doc_id = 0;
    }

    while self.pos_upto < self.freq {
      self.next_position()?;
    }

    if self.upto < self.doc_freq {
      if self.upto == 0 && self.doc_random.random_bool(0.5) {
      } else if self.max_doc_spacing == 1 {
        self.doc_id += 1;
      } else {
        self.doc_id += TestUtil::next_int(&mut self.doc_random, 1, self.max_doc_spacing);
      }

      if self.random.random_range(0..200) == 17 {
        self.freq = TestUtil::next_int(&mut self.random, 1, 1000);
      } else if self.random.random_range(0..10) == 17 {
        self.freq = TestUtil::next_int(&mut self.random, 1, 20);
      } else {
        self.freq = TestUtil::next_int(&mut self.random, 1, 4);
      }

      self.pos = 0;
      self.offset = 0;
      self.pos_upto = 0;
      self.pos_spacing = TestUtil::next_int(&mut self.random, 1, 100);

      self.upto += 1;
      Ok(self.doc_id)
    } else {
      self.doc_id = NO_MORE_DOCS;
      Ok(self.doc_id)
    }
  }
}

impl DocIdSetIterator for SeedPostings {
  fn doc_id(&self) -> i32 {
    self.doc_id
  }

  fn next_doc(&mut self) -> Result<i32> {
    self._next_doc()?;
    Ok(self.doc_id)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.slow_advance(target)
  }

  fn cost(&self) -> Result<i64> {
    Ok(self.doc_freq as i64)
  }
}

impl PostingsEnum for SeedPostings {
  fn freq(&mut self) -> Result<i32> {
    Ok(self.freq)
  }

  fn next_position(&mut self) -> Result<i32> {
    if !self.do_positions {
      self.pos_upto = self.freq;
      return Ok(-1);
    }

    debug_assert!(self.pos_upto < self.freq);

    if self.pos_upto == 0 && self.random.random_bool(0.5) {
    } else if self.pos_spacing == 1 {
      self.pos += 1;
    } else {
      self.pos += TestUtil::next_int(&mut self.random, 1, self.pos_spacing);
    }

    if self.payload_size != 0 {
      if self.fixed_payloads {
        self.payload.length = self.payload_size as usize;
        self.random.fill_bytes(&mut self.payload.bytes);
      } else {
        let this_payload_size = self.random.random_range(0..self.payload_size);
        if this_payload_size != 0 {
          self.payload.length = self.payload_size as usize;
          self.random.fill_bytes(&mut self.payload.bytes);
        } else {
          self.payload.length = 0;
        }
      }
    } else {
      self.payload.length = 0;
    }

    if !self.allow_payloads {
      self.payload.length = 0;
    }

    self.start_offset = self.offset + self.random.random_range(0..5);
    self.end_offset = self.start_offset + self.random.random_range(0..10);
    self.offset = self.end_offset;

    self.pos_upto += 1;
    Ok(self.pos)
  }

  fn start_offset(&self) -> Result<i32> {
    Ok(self.start_offset)
  }

  fn end_offset(&self) -> Result<i32> {
    Ok(self.end_offset)
  }

  fn get_payload(&self) -> Result<std::option::Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if self.payload.length == 0 {
      Ok(None)
    } else {
      Ok(Some(Cow::Borrowed(&self.payload)))
    }
  }
}
/// Holds one field, term and ord.
pub struct FieldAndTerm {
  field: String,
  term: BytesRef<Vec<u8>>,
  ord: i64,
}

impl FieldAndTerm {
  pub fn new(field: String, term: &BytesRef<Vec<u8>>, ord: i64) -> Self {
    Self {
      field,
      term: BytesRef::deep_copy_of(term),
      ord,
    }
  }
}
