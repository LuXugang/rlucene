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
use crate::core::index::point_values::IntersectVisitor;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::store::{DataOutput, IndexInput};
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::doc_base_bit_set_iterator::DocBaseBitSetIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::longs_ref::LongsRef;
use crate::core::util::{CoreHelper, TryIntoInt};

pub struct DocIdsWriter {
  scratch: Vec<i32>,
  scratch_longs: LongsRef,
  /// IntsRef to be used to iterate over the scratch buffer. A single
  /// instance is reused to avoid re-allocating the object. The ints and
  /// length fields need to be reset each use.
  ///
  /// The main reason for existing is to be able to call the
  /// [`IntersectVisitor#
  /// visit_with_ints_ref`](IntersectVisitor::visit_with_ints_ref)
  /// method rather than the
  /// [`IntersectVisitor#
  /// visit(int)`](IntersectVisitor::visit)
  /// method. This seems to make a difference in performance, probably due to
  /// fewer virtual calls then happening (once per read call rather than
  /// once per doc).
  scratch_ints_ref: IntsRef<Vec<i32>>,
  /// used to init a new scratch
  max_points_in_leaf: usize,
}
impl Clone for DocIdsWriter {
  fn clone(&self) -> Self {
    let max_points_in_leaf = self.scratch.len();
    DocIdsWriter::new(max_points_in_leaf)
  }
}

impl DocIdsWriter {
  pub const CONTINUOUS_IDS: i8 = -2;
  pub const BITSET_IDS: i8 = -1;
  pub const DELTA_BPV_16: i8 = 16;
  pub const BPV_24: i8 = 24;
  pub const BPV_32: i8 = 32;
  // These signs are legacy, should no longer be used in the writing side.
  pub const LEGACY_DELTA_VINT: i8 = 0;

  pub fn new(max_points_in_leaf: usize) -> Self {
    let mut scratch_ints_ref = IntsRef::default();
    {
      // This is here to not rely on the default constructor of IntsRef to
      // set1 offset to 0
      scratch_ints_ref.offset = 0;
    }
    Self {
      scratch: vec![0; max_points_in_leaf],
      scratch_longs: LongsRef::new(),
      scratch_ints_ref,
      max_points_in_leaf,
    }
  }
  pub(crate) fn write_doc_ids(
    &self,
    doc_ids: &[i32],
    start: usize,
    count: usize,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    // docs can be sorted either when all docs in a block have the same
    // value or when a segment is sorted
    let mut strictly_sorted = true;
    let mut min = doc_ids[0];
    let mut max = doc_ids[0];
    for i in 1..count {
      let last = doc_ids[start + i - 1];
      let current = doc_ids[start + i];
      if last >= current {
        strictly_sorted = false;
      }
      min = min.min(current);
      max = max.max(current);
    }

    let min2max: usize = (max - min + 1).try_convert()?;
    if strictly_sorted {
      if min2max == count {
        // continuous ids, typically happens when segment is sorted
        out.write_byte(DocIdsWriter::CONTINUOUS_IDS as u8)?;
        out.write_vint(doc_ids[start])?;
        return Ok(());
      } else if min2max <= ((count) << 4) {
        debug_assert!(min2max > count, "min2max: {min2max}, count: {count}");
        // Only trigger bitset optimization when max - min + 1 <= 16 *
        // count in order to avoid expanding too much
        // storage. A field with lower cardinality will
        // have higher probability to trigger this optimization.
        out.write_byte(DocIdsWriter::BITSET_IDS as u8)?;
        Self::write_ids_as_bit_set(doc_ids, start, count, out)?;
        return Ok(());
      }
    }
    if min2max <= 0xffff {
      out.write_byte(DocIdsWriter::DELTA_BPV_16 as u8)?;
      let mut scratch = vec![0; count];
      for i in 0..count {
        scratch[i] = doc_ids[start + i] - min;
      }
      out.write_vint(min)?;
      let half_len = count >> 1;
      for i in 0..half_len {
        scratch[i] = scratch[half_len + i] | (scratch[i] << 16);
      }
      for value in scratch.iter().take(half_len) {
        out.write_int(*value)?;
      }
      if count & 1 == 1 {
        out.write_short(scratch[count - 1] as i16)?;
      }
    } else if max <= 0xFFFFFF {
      out.write_byte(DocIdsWriter::BPV_24 as u8)?;
      // write them the same way we are reading them.
      let mut i = 0;
      while i + 7 < count {
        let doc1 = doc_ids[start + i];
        let doc2 = doc_ids[start + i + 1];
        let doc3 = doc_ids[start + i + 2];
        let doc4 = doc_ids[start + i + 3];
        let doc5 = doc_ids[start + i + 4];
        let doc6 = doc_ids[start + i + 5];
        let doc7 = doc_ids[start + i + 6];
        let doc8 = doc_ids[start + i + 7];
        let l1 = ((doc1 as i64 & 0xffffff) << 40)
          | ((doc2 as i64 & 0xffffff) << 16)
          | (((doc3 as u32) >> 8) as i64 & 0xffff);
        let l2 = ((doc3 as i64 & 0xff) << 56)
          | ((doc4 as i64 & 0xffffff) << 32)
          | ((doc5 as i64 & 0xffffff) << 8)
          | (((doc6 as u32) >> 16) as i64 & 0xff);
        let l3 = ((doc6 as i64 & 0xffff) << 48)
          | ((doc7 as i64 & 0xffffff) << 24)
          | (doc8 as i64 & 0xffffff);
        out.write_long(l1)?;
        out.write_long(l2)?;
        out.write_long(l3)?;
        i += 8;
      }

      while i < count {
        out.write_short(((doc_ids[start + i] as u32) >> 8) as i16)?;
        out.write_byte(doc_ids[start + i] as u8)?;
        i += 1;
      }
    } else {
      out.write_byte(DocIdsWriter::BPV_32 as u8)?;
      for i in 0..count {
        out.write_int(doc_ids[start + i])?;
      }
    }
    Ok(())
  }
  fn write_ids_as_bit_set(
    doc_ids: &[i32],
    start: usize,
    count: usize,
    out: &mut impl DataOutput,
  ) -> Result<()> {
    let min = doc_ids[start];
    let max = doc_ids[start + count - 1];

    let offset_words = min >> 6;
    let offset_bits = offset_words << 6;
    let total_word_count = FixedBitSet::bits2words((max - offset_bits + 1).try_convert()?);
    let mut current_word: i64 = 0;
    let mut current_word_index = 0;

    out.write_vint(offset_words)?;
    out.write_vint(total_word_count.try_convert()?)?;
    // build bit set streaming
    for i in 0..count {
      let index = doc_ids[start + i] - offset_bits;
      let next_word_index = index >> 6;
      debug_assert!(
        current_word_index <= next_word_index,
        "current_word_index: {current_word_index}, next_word_index: {next_word_index}"
      );
      if current_word_index < next_word_index {
        out.write_long(current_word)?;
        current_word = 0;
        current_word_index += 1;
        while current_word_index < next_word_index {
          current_word_index += 1;
          out.write_long(0)?;
        }
      }
      current_word |= 1i64 << (index as u32);
    }
    out.write_long(current_word)?;
    debug_assert!(
      current_word_index + 1 == total_word_count as i32,
      "current_word_index + 1: {}, total_word_count: {}",
      current_word_index + 1,
      total_word_count
    );
    Ok(())
  }

  /// Read `count` integers into `doc_ids`.
  pub(crate) fn read_ints(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
    doc_ids: &mut [i32],
  ) -> Result<()> {
    let bpv = input.read_byte()? as i8;
    match bpv {
      DocIdsWriter::CONTINUOUS_IDS => Self::read_continuous_ids(input, count, doc_ids),
      DocIdsWriter::BITSET_IDS => self.read_bit_set(input, count, doc_ids),
      DocIdsWriter::DELTA_BPV_16 => Self::read_delta16(input, count, doc_ids),
      DocIdsWriter::BPV_24 => Self::read_ints24(input, count, doc_ids),
      DocIdsWriter::BPV_32 => Self::read_ints32(input, count, doc_ids),
      DocIdsWriter::LEGACY_DELTA_VINT => Self::read_legacy_delta_vints(input, count, doc_ids),
      _ => Err(LuceneError::illegal_state(format!(
        "Unsupported number of bits per value: {bpv}"
      ))),
    }
  }
  fn read_bit_set_iterator(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
  ) -> Result<impl DocIdSetIterator> {
    let offset_words = input.read_vint()?;
    let long_len = input.read_vint()?.try_convert()?;
    if let Some(new_array) = ArrayUtil::grow_no_copy(&self.scratch_longs.longs, long_len) {
      self.scratch_longs.longs = new_array
    }
    input.read_longs(&mut self.scratch_longs.longs, 0, long_len)?;
    // make ghost bits clear for FixedBitSet.
    if (long_len) < self.scratch_longs.length {
      self.scratch_longs.longs[long_len..].fill(0);
    }
    self.scratch_longs.length = long_len;
    let bit_set =
      FixedBitSet::with_capacity(std::mem::take(&mut self.scratch_longs.longs), long_len << 6)?;
    DocBaseBitSetIterator::new(bit_set, count as i64, (offset_words << 6).try_convert()?)
  }

  fn read_continuous_ids(
    input: &mut impl IndexInput,
    count: usize,
    doc_ids: &mut [i32],
  ) -> Result<()> {
    let start = input.read_vint()?;
    for (i, doc_id) in doc_ids.iter_mut().take(count).enumerate() {
      *doc_id = start + i as i32;
    }
    Ok(())
  }

  fn read_legacy_delta_vints(
    input: &mut impl IndexInput,
    count: usize,
    doc_ids: &mut [i32],
  ) -> Result<()> {
    let mut doc = 0;
    for doc_id in doc_ids.iter_mut().take(count) {
      doc += input.read_vint()?;
      *doc_id = doc;
    }
    Ok(())
  }

  fn read_bit_set(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
    doc_ids: &mut [i32],
  ) -> Result<()> {
    let mut iterator = self.read_bit_set_iterator(input, count)?;
    let mut pos = 0;
    let mut doc_id;
    while {
      doc_id = iterator.next_doc()?;
      doc_id != NO_MORE_DOCS
    } {
      doc_ids[pos] = doc_id;
      pos += 1;
    }
    debug_assert!(pos == count, "pos: {pos}, count: {count}");
    Ok(())
  }

  fn read_delta16(input: &mut impl IndexInput, count: usize, doc_ids: &mut [i32]) -> Result<()> {
    let min = input.read_vint()?;
    let half_len = count >> 1;
    input.read_ints(doc_ids, 0, half_len)?;
    for i in 0..half_len {
      let l = doc_ids[i];
      doc_ids[i] = ((l as u32) >> 16) as i32 + min;
      doc_ids[half_len + i] = (l & 0xffff) + min;
    }
    if count & 1 == 1 {
      doc_ids[count - 1] = (input.read_short()? as u16 as i32) + min;
    }
    Ok(())
  }

  fn read_ints24(input: &mut impl IndexInput, count: usize, doc_ids: &mut [i32]) -> Result<()> {
    let mut i = 0;
    let count_usize = count;
    while i < count_usize.saturating_sub(7) {
      let l1 = input.read_long()? as u64;
      let l2 = input.read_long()? as u64;
      let l3 = input.read_long()? as u64;
      doc_ids[i] = (l1 >> 40) as i32;
      doc_ids[i + 1] = ((l1 >> 16) & 0xffffff) as i32;
      doc_ids[i + 2] = (((l1 & 0xffff) << 8) | ((l2 >> 56) & 0xff)) as i32;
      doc_ids[i + 3] = ((l2 >> 32) & 0xffffff) as i32;
      doc_ids[i + 4] = ((l2 >> 8) & 0xffffff) as i32;
      doc_ids[i + 5] = (((l2 & 0xff) << 16) | ((l3 >> 48) & 0xffff)) as i32;
      doc_ids[i + 6] = ((l3 >> 24) & 0xffffff) as i32;
      doc_ids[i + 7] = (l3 & 0xffffff) as i32;
      i += 8;
    }
    while i < count_usize {
      doc_ids[i] =
        ((input.read_short()? as u16 as i32) << 8) | ((input.read_byte()? as i32) & 0xff);
      i += 1;
    }
    Ok(())
  }

  fn read_ints32(input: &mut impl IndexInput, count: usize, doc_ids: &mut [i32]) -> Result<()> {
    input.read_ints(doc_ids, 0, count)?;
    Ok(())
  }
  pub(crate) fn read_ints_with_visitor(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    let bpv = input.read_byte()? as i8;
    match bpv {
      DocIdsWriter::CONTINUOUS_IDS => Self::read_continuous_ids_with_visitor(input, count, visitor),
      DocIdsWriter::BITSET_IDS => self.read_bit_set_with_visitor(input, count, visitor),
      DocIdsWriter::DELTA_BPV_16 => self.read_delta16_with_visitor(input, count, visitor),
      DocIdsWriter::BPV_24 => Self::read_ints24_with_visitor(input, count, visitor),
      DocIdsWriter::BPV_32 => self.read_ints32_with_visitor(input, count, visitor),
      DocIdsWriter::LEGACY_DELTA_VINT => {
        Self::read_legacy_delta_vints_with_visitor(input, count, visitor)
      },
      _ => Err(LuceneError::illegal_state(format!(
        "Unsupported number of bits per value: {bpv}"
      ))),
    }
  }

