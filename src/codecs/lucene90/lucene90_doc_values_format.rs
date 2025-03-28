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
use once_cell::sync::Lazy;

pub struct Lucene90DocValuesFormat;
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

    /// Number of intervals represented as a shift to create a new level, this is 1 << 3 == 8 intervals.
    pub const SKIP_INDEX_LEVEL_SHIFT: i32 = 3;

    /// Max number of levels
    /// Increasing this number increases how much heap we need at index time.
    /// We currently need (1 * 8 * 8 * 8) = 512 accumulators on heap
    pub const SKIP_INDEX_MAX_LEVEL: usize = 4;
}
/// Number of bytes to skip when skipping a level. It does not take into account the
/// current interval that is being read.
pub static SKIP_INDEX_JUMP_LENGTH_PER_LEVEL: Lazy<
    [i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
> = Lazy::new(|| {
    let mut arr = [0i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL];
    // Size of the interval minus read bytes (1 byte for level and 4 bytes for maxDocID)
    arr[0] = Lucene90DocValuesFormat::SKIP_INDEX_INTERVAL_BYTES - 5;
    for level in 1..Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL {
        // Jump from previous level
        arr[level] = arr[level - 1];
        // Nodes added by new level
        arr[level] += (1 << (level as i32 * Lucene90DocValuesFormat::SKIP_INDEX_LEVEL_SHIFT))
            as i64
            * Lucene90DocValuesFormat::SKIP_INDEX_INTERVAL_BYTES;
        // Remove the byte levels added in the previous level
        arr[level] -=
            (1 << ((level as i32 - 1) * Lucene90DocValuesFormat::SKIP_INDEX_LEVEL_SHIFT)) as i64;
    }
    arr
});
