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
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashSet;

/// Format for live/deleted documents
pub trait LiveDocsFormat {
    type Bits: Bits;
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
        dir: &impl Directory,
        info: &SegmentCommitInfo<D>,
        context: &IOContext,
    ) -> Result<Self::Bits>
    where
        D: Directory;

    /// Persist live docs bits. Use
    /// [`SegmentCommitInfo#
    /// getNextDelGen`](SegmentCommitInfo::get_next_write_del_gen) to determine
    /// the generation of the deletes file you should write to.
    fn write_live_docs<D>(
        &self,
        bits: &impl Bits,
        dir: &impl Directory,
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
