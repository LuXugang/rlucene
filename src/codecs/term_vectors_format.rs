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
use crate::codecs::term_vectors_reader::TermVectorsReaderEnum;
use crate::codecs::term_vectors_writer::TermVectorsWriterEnum;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_info::SegmentInfo;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use std::rc::Rc;

/// Controls the format of term vectors
pub trait TermVectorsFormat {
    /// Returns a [`TermVectorsReader`](crate::codecs::term_vectors_reader::TermVectorsReader) to read term vectors.
    fn vectors_reader<D1, D2>(
        &self,
        directory: &D1,
        segment_info: &SegmentInfo<D2>,
        field_infos: Rc<FieldInfos>,
        context: &IOContext,
    ) -> Result<TermVectorsReaderEnum<D1::IndexInput>>
    where
        D1: Directory,
        D2: Directory;
    /// Returns a [`TermVectorsWriter`](crate::codecs::term_vectors_writer::TermVectorsWriter) to write term vectors.
    fn vectors_writer<D1, D2>(
        &self,
        directory: &D1,
        segment_info: &SegmentInfo<D2>,
        context: &IOContext,
    ) -> Result<TermVectorsWriterEnum<D1>>
    where
        D1: Directory,
        D2: Directory;
}
