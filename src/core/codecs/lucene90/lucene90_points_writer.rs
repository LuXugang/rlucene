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
use crate::core::codecs::points_reader::PointsReader;
use crate::core::codecs::points_writer::PointsWriter;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use std::sync::Arc;

pub struct Lucene90PointWriter;

impl Lucene90PointWriter {
    pub fn new<D>(_state: &SegmentWriteState<D>) -> Self
    where
        D: Directory,
    {
        todo!()
    }
}

impl PointsWriter for Lucene90PointWriter {
    fn write_field<PR>(
        &mut self,
        _field_info: &Arc<FieldInfo>,
        _values: &mut PR,
    ) -> crate::core::util::error::lucene_error::Result<()>
    where
        PR: PointsReader,
    {
        todo!()
    }

    fn finish(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
        todo!()
    }
}
