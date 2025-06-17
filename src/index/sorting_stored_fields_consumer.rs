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
use crate::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::index::stored_fields_consumer::StoredFieldsConsumer;
use crate::index::BytesRef;
use crate::store::directory::Directory;
use crate::store::DataInput;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    base: StoredFieldsConsumer<D>,
}
impl<D> SortingStoredFieldsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(directory: Arc<Mutex<D>>, info: Rc<SegmentInfo<D>>) -> Self {
        Self {
            base: StoredFieldsConsumer::new(directory, info),
        }
    }
}

/// A visitor that copies every field it sees in the provided [`StoredFieldsWriter`]
pub(crate) struct CopyVisitor;
impl StoredFieldVisitor for CopyVisitor {
    fn binary_field_with_input(
        &mut self,
        field_info: Rc<FieldInfo>,
        input: &mut impl DataInput,
        length: i32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_with_input(&field_info, input, length)
    }

    fn binary_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: Vec<u8>,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_bytes(&field_info, &BytesRef::from_bytes(value))
    }

    fn string_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: &str,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_str(&field_info, value)
    }

    fn int_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: i32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_i32(&field_info, value)
    }

    fn long_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: i64,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_i64(&field_info, value)
    }

    fn float_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: f32,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_f32(&field_info, value)
    }

    fn double_field(
        &mut self,
        field_info: Rc<FieldInfo>,
        value: f64,
        writer: &mut impl StoredFieldsWriter,
    ) -> Result<()> {
        writer.write_field_f64(&field_info, value)
    }

    fn needs_field(
        &mut self,
        _field_info: Rc<FieldInfo>,
        _writer: &mut impl StoredFieldsWriter,
    ) -> Result<Status> {
        Ok(Status::Yes)
    }
}
