/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::codecs::lucene90::block_tree::compression_algorithm::CompressionAlgorithm;

pub struct Stats {
    /// Byte size of the index.
    pub index_num_bytes: i64,

    /// Total number of terms in the field.
    pub total_term_count: i64,

    /// Total number of bytes (sum of term lengths) across all terms in the
    /// field.
    pub total_term_bytes: i64,

    /// The number of normal (non-floor) blocks in the terms file.
    pub non_floor_block_count: i32,

    /// The number of floor blocks (meta-blocks larger than the allowed
    /// `maxItemsPerBlock`) in the terms file.
    pub floor_block_count: i32,

    /// The number of sub-blocks within the floor blocks.
    pub floor_sub_block_count: i32,

    /// The number of "internal" blocks (that have both terms and sub-blocks).
    pub mixed_block_count: i32,

    /// The number of "leaf" blocks (blocks that have only terms).
    pub terms_only_block_count: i32,

    /// The number of "internal" blocks that do not contain terms (have only
    /// sub-blocks).
    pub sub_blocks_only_block_count: i32,

    /// Total number of blocks.
    pub total_block_count: i32,

    /// Number of blocks at each prefix depth.
    pub block_count_by_prefix_len: Vec<i32>,

    start_block_count: i32,
    end_block_count: i32,

    /// Total number of bytes used to store term suffixes.
    pub total_block_suffix_bytes: i64,

    /// Number of times each compression method has been used.
    /// 0 = uncompressed, 1 = lowercase_ascii, 2 = LZ4
    pub compression_algorithms: [i64; 3],

    /// Total number of suffix bytes before compression.
    pub total_uncompressed_block_suffix_bytes: i64,

    /// Total number of bytes used to store term stats (not including
    /// [`PostingsReaderBase`](crate::codecs::postings_reader_base::PostingsReaderBase)).
    pub total_block_stats_bytes: i64,

    /// Total bytes stored by
    /// [`PostingsReaderBase`](crate::codecs::postings_reader_base::PostingsReaderBase)
    /// and frame metadata.
    pub total_block_other_bytes: i64,

    /// Segment name.
    pub segment: String,

    /// Field name.
    pub field: String,
}
impl Stats {
    pub fn new(segment: String, field: String) -> Self {
        Self {
            index_num_bytes: 0,
            total_term_count: 0,
            total_term_bytes: 0,
            non_floor_block_count: 0,
            floor_block_count: 0,
            floor_sub_block_count: 0,
            mixed_block_count: 0,
            terms_only_block_count: 0,
            sub_blocks_only_block_count: 0,
            total_block_count: 0,
            block_count_by_prefix_len: vec![0; 10],
            start_block_count: 0,
            end_block_count: 0,
            total_block_suffix_bytes: 0,
            compression_algorithms: [0; 3],
            total_uncompressed_block_suffix_bytes: 0,
            total_block_stats_bytes: 0,
            total_block_other_bytes: 0,
            segment,
            field,
        }
    }
    pub(crate) fn finish(&self) {
        debug_assert_eq!(
            self.start_block_count, self.end_block_count,
            "startBlockCount={} endBlockCount={}",
            self.start_block_count, self.end_block_count
        );

        debug_assert_eq!(
            self.total_block_count,
            self.floor_sub_block_count + self.non_floor_block_count,
            "floorSubBlockCount={} nonFloorBlockCount={} totalBlockCount={}",
            self.floor_sub_block_count,
            self.non_floor_block_count,
            self.total_block_count
        );

        debug_assert_eq!(
            self.total_block_count,
            self.mixed_block_count + self.terms_only_block_count + self.sub_blocks_only_block_count,
            "totalBlockCount={} mixedBlockCount={} subBlocksOnlyBlockCount={} termsOnlyBlockCount={}",
            self.total_block_count,
            self.mixed_block_count,
            self.sub_blocks_only_block_count,
            self.terms_only_block_count
        );
    }
}
impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "  index FST:")?;
        writeln!(f, "    {} bytes", self.index_num_bytes)?;

        writeln!(f, "  terms:")?;
        writeln!(f, "    {} terms", self.total_term_count)?;
        if self.total_term_count != 0 {
            writeln!(
                f,
                "    {} bytes ({:.1} bytes/term)",
                self.total_term_bytes,
                self.total_term_bytes as f64 / self.total_term_count as f64
            )?;
        } else {
            writeln!(f, "    {} bytes", self.total_term_bytes)?;
        }

        writeln!(f, "  blocks:")?;
        writeln!(f, "    {} blocks", self.total_block_count)?;
        writeln!(f, "    {} terms-only blocks", self.terms_only_block_count)?;
        writeln!(
            f,
            "    {} sub-block-only blocks",
            self.sub_blocks_only_block_count
        )?;
        writeln!(f, "    {} mixed blocks", self.mixed_block_count)?;
        writeln!(f, "    {} floor blocks", self.floor_block_count)?;
        writeln!(
            f,
            "    {} non-floor blocks",
            self.total_block_count - self.floor_sub_block_count
        )?;
        writeln!(f, "    {} floor sub-blocks", self.floor_sub_block_count)?;

        if self.total_block_count != 0 {
            writeln!(
                f,
                "    {} term suffix bytes before compression ({:.1} suffix-bytes/block)",
                self.total_uncompressed_block_suffix_bytes,
                self.total_block_suffix_bytes as f64 / self.total_block_count as f64
            )?;
        } else {
            writeln!(
                f,
                "    {} term suffix bytes before compression",
                self.total_uncompressed_block_suffix_bytes
            )?;
        }

        // Compression algorithm usage summary
        let mut compression_summary = Vec::new();
        for (code, &count) in self.compression_algorithms.iter().enumerate() {
            if count > 0 {
                let v = CompressionAlgorithm::by_code(code as u8)
                    .expect("Invalid compression algorithm code");
                compression_summary.push(format!("{v:?}: {count}"));
            }
        }
        let compression_ratio = if self.total_uncompressed_block_suffix_bytes > 0 {
            self.total_block_suffix_bytes as f64 / self.total_uncompressed_block_suffix_bytes as f64
        } else {
            0.0
        };
        writeln!(
            f,
            "    {} compressed term suffix bytes ({:.2} compression ratio - compression count by algorithm: {})",
            self.total_block_suffix_bytes,
            compression_ratio,
            compression_summary.join(", ")
        )?;

        // Term stats bytes
        if self.total_block_count != 0 {
            writeln!(
                f,
                "    {} term stats bytes ({:.1} stats-bytes/block)",
                self.total_block_stats_bytes,
                self.total_block_stats_bytes as f64 / self.total_block_count as f64
            )?;
        } else {
            writeln!(f, "    {} term stats bytes", self.total_block_stats_bytes)?;
        }

        // Other bytes
        if self.total_block_count != 0 {
            writeln!(
                f,
                "    {} other bytes ({:.1} other-bytes/block)",
                self.total_block_other_bytes,
                self.total_block_other_bytes as f64 / self.total_block_count as f64
            )?;
        } else {
            writeln!(f, "    {} other bytes", self.total_block_other_bytes)?;
        }

        // Block count by prefix length
        if self.total_block_count != 0 {
            writeln!(f, "    by prefix length:")?;
            let mut total = 0;
            for (prefix, &count) in self.block_count_by_prefix_len.iter().enumerate() {
                total += count;
                if count > 0 {
                    writeln!(f, "      {prefix:2}: {count}")?;
                }
            }
            debug_assert_eq!(self.total_block_count, total);
        }

        Ok(())
    }
}
