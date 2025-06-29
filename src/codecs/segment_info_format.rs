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
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use crate::util::StringHelper;

/// Expert: Controls the format of the [`SegmentInfo`] (segment metadata file).
///
/// # See Also
/// - [`SegmentInfo`]
///
/// # Note
/// This is considered experimental and may change in future versions.
pub trait SegmentInfoFormat {
    /// Read `SegmentInfo` data from a directory.
    ///
    /// # Arguments
    ///
    /// * `directory` - The directory to read from.
    /// * `segment_name` - The name of the segment to read.
    /// * `segment_id` - The expected identifier for the segment.
    /// * `context` - The IO context.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs.
    fn read<D>(
        &self,
        directory: Arc<Mutex<D>>,
        segment_name: &str,
        segment_id: &[u8; StringHelper::ID_LENGTH],
        context: &IOContext,
    ) -> Result<SegmentInfo<D>>
    where
        D: Directory;

    /// Write [`SegmentInfo`] data.
    ///
    /// The codec must add its SegmentInfo filename(s) to `info` before doing
    /// I/O.
    fn write<D>(
        &self,
        directory: &mut impl Directory,
        info: &mut SegmentInfo<D>,
        context: &IOContext,
    ) -> Result<()>
    where
        D: Directory;
}
