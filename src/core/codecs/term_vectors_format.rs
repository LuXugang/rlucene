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

use crate::core::codecs::term_vectors_reader::{TermVectorsReader, TermVectorsReaderType};
use crate::core::codecs::term_vectors_writer::{TermVectorsWriter, TermVectorsWriterEnum};
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::{IOContext, IndexInput, IndexOutput};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// Controls the format of term vectors
pub trait TermVectorsFormat {
    type TermVectorsReader<T:IndexInput>: TermVectorsReader;

    /// Returns a [`TermVectorsReader`] to read term vectors.
    fn vectors_reader<D1, D2>(
        &self,
        directory: &D1,
        segment_info: &SegmentInfo<D2>,
        field_infos: Arc<FieldInfos>,
        context: &IOContext,
    ) -> Result<Self::TermVectorsReader<D1::IndexInput>>
    where
        D1: Directory,
        D2: Directory;

    type TermVectorsWriter<T:IndexOutput>: TermVectorsWriter;
    /// Returns a [`TermVectorsWriter`] to write term vectors.
    fn vectors_writer<D1, D2>(
        &self,
        directory: &D1,
        segment_info: &SegmentInfo<D2>,
        context: &IOContext,
    ) -> Result<Self::TermVectorsWriter<D1::IndexOutput>>
    where
        D1: Directory,
        D2: Directory;
}