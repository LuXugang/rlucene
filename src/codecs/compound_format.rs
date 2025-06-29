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
use crate::codecs::compound_directory::CompoundDirectory;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;

/// Encodes/decodes compound files
pub trait CompoundFormat {
    /// Returns a read-only view of the compound files in this segment.
    fn get_compound_reader<D>(
        &self,
        dir: &mut D,
        si: &SegmentInfo<D>,
    ) -> Result<CompoundDirectory<D>>
    where
        D: Directory;

    /// Packs the provided segment's files into a compound format.
    ///
    /// All files referenced by the provided [`SegmentInfo`]
    /// must have their headers and footers
    /// written using
    /// [`CodecUtil::write_index_header`](crate::codecs::codec_util::CodecUtil::write_index_header)
    /// and [`CodecUtil::write_footer`](crate::codecs::codec_util::CodecUtil::write_footer).
    fn write<D: Directory>(
        &self,
        dir: &mut impl Directory,
        si: &SegmentInfo<D>,
        context: &IOContext,
    ) -> Result<()>;
}
pub struct SizedFileQueue;
