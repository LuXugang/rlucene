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
use std::fmt::{Display, Formatter};

use std::sync::LazyLock;

use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::codecs::lucene90_doc_values_consumer::Lucene90DocValuesConsumer;
use crate::core::codecs::lucene90_doc_values_producer::Lucene90DocValuesProducer;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Lucene 9.0 DocValues format.
///
/// Documents that have a value for the field are encoded in a way that it is
/// always possible to know the ordinal of the current document in the set of
/// documents that have a value. For instance, say the set of documents that
/// have a value for the field is <code>{1, 5, 6, 11}</code>. When the
/// iterator is on <code>6</code>, it knows that this is the 3rd item of the
/// set. This way, values can be stored densely and accessed based on their
/// index at search time. If all documents in a segment have a value for the
/// field, the index is the same as the doc ID, so this case is encoded
/// implicitly and is very fast at query time. On the other hand if some
/// documents are missing a value for the field then the set of documents that
/// have a value is encoded into blocks. All doc IDs that share the same upper
/// 16 bits are encoded into the same block with the following strategies:
///
/// - SPARSE: This strategy is used when a block contains at most 4095
///   documents. The lower 16 bits of doc IDs are stored as
///   [`DataOutput::write_short`](crate::core::store::DataOutput::write_short) while
///   the upper 16 bits are given by the block ID.
/// - DENSE: This strategy is used when a block contains between 4096 and 65535
///   documents. The lower bits of doc IDs are stored in a bit set. Advancing <
///   512 documents is performed using `trailing_zeros` operations while the
///   index is computed by accumulating the `bitCount` of the visited longs.
///   Advancing \>= 512 documents is performed by skipping to the start of the
///   needed 512 document sub-block and iterating to the specific document
///   within that block. The index for the sub-block that is skipped to is
///   retrieved from a rank-table positioned before the bit set. The rank-table
///   holds the origo index numbers for all 512 documents sub-blocks,
///   represented as an unsigned short for each 128 blocks.
/// - ALL: This strategy is used when a block contains exactly 65536 documents, meaning that the
///   block is full. In that case doc IDs do not need to be stored explicitly. This is typically
///   faster than both SPARSE and DENSE which is a reason why it is preferable to have all
///   documents that have a value for a field using contiguous doc IDs, for instance by using
///   `set_index_sort` with a sort.
///
/// Skipping blocks to arrive at a wanted document is either done on an
/// iterative basis or by using the jump-table stored at the end of the chain of
/// blocks. The jump-table holds the offset as well as the index for all blocks,
/// packed in a single long per block.
///
/// Then the five per-document value types
/// (Numeric,Binary,Sorted,SortedSet,SortedNumeric) are encoded using the
/// following strategies:
///
/// [`DocValuesType::NUMERIC`](crate::core::index::doc_values_type::DocValuesType::Numeric):
///
/// - Delta-compressed: per-document integers written as deltas from the minimum
///   value, compressed with bitpacking. For more information, see
///   [`DirectWriter`](crate::core::util::packed::direct_writer::DirectWriter).
/// - Table-compressed: when the number of unique values is very small (< 256),
///   and when there are unused "gaps" in the range of values used (such as
///   [`SmallFloat`](crate::core::util::small_float::SmallFloat)), a lookup table is
///   written instead. Each per-document entry is instead the ordinal to this
///   table, and those ordinals are compressed with bitpacking
///   ([`DirectWriter`](crate::core::util::packed::direct_writer::DirectWriter)).
/// - GCD-compressed: when all numbers share a common divisor, such as dates,
///   the greatest common denominator (GCD) is computed, and quotients are
///   stored using Delta-compressed Numerics.
/// - Monotonic-compressed: when all numbers are monotonically increasing
///   offsets, they are written as blocks of bitpacked integers, encoding the
///   deviation from the expected delta.
/// - Const-compressed: when there is only one possible value, no per-document
///   data is needed and this value is encoded alone.
///
/// Depending on calculated gains, the numbers might be split into blocks of
/// 16384 values. In that case, a jump-table with block offsets is appended to
/// the blocks for O(1) access to the needed block.
///
/// [`DocValuesType::BINARY`](crate::core::index::doc_values_type::DocValuesType::Binary):
///
/// - Fixed-width Binary: one large concatenated `byte[]` is written, along with
///   the fixed length. Each document's value can be addressed directly with
///   multiplication (`docID * length`).
/// - Variable-width Binary: one large concatenated `byte[]` is written, along
///   with end addresses for each document. The addresses are written as
///   Monotonic-compressed numerics.
/// - Prefix-compressed Binary: values are written in chunks of 16, with the
///   first value written completely and other values sharing prefixes. Chunk
///   addresses are written as Monotonic-compressed numerics. A reverse lookup
///   index is written from a portion of every 1024th term.
///
/// [`DocValuesType::SORTED`](crate::core::index::doc_values_type::DocValuesType::Sorted):
///
/// - Sorted: a mapping of ordinals to deduplicated terms is written as
///   Prefix-compressed Binary, along with the per-document ordinals written
///   using one of the numeric strategies above.
///
/// [`DocValuesType::SORTED_SET`](crate::core::index::doc_values_type::DocValuesType::SortedSet):
///
/// - Single: if all documents have 0 or 1 value, then data are written like
///   SORTED.
/// - SortedSet: a mapping of ordinals to deduplicated terms is written as
///   Binary, an ordinal list and per-document index into this list are written
///   using the numeric strategies above.
///
/// [`DocValuesType::SORTED_NUMERIC`](crate::core::index::doc_values_type::DocValuesType::SortedNumeric):
///
/// - Single: if all documents have 0 or 1 value, then data are written like
///   NUMERIC.
/// - SortedNumeric: a value list and per-document index into this list are
///   written using the numeric strategies above.
///
/// # Files:
///
/// - `.dvd`: DocValues data
/// - `.dvm`: DocValues metadata
pub struct Lucene90DocValuesFormat {
  skip_index_interval_size: i32,
  name: String,
}
impl Lucene90DocValuesFormat {
  pub const DATA_CODEC: &'static str = "Lucene90DocValuesData";
  pub const DATA_EXTENSION: &'static str = "dvd";
  pub const META_CODEC: &'static str = "Lucene90DocValuesMetadata";
  pub const META_EXTENSION: &'static str = "dvm";