  fn read_bit_set_with_visitor(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    let mut bit_set_iterator = self.read_bit_set_iterator(input, count)?;
    visitor.visit_with_iterator(&mut bit_set_iterator)?;
    Ok(())
  }
  fn read_continuous_ids_with_visitor(
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    let start: usize = input.read_vint()?.try_convert()?;
    let extra = start & 63;
    let offset = start - extra;
    let num_bits = count + extra;
    let mut bit_set = FixedBitSet::new(num_bits);
    bit_set.set_with_range(extra, num_bits);
    let mut disi = DocBaseBitSetIterator::new(bit_set, count as i64, offset)?;
    visitor.visit_with_iterator(&mut disi)?;
    Ok(())
  }
  fn read_legacy_delta_vints_with_visitor(
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    let mut doc = 0;
    for _ in 0..count {
      doc += input.read_vint()?;
      visitor.visit(doc)?;
    }
    Ok(())
  }
  fn read_delta16_with_visitor(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    Self::read_delta16(input, count, &mut self.scratch)?;
    self.scratch_ints_ref.ints =
      CoreHelper::take_and_reset(&mut self.scratch, |_| vec![0; self.max_points_in_leaf]);

    self.scratch_ints_ref.length = count;
    visitor.visit_with_ints_ref(&self.scratch_ints_ref)?;
    Ok(())
  }
  fn read_ints24_with_visitor(
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    let mut i = 0;
    let count_usize = count;
    while i < count_usize.saturating_sub(7) {
      let l1 = input.read_long()? as u64;
      let l2 = input.read_long()? as u64;
      let l3 = input.read_long()? as u64;
      visitor.visit((l1 >> 40) as i32)?;
      visitor.visit(((l1 >> 16) & 0xffffff) as i32)?;
      visitor.visit((((l1 & 0xffff) << 8) | ((l2 >> 56) & 0xff)) as i32)?;
      visitor.visit(((l2 >> 32) & 0xffffff) as i32)?;
      visitor.visit(((l2 >> 8) & 0xffffff) as i32)?;
      visitor.visit((((l2 & 0xff) << 16) | ((l3 >> 48) & 0xffff)) as i32)?;
      visitor.visit(((l3 >> 24) & 0xffffff) as i32)?;
      visitor.visit((l3 & 0xffffff) as i32)?;
      i += 8;
    }
    while i < count_usize {
      let s = input.read_short()? as u16 as i32;
      let b = input.read_byte()? as i32;
      visitor.visit((s << 8) | b)?;
      i += 1;
    }
    Ok(())
  }
  fn read_ints32_with_visitor(
    &mut self,
    input: &mut impl IndexInput,
    count: usize,
    visitor: &mut impl IntersectVisitor,
  ) -> Result<()> {
    input.read_ints(&mut self.scratch, 0, count)?;
    self.scratch_ints_ref.ints =
      CoreHelper::take_and_reset(&mut self.scratch, |old| vec![0; old.len()]);

    self.scratch_ints_ref.length = count;
    visitor.visit_with_ints_ref(&self.scratch_ints_ref)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::int_point::IntPoint;
  use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::index_writer_config::IndexWriterConfig;
  use crate::core::index::point_values::{IntersectVisitor, Relation};
  use crate::core::store::directory::Directory;
  use crate::core::store::{DataOutput, IOContext, IndexInput, IndexOutput};
  use crate::core::util::bkd::doc_ids_writer::DocIdsWriter;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory, new_directory_shared, random,
  };
  use crate::test::core::util::test_util::TestUtil;
  use rand::Rng;
  use rand::RngExt;
  use std::collections::HashSet;

  #[allow(dead_code)] // for quick search
  struct TestDocIdsWriter;

  #[test]
  fn test_random() -> Result<()> {
    let mut random = random();
    let num_iters = at_least(&mut random, 100);
    let dir = new_directory(&mut random)?;
    for _ in 0..num_iters {
      let len = 1 + random.random_range(0..5000);
      let mut doc_ids = vec![0; len];
      let bpv = TestUtil::next_int(&mut random, 1, 32);
      for doc_id in doc_ids.iter_mut().take(len) {
        *doc_id = TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
      }
      test(&mut random, &dir, &doc_ids)?;
    }
    Ok(())
  }

  #[test]
  fn test_sorted() -> Result<()> {
    let mut random = random();
    let num_iters = at_least(&mut random, 100);
    let dir = new_directory(&mut random)?;
    for _ in 0..num_iters {
      let len = 1 + random.random_range(0..5000);
      let mut doc_ids = vec![0; len];
      let bpv = TestUtil::next_int(&mut random, 1, 32);
      for doc_id in doc_ids.iter_mut().take(len) {
        *doc_id = TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
      }
      doc_ids.sort_unstable();
      test(&mut random, &dir, &doc_ids)?;
    }
    Ok(())
  }

