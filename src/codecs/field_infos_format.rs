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
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::LuceneError;
use std::sync::{Arc, Mutex};

/// Encodes/decodes FieldInfos
///
/// # Experimental
pub trait FieldInfosFormat {
    /// Reads the FieldInfos previously written.
    fn read<D>(
        &self,
        directory: Arc<Mutex<D>>,
        segment_info: &SegmentInfo<D>,
        segment_suffix: &str,
        io_context: &IOContext,
    ) -> Result<FieldInfos, LuceneError>
    where
        D: Directory;

    /// Writes the provided FieldInfos.
    fn write<D>(
        &self,
        directory: Arc<Mutex<D>>,
        segment_info: &SegmentInfo<D>,
        segment_suffix: &str,
        infos: &FieldInfos,
        io_context: &IOContext,
    ) -> Result<(), LuceneError>
    where
        D: Directory;
}
