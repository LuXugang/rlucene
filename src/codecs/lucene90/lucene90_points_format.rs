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
use crate::codecs::lucene90_points_reader::Lucene90PointsReader;
use crate::codecs::lucene90_points_writer::Lucene90PointWriter;
use crate::codecs::points_format::PointsFormat;
use crate::codecs::points_reader::PointsReaderEnum;
use crate::codecs::points_writer::PointsWriterEnum;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
/// Lucene 9.0 point format, which encodes dimensional values in a block KD-tree structure for fast
/// 1D range and N-dimensional shape intersection filtering. See the [BKD paper] for details.
///
/// Data is stored across three files:
/// - **`.kdm`**: records metadata about the fields (e.g., number of dimensions, bytes per dimension).
/// - **`.kdi`**: stores inner nodes of the KD-tree.
/// - **`.kdd`**: stores leaf nodes, where most of the indexed data resides.
///
/// See the [Lucene BKD wiki] for detailed data structures of the three files.
///
/// [BKD paper]: https://www.cs.duke.edu/~pankaj/publications/papers/bkd-sstd.pdf
/// [Lucene BKD wiki]: https://cwiki.apache.org/confluence/pages/viewpage.action?pageId=173081898
pub struct Lucene90PointsFormat;
impl Default for Lucene90PointsFormat {
    fn default() -> Self {
        Lucene90PointsFormat
    }
}

impl Lucene90PointsFormat {
    pub(crate) const DATA_CODEC_NAME: &'static str = "Lucene90PointsFormatData";
    pub(crate) const INDEX_CODEC_NAME: &'static str = "Lucene90PointsFormatIndex";
    pub(crate) const META_CODEC_NAME: &'static str = "Lucene90PointsFormatMeta";

    /// Filename extension for the leaf blocks
    pub(crate) const DATA_EXTENSION: &'static str = "kdd";
    /// Filename extension for the index per field
    pub(crate) const INDEX_EXTENSION: &'static str = "kdi";
    /// Filename extension for the meta per field
    pub(crate) const META_EXTENSION: &'static str = "kdm";

    pub(crate) const VERSION_START: i32 = 0;
    pub(crate) const VERSION_CURRENT: i32 = Self::VERSION_START;
}

impl PointsFormat for Lucene90PointsFormat {
    fn fields_writer<D>(&self, state: &SegmentWriteState<D>) -> Result<PointsWriterEnum>
    where
        D: Directory,
    {
        Ok(PointsWriterEnum::Lucene90(Lucene90PointWriter::new(state)))
    }

    fn fields_reader<D>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<PointsReaderEnum<D::IndexInputType>>
    where
        D: Directory,
    {
        Ok(PointsReaderEnum::Lucene90(Lucene90PointsReader::new(state)))
    }
}