  pub const VERSION_START: i32 = 0;
  pub const VERSION_CURRENT: i32 = Self::VERSION_START;

  /// Indicates docvalues type
  pub const NUMERIC: u8 = 0;
  pub const BINARY: u8 = 1;
  pub const SORTED: u8 = 2;
  pub const SORTED_SET: u8 = 3;
  pub const SORTED_NUMERIC: u8 = 4;

  pub const DIRECT_MONOTONIC_BLOCK_SHIFT: i32 = 16;

  pub const NUMERIC_BLOCK_SHIFT: i32 = 14;
  pub const NUMERIC_BLOCK_SIZE: i32 = 1 << Self::NUMERIC_BLOCK_SHIFT;

  pub const TERMS_DICT_BLOCK_LZ4_SHIFT: i32 = 6;
  pub const TERMS_DICT_BLOCK_LZ4_SIZE: i32 = 1 << Self::TERMS_DICT_BLOCK_LZ4_SHIFT;
  pub const TERMS_DICT_BLOCK_LZ4_MASK: i32 = Self::TERMS_DICT_BLOCK_LZ4_SIZE - 1;

  pub const TERMS_DICT_REVERSE_INDEX_SHIFT: i32 = 10;
  pub const TERMS_DICT_REVERSE_INDEX_SIZE: i32 = 1 << Self::TERMS_DICT_REVERSE_INDEX_SHIFT;
  pub const TERMS_DICT_REVERSE_INDEX_MASK: i32 = Self::TERMS_DICT_REVERSE_INDEX_SIZE - 1;

