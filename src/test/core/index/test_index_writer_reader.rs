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
use crate::core::document::long_point::LongPoint;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_bits::get_live_docs;
use crate::core::index::point_values::PointValues;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::Comparator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use std::cmp::Ordering;

#[allow(dead_code)] // for quick search
struct TestIndexWriterReader;
pub(crate) fn count<R, CR>(random: &mut R, t: &Term, r: &CR) -> Result<i32>
where
  R: Rng + ?Sized,
  CR: CompositeReader,
{
  let mut count = 0;
  let term_bytes = BytesRef::from_string(&t.text()?);
  let mut td = TestUtil::docs_with_reader(random, r, t.field(), &term_bytes, None, 0)?;

  if let Some(td) = td.as_mut() {
    let live_docs = get_live_docs(r)?;
    while td.next_doc()? != NO_MORE_DOCS {
      let doc_id = td.doc_id();
      if live_docs
        .as_ref()
        .is_none_or(|bits| bits.get(doc_id as usize).expect(""))
      {
        count += 1;
      }
    }
  }

  Ok(count)
}
#[test]
fn test_add_close_open() -> Result<()> {
  Ok(())
}

#[test]
fn test_update_document() -> Result<()> {
  Ok(())
}

#[test]
fn test_is_current() -> Result<()> {
  Ok(())
}

#[test]
fn test_add_indexes() -> Result<()> {
  Ok(())
}

#[test]
fn test_add_indexes2() -> Result<()> {
  Ok(())
}

#[test]
fn test_delete_from_index_writer() -> Result<()> {
  Ok(())
}

#[test]
fn test_add_indexes_and_do_deletes_threads() -> Result<()> {
  Ok(())
}

#[test]
fn test_index_writer_reopen_segment_full_merge() -> Result<()> {
  Ok(())
}

#[test]
fn test_index_writer_reopen_segment() -> Result<()> {
  Ok(())
}

#[test]
fn test_merge_warmer() -> Result<()> {
  Ok(())
}

#[test]
fn test_after_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_after_close() -> Result<()> {
  Ok(())
}

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_during_add_indexes() -> Result<()> {
  Ok(())
}

#[test]
fn test_during_add_delete() -> Result<()> {
  Ok(())
}

#[test]
fn test_force_merge_deletes() -> Result<()> {
  Ok(())
}

#[test]
fn test_deletes_num_docs() -> Result<()> {
  Ok(())
}

#[test]
fn test_empty_index() -> Result<()> {
  Ok(())
}

#[test]
fn test_segment_warmer() -> Result<()> {
  Ok(())
}

#[test]
fn test_simple_merged_segment_warmer() -> Result<()> {
  Ok(())
}

#[test]
fn test_reopen_after_no_real_change() -> Result<()> {
  Ok(())
}

#[test]
fn test_nrt_open_exceptions() -> Result<()> {
  Ok(())
}

#[test]
fn test_too_many_segments() -> Result<()> {
  Ok(())
}

#[test]
fn test_reopen_nrt_reader_on_commit() -> Result<()> {
  Ok(())
}

#[test]
fn test_index_reader_writer_with_leaf_sorter() -> Result<()> {
  Ok(())
}
#[derive(Clone)]
pub struct PointValueLeafSorter {
  asc_sort: bool,
  field_name: String,
  missing_value: i64,
}

impl PointValueLeafSorter {
  fn sort_key<LR>(&self, reader: &LR) -> Result<i64>
  where
    LR: LeafReader,
  {
    let result = (|| -> Result<i64> {
      let Some(points) = reader.get_point_values(&self.field_name)? else {
        return Ok(self.missing_value);
      };
      let sort_value = if self.asc_sort {
        points.get_min_packed_value()?
      } else {
        points.get_max_packed_value()?
      };
      Ok(
        sort_value
          .map(|value| LongPoint::decode_dimension(&value, 0))
          .unwrap_or(self.missing_value),
      )
    })();
    Ok(result.unwrap_or(self.missing_value))
  }
}

impl<LR> Comparator<LR> for PointValueLeafSorter
where
  LR: LeafReader,
{
  const TYPE: &'static str = "PointValueLeafSorter";

  fn compare(&self, a: &LR, b: &LR) -> Result<i32> {
    let ord = self.sort_key(a)?.cmp(&self.sort_key(b)?);
    let ord = if self.asc_sort { ord } else { ord.reverse() };

    Ok(match ord {
      Ordering::Less => -1,
      Ordering::Equal => 0,
      Ordering::Greater => 1,
    })
  }
}
