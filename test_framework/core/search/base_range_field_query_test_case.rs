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
use crate::core::document::fields::Fields;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::query::Query;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::fixed_bit_set_collector::FixedBitSetCollector;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, create_temp_dir_with_prefix, new_directory_shared, new_fs_directory,
  new_index_writer_config, new_searcher_with_reader,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::fmt::Display;

pub(crate) trait BaseRangeFieldQueryTestCase {
  type Range: Range + Clone + Display;
  type RangeField: Into<Fields>;

  fn new_range_field(&self, box_: &Self::Range) -> Result<Self::RangeField>;

  fn new_intersects_query(&self, box_: &Self::Range) -> Result<Query>;

  fn new_contains_query(&self, box_: &Self::Range) -> Result<Query>;

  fn new_within_query(&self, box_: &Self::Range) -> Result<Query>;

  fn new_crosses_query(&self, box_: &Self::Range) -> Result<Query>;

  fn next_range<R>(&self, random: &mut R, dimensions: usize) -> Result<Self::Range>
  where
    R: Rng + ?Sized;

  fn dimension<R>(&self, random: &mut R) -> usize
  where
    R: Rng + ?Sized,
  {
    random.random_range(0..4) + 1
  }

  fn test_random_tiny<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // Make sure single-leaf-node case is OK:
    for _ in 0..10 {
      self.do_test_random(random, 10, false)?;
    }
    Ok(())
  }

  fn test_random_medium<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_random(random, 1000, false)
  }

  fn test_random_big<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_random(random, 200000, false)
  }

  fn test_multi_valued<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.do_test_random(random, 1000, true)
  }

  fn test_all_equal<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_docs = at_least(random, 1000) as usize;
    let dimensions = self.dimension(random);
    let the_range = vec![self.next_range(random, dimensions)?];
    let ranges = vec![the_range; num_docs];
    self.verify(random, &ranges)
  }

  // Force low cardinality leaves
  fn test_low_cardinality<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_docs = at_least(random, 1000) as usize;
    let dimensions = self.dimension(random);

    let cardinality = TestUtil::next_int(random, 2, 20) as usize;
    let mut diff_ranges = Vec::with_capacity(cardinality);
    for _ in 0..cardinality {
      diff_ranges.push(vec![self.next_range(random, dimensions)?]);
    }

    let mut ranges = Vec::with_capacity(num_docs);
    for _ in 0..num_docs {
      ranges.push(diff_ranges[random.random_range(0..cardinality)].clone());
    }
    self.verify(random, &ranges)
  }

  fn do_test_random<R>(&self, random: &mut R, count: i32, multi_valued: bool) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let num_docs = at_least(random, count) as usize;
    let dimensions = self.dimension(random);

    let mut ranges = vec![Vec::new(); num_docs];

    let have_real_doc = true;

    'nextdoc: for id in 0..num_docs {
      let x = random.random_range(0..20);
      if ranges[id].is_empty() {
        ranges[id].push(self.next_range(random, dimensions)?);
      }
      if x == 17 {
        // some docs don't have a box:
        ranges[id][0].set_missing(true);
        continue;
      }

      if multi_valued && random.random_bool(0.5) {
        // randomly add multi valued documents (up to 2 fields)
        let n = random.random_range(0..2) + 1;
        ranges[id] = Vec::with_capacity(n);
        for _ in 0..n {
          ranges[id].push(self.next_range(random, dimensions)?);
        }
      }

      if id > 0 && x < 9 && have_real_doc {
        let mut i = 0;
        let old_id = loop {
          let old_id = random.random_range(0..id);
          if !ranges[old_id][0].is_missing() {
            break old_id;
          }
          i += 1;
          if i > id {
            continue 'nextdoc;
          }
        };

        if x == dimensions * 2 {
          // Fully identical box (use first box in case current is multivalued but old is not)
          for d in 0..dimensions {
            let min = ranges[old_id][0].get_min(d);
            let max = ranges[old_id][0].get_max(d);
            ranges[id][0].set_min(d, min);
            ranges[id][0].set_max(d, max);
          }
        } else {
          for m in 0..dimensions * 2 {
            let even = dimensions % 2;
            if x == m {
              let d = m / 2;
              // current could be multivalue but old may not be, so use first box
              if even == 0 {
                // even is min
                let min = ranges[old_id][0].get_min(d);
                ranges[id][0].set_min(d, min);
              } else {
                // odd is max
                let max = ranges[old_id][0].get_max(d);
                ranges[id][0].set_max(d, max);
              }
            }
          }
        }
      }
    }
    self.verify(random, &ranges)
  }

  fn verify<R>(&self, random: &mut R, ranges: &[Vec<Self::Range>]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut iwc = new_index_writer_config(random)?;
    // Else seeds may not reproduce:
    iwc.set_merge_scheduler(SerialMergeScheduler::new());
    // Else we can get O(N^2) merging
    let mbd = iwc.get_max_buffered_docs();
    if mbd != -1 && mbd < (ranges.len() / 100) as i32 {
      iwc.set_max_buffered_docs((ranges.len() / 100) as i32);
    }
    let dir = if ranges.len() > 50000 {
      // Avoid slow codecs like SimpleText
      iwc.set_codec(TestUtil::get_default_codec());
      new_fs_directory(
        random,
        create_temp_dir_with_prefix(std::any::type_name::<Self>())?,
      )?
    } else {
      new_directory_shared(random)?
    };

    let mut deleted = HashSet::new();
    let w = IndexWriter::new(dir, iwc)?;
    #[allow(clippy::needless_range_loop)]
    for id in 0..ranges.len() {
      let mut doc = Document::new();
      doc.add(StringField::from_string("id", id.to_string(), Store::No)?);
      doc.add(NumericDocValuesField::new("id", id as i64));
      if !ranges[id][0].is_missing() {
        for n in 0..ranges[id].len() {
          self.add_range(&mut doc, &ranges[id][n])?;
        }
      }
      w.add_document(doc)?;
      if id > 0 && random.random_range(0..100) == 1 {
        let id_to_delete = random.random_range(0..id);
        w.delete_documents_with_terms(vec![Term::from_text("id", id_to_delete.to_string())])?;
        deleted.insert(id_to_delete);
      }
    }

    if random.random_bool(0.5) {
      w.force_merge(1)?;
    }
    let r = directory_reader::open_from_writer(&w)?;
    w.close()?;
    let s = new_searcher_with_reader(r)?;

    let dimensions = ranges[0][0].num_dimensions();
    let iters = at_least(random, 25);
    let live_docs = get_live_docs(s.get_index_reader())?;
    let max_doc = s.get_index_reader().max_doc()?;

    for iter in 0..iters {
      let query_range = self.next_range(random, dimensions)?;
      let rv = random.random_range(0..4);
      let (query, query_type) = if rv == 0 {
        (
          self.new_intersects_query(&query_range)?,
          QueryType::Intersects,
        )
      } else if rv == 1 {
        (self.new_contains_query(&query_range)?, QueryType::Contains)
      } else if rv == 2 {
        (self.new_within_query(&query_range)?, QueryType::Within)
      } else {
        (self.new_crosses_query(&query_range)?, QueryType::Crosses)
      };

      let hits =
        s.search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))?;

      let mut doc_id_to_id = MultiDocValues::get_numeric_values(s.get_index_reader(), "id")?
        .expect("id doc values should exist");
      for doc_id in 0..max_doc {
        assert_eq!(doc_id, doc_id_to_id.next_doc()?);
        let id = doc_id_to_id.long_value()? as usize;
        let is_live = live_docs
          .as_ref()
          .is_none_or(|live_docs| live_docs.get(doc_id as usize).expect(""));
        let expected = if !is_live || ranges[id][0].is_missing() {
          false
        } else {
          self.expected_result(&query_range, &ranges[id], query_type)
        };

        if hits.get(doc_id as usize)? != expected {
          let mut b = String::new();
          b.push_str(&format!("FAIL (iter {iter}): "));
          if expected {
            b.push_str(&format!(
              "id={} {}should match but did not\n",
              id,
              if ranges[id].len() > 1 {
                "(MultiValue) "
              } else {
                ""
              }
            ));
          } else {
            b.push_str(&format!("id={id} should not match but did\n"));
          }
          b.push_str(&format!(" queryRange={query_range}\n"));
          b.push_str(if ranges[id].len() > 1 {
            " boxes="
          } else {
            " box="
          });
          b.push_str(&ranges[id][0].to_string());
          #[allow(clippy::needless_range_loop)]
          for n in 1..ranges[id].len() {
            b.push_str(", ");
            b.push_str(&ranges[id][n].to_string());
          }
          b.push_str(&format!("\n queryType={query_type:?}\n"));
          b.push_str(&format!(" deleted?={}", !is_live));
          unreachable!("wrong hit (first of possibly more):\n\n{b}");
        }
      }
    }

    Ok(())
  }

  fn add_range(&self, doc: &mut Document, box_: &Self::Range) -> Result<()> {
    doc.add(self.new_range_field(box_)?);
    Ok(())
  }

  fn expected_result(
    &self,
    query_range: &Self::Range,
    range: &[Self::Range],
    query_type: QueryType,
  ) -> bool {
    for r in range {
      if self.expected_bbox_query_result(query_range, r, query_type) {
        return true;
      }
    }
    false
  }

  fn expected_bbox_query_result(
    &self,
    query_range: &Self::Range,
    range: &Self::Range,
    query_type: QueryType,
  ) -> bool {
    if query_range.is_equal(range) && query_type != QueryType::Crosses {
      return true;
    }
    let relation = range.relate(query_range);
    if query_type == QueryType::Intersects {
      relation.is_some()
    } else if query_type == QueryType::Crosses {
      // by definition, RangeFields that CONTAIN the query are also considered to cross
      relation == Some(query_type) || relation == Some(QueryType::Contains)
    } else {
      relation == Some(query_type)
    }
  }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RangeBase {
  pub(crate) is_missing: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryType {
  Intersects,
  Within,
  Contains,
  Crosses,
}

pub(crate) trait Range {
  type Value: Clone;

  fn get_base(&self) -> &RangeBase;

  fn get_base_mut(&mut self) -> &mut RangeBase;

  fn is_missing(&self) -> bool {
    self.get_base().is_missing
  }

  fn set_missing(&mut self, is_missing: bool) {
    self.get_base_mut().is_missing = is_missing;
  }

  fn num_dimensions(&self) -> usize;

  fn get_min(&self, dim: usize) -> Self::Value;

  fn set_min(&mut self, dim: usize, val: Self::Value);

  fn get_max(&self, dim: usize) -> Self::Value;

  fn set_max(&mut self, dim: usize, val: Self::Value);

  fn is_equal(&self, other: &Self) -> bool;

  fn is_disjoint(&self, other: &Self) -> bool;

  fn is_within(&self, other: &Self) -> bool;

  fn contains(&self, other: &Self) -> bool;

  fn relate(&self, other: &Self) -> Option<QueryType> {
    if self.is_disjoint(other) {
      None
    } else if self.is_within(other) {
      Some(QueryType::Within)
    } else if self.contains(other) {
      Some(QueryType::Contains)
    } else {
      Some(QueryType::Crosses)
    }
  }
}