  /// Number of documents in an interval
  pub const DEFAULT_SKIP_INDEX_INTERVAL_SIZE: i32 = 4096;

  /// Bytes on an interval:
  ///   * 1 byte : number of levels
  ///   * 16 bytes: min / max value,
  ///   * 8 bytes:  min / max docID
  ///   * 4 bytes: number of documents
  pub const SKIP_INDEX_INTERVAL_BYTES: i64 = 29;

  /// Number of intervals represented as a shift to create a new level, this
  /// is 1 << 3 == 8 intervals.
  pub const SKIP_INDEX_LEVEL_SHIFT: i32 = 3;

  /// Max number of levels
  /// Increasing this number increases how much heap we need at index time.
  /// We currently need (1 * 8 * 8 * 8) = 512 accumulators on heap
  pub const SKIP_INDEX_MAX_LEVEL: usize = 4;

  pub fn new() -> Result<Self> {
    Self::with_skip_index_interval_size(Self::DEFAULT_SKIP_INDEX_INTERVAL_SIZE)
  }
  pub fn with_skip_index_interval_size(skip_index_interval_size: i32) -> Result<Self> {
    if skip_index_interval_size < 2 {
      return Err(LuceneError::illegal_argument(format!(
        "skip_index_interval_size must be > 1, got [{skip_index_interval_size}]"
      )));
    }
    Ok(Self {
      skip_index_interval_size,
      name: "Lucene90".to_string(),
    })
  }
}
impl Default for Lucene90DocValuesFormat {
  fn default() -> Self {
    let result = Self::new();
    debug_assert!(result.is_ok());
    Self::new().unwrap()
  }
}

impl Display for Lucene90DocValuesFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "DocValuesFormat(name= {} )",
      self.skip_index_interval_size
    )
  }
}

impl DocValuesFormat for Lucene90DocValuesFormat {
  type DocValuesConsumer<T: IndexOutput> = Lucene90DocValuesConsumer<T>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesConsumer<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    Lucene90DocValuesConsumer::new(
      state,
      self.skip_index_interval_size,
      Self::DATA_CODEC,
      Self::DATA_EXTENSION,
      Self::META_CODEC,
      Self::META_EXTENSION,
      segment_info,
    )
  }

  type DocValuesProducer<T: IndexInput> = Lucene90DocValuesProducer<T>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesProducer<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    Lucene90DocValuesProducer::new(
      state,
      Self::DATA_CODEC,
      Self::DATA_EXTENSION,
      Self::META_CODEC,
      Self::META_EXTENSION,
      segment_info,
    )
  }
}
/// Number of bytes to skip when skipping a level. It does not take into account
/// the current interval that is being read.
pub static SKIP_INDEX_JUMP_LENGTH_PER_LEVEL: LazyLock<
  [i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
> = LazyLock::new(|| {
  let mut arr = [0i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL];
  // Size of the interval minus read bytes (1 byte for level and 4 bytes for
  // maxDocID)
  arr[0] = Lucene90DocValuesFormat::SKIP_INDEX_INTERVAL_BYTES - 5;
  for level in 1..Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL {
    // Jump from previous level
    arr[level] = arr[level - 1];
    // Nodes added by new level
    arr[level] += (1 << (level as i32 * Lucene90DocValuesFormat::SKIP_INDEX_LEVEL_SHIFT)) as i64
      * Lucene90DocValuesFormat::SKIP_INDEX_INTERVAL_BYTES;
    // Remove the byte levels added in the previous level
    arr[level] -=
      (1 << ((level as i32 - 1) * Lucene90DocValuesFormat::SKIP_INDEX_LEVEL_SHIFT)) as i64;
  }
  arr
});
