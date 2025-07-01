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
use crate::index::merge_state::MergeState;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use crate::util::info_stream::InfoStream;

/// Computes which segments have identical field name to number mappings,
/// which allows stored fields and term vectors in this codec to be bulk-merged.
pub struct MatchingReaders {
    /// [`SegmentReader`](crate::index::segment_reader::SegmentReader)'s that
    /// have identical field name/number mapping, so their stored fields
    /// and term vectors may be bulk merged.
    pub matching_readers: Vec<bool>,

    /// How many `matching_readers` are set.
    pub count: i32,
}

impl MatchingReaders {
    pub fn new<I>(merge_state: &MergeState<I>) -> Result<Self>
    where
        I: IndexInput,
    {
        // If the i'th reader is a SegmentReader and has
        // identical fieldName -> number mapping, then this
        // array will be non-null at position i:
        let num_readers = merge_state.max_docs.len();
        let mut matching_readers = vec![false; num_readers];
        let mut matched_count: i32 = 0;

        'next_reader: for (i, field_infos) in merge_state.field_infos.iter().enumerate() {
            for fi in &**field_infos {
                match merge_state
                    .merge_field_infos
                    .field_info_by_number(fi.number)?
                {
                    Some(other) if other.name == fi.name => continue,
                    _ => continue 'next_reader,
                }
            }
            matching_readers[i] = true;
            matched_count += 1;
        }
        let mut info_stream = merge_state.info_stream.lock();
        if info_stream.enabled("SM") {
            info_stream.message(
                "SM",
                &format!("merge store matched_count={matched_count} vs {num_readers}"),
            );
        }
        if matched_count as usize != num_readers {
            info_stream.message(
                "SM",
                &format!("{} non-bulk merges", num_readers as i32 - matched_count),
            );
        }

        Ok(Self {
            matching_readers,
            count: matched_count,
        })
    }
}
