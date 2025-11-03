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
use crate::core::codecs::points_reader::PointsReaderType;
use crate::core::codecs::points_writer::PointsWriterType;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;

/// Encodes/decodes indexed points.
pub trait PointsFormat {
    fn fields_writer<D>(&self, state: &SegmentWriteState<D>) -> Result<PointsWriterType>
    where
        D: Directory;

    /// Reads a segment. NOTE: by the time this call returns, it must hold open any files it will need
    ///  to use; else, those files may be deleted. Additionally, required files may be deleted during
    ///  the execution of this call before there is a chance to open them. Under these circumstances an
    ///  IOException should be thrown by the implementation. IOExceptions are expected and will
    ///  automatically cause a retry of the segment opening logic with the newly revised segments.
    fn fields_reader<D>(
        &self,
        state: &SegmentReadState<D>,
    ) -> Result<PointsReaderType<D::IndexInput>>
    where
        D: Directory;
}
