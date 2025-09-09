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
use crate::core::codecs::lucene90_points_writer::Lucene90PointWriter;
use crate::core::codecs::points_reader::PointsReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub trait PointsWriter {
    /// Write all values contained in the provided reader
    fn write_field<PR>(&mut self, field_info: &Arc<FieldInfo>, values: &mut PR) -> Result<()>
    where
        PR: PointsReader;

    /// Called once at the end before close
    fn finish(&mut self) -> Result<()>;
}

pub enum PointsWriterEnum {
    Lucene90(Lucene90PointWriter),
}
impl PointsWriter for PointsWriterEnum {
    fn write_field<PR>(&mut self, field_info: &Arc<FieldInfo>, values: &mut PR) -> Result<()>
    where
        PR: PointsReader,
    {
        match self {
            PointsWriterEnum::Lucene90(writer) => writer.write_field(field_info, values),
        }
    }

    fn finish(&mut self) -> Result<()> {
        match self {
            PointsWriterEnum::Lucene90(writer) => writer.finish(),
        }
    }
}