  #[test]
  fn test_cluster() -> Result<()> {
    let mut random = random();
    let num_iters = at_least(&mut random, 100);
    let dir = new_directory(&mut random)?;
    for _ in 0..num_iters {
      let len = 1 + random.random_range(0..5000);
      let mut doc_ids = vec![0; len];
      let min = random.random_range(0..1000);
      let bpv = TestUtil::next_int(&mut random, 1, 16);
      for doc_id in doc_ids.iter_mut().take(len) {
        *doc_id = min + TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
      }
      test(&mut random, &dir, &doc_ids)?;
    }
    Ok(())
  }

  #[test]
  fn test_bit_set() -> Result<()> {
    let mut random = random();
    let num_iters = at_least(&mut random, 100);
    let dir = new_directory(&mut random)?;
    for _ in 0..num_iters {
      let size = 1 + random.random_range(0..5000);
      let mut set = HashSet::with_capacity(size);
      let small = random.random_range(0..1000);
      while set.len() < size {
        set.insert(small + random.random_range(0..(size * 16)) as i32);
      }
      let mut doc_ids: Vec<i32> = set.into_iter().collect();
      doc_ids.sort_unstable();
      test(&mut random, &dir, &doc_ids)?;
    }
    Ok(())
  }
  #[test]
  fn test_continuous_ids() -> Result<()> {
    let mut random = random();
    let num_iters = at_least(&mut random, 100);
    let dir = new_directory(&mut random)?;
    for _ in 0..num_iters {
      let size = 1 + random.random_range(0..5000);
      let mut doc_ids = vec![0; size];
      let start = random.random_range(0..1000000);
      for (i, doc_id) in doc_ids.iter_mut().take(size).enumerate() {
        *doc_id = start + i as i32;
      }
      test(&mut random, &dir, &doc_ids)?;
    }
    Ok(())
  }

  fn test<R: Rng + ?Sized>(random: &mut R, dir: &impl Directory, ints: &[i32]) -> Result<()> {
    let len;
    let mut doc_ids_writer = DocIdsWriter::new(ints.len());
    {
      let mut out = dir.create_output("tmp", &IOContext::default_io_context()?)?;
      doc_ids_writer.write_doc_ids(ints, 0, ints.len(), &mut out)?;
      len = out.get_file_pointer();
      if random.random_bool(0.5) {
        out.write_long(0)?;
      }
    }
    {
      let mut input = dir.open_input("tmp", &IOContext::read_once_io_context()?)?;
      let mut read = vec![0; ints.len()];
      doc_ids_writer.read_ints(&mut input, ints.len(), &mut read)?;
      assert_eq!(ints, &read[..]);
      assert_eq!(len, input.get_file_pointer()?);
    }
    {
      let mut input = dir.open_input("tmp", &IOContext::read_once_io_context()?)?;
      let mut read = vec![0; ints.len()];
      let mut visitor = IntersectVisitorMock {
        i: 0,
        read: &mut read,
      };
      doc_ids_writer.read_ints_with_visitor(&mut input, ints.len(), &mut visitor)?;
      assert_eq!(ints, &read[..]);
      assert_eq!(len, input.get_file_pointer()?);
    }
    dir.delete_file("tmp")?;
    Ok(())
  }

  #[test]
  #[ignore]
  fn test_crash() -> Result<()> {
    let mut random = random();
    let itrs = at_least(&mut random, 100);

    for _ in 0..itrs {
      let dir = new_directory_shared(&mut random)?;

      let config = IndexWriterConfig::new();
      let iw = IndexWriter::new(dir.clone(), config)?;

      for _d in 0..20_000 {
        let mut doc = Document::new();
        doc.add(IntPoint::new("foo", [0])?);
        doc.add(SortedNumericDocValuesField::new("bar", 0));
        iw.add_document(doc)?;
      }
      iw.close()?;
    }

    Ok(())
  }

  struct IntersectVisitorMock<'a> {
    i: usize,
    read: &'a mut Vec<i32>,
  }
  impl IntersectVisitor for IntersectVisitorMock<'_> {
    fn visit(&mut self, doc_id: i32) -> Result<()> {
      self.read[self.i] = doc_id;
      self.i += 1;
      Ok(())
    }

    fn visit_with_packed_value(&mut self, _doc_id: i32, _packed_value: &[u8]) -> Result<()> {
      Err(LuceneError::unsupported_operation(""))
    }

    fn compare(&self, _min_packed_value: &[u8], _max_packed_value: &[u8]) -> Result<Relation> {
      Err(LuceneError::unsupported_operation(""))
    }
  }
}
