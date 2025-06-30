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
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;

/// Holder struct for common parameters used during read.
///
/// @lucene.experimental
pub struct SegmentReadState<D>
where
    D: Directory,
{
    /// Directory where this segment is read from.
    pub directory: Arc<Mutex<D>>,

    /// SegmentInfo describing this segment.
    pub segment_info: Rc<SegmentInfo<D>>,

    /// FieldInfos describing all fields in this segment.
    pub field_infos: Rc<FieldInfos>,

    /// IOContext to pass to Directory::open_input.
    pub context: Rc<IOContext>,

    /// Unique suffix for any postings files read for this segment.
    pub segment_suffix: String,
}

impl<D> SegmentReadState<D>
where
    D: Directory,
{
    /// Creates a SegmentReadState with an empty segment suffix.
    pub fn new(
        directory: Arc<Mutex<D>>,
        segment_info: Rc<SegmentInfo<D>>,
        field_infos: Rc<FieldInfos>,
        context: Rc<IOContext>,
    ) -> Self {
        Self::with_suffix(directory, segment_info, field_infos, context, "")
    }

    /// Creates a SegmentReadState with a custom segment suffix.
    pub fn with_suffix(
        directory: Arc<Mutex<D>>,
        segment_info: Rc<SegmentInfo<D>>,
        field_infos: Rc<FieldInfos>,
        context: Rc<IOContext>,
        segment_suffix: &str,
    ) -> Self {
        Self {
            directory,
            segment_info,
            field_infos,
            context,
            segment_suffix: segment_suffix.to_string(),
        }
    }

    /// Creates a copy of an existing SegmentReadState with a different segment
    /// suffix.
    pub fn copy_with_suffix(other: &SegmentReadState<D>, segment_suffix: &str) -> Self {
        Self {
            directory: Arc::clone(&other.directory),
            segment_info: other.segment_info.clone(),
            field_infos: other.field_infos.clone(),
            context: other.context.clone(),
            segment_suffix: segment_suffix.to_string(),
        }
    }
}
