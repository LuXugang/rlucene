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
