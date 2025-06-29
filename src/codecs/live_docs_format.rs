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
use std::collections::HashSet;

use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::Result;

/// Format for live/deleted documents
pub trait LiveDocsFormat {
    /// Reads live docs bits from the specified directory.
    ///
    /// # Arguments
    /// - `dir`: The directory to read from.
    /// - `info`: The segment commit info for the segment.
    /// - `Context`: The IO context.
    ///
    /// # Returns
    /// A `Bits` implementation representing the live docs.
    fn read_live_docs<D>(
        &self,
        dir: &mut impl Directory,
        info: &SegmentCommitInfo<D>,
        context: &IOContext,
    ) -> Result<impl Bits>
    where
        D: Directory;

    /// Persist live docs bits. Use
    /// [`SegmentCommitInfo#
    /// getNextDelGen`](SegmentCommitInfo::get_next_write_del_gen) to determine
    /// the generation of the deletes file you should write to.
    fn write_live_docs<D>(
        &self,
        bits: &impl Bits,
        dir: &mut impl Directory,
        info: &SegmentCommitInfo<D>,
        new_del_count: i32,
        context: &IOContext,
    ) -> Result<()>
    where
        D: Directory;

    /// Records all files in use by this [`SegmentCommitInfo`] into the files
    /// argument.
    fn files<D>(&self, info: &SegmentCommitInfo<D>, files: &mut HashSet<String>) -> Result<()>
    where
        D: Directory;
}
