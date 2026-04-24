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
use crate::core::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::core::index::BytesRef;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::merge_state::MergeState;
use crate::core::store::DataInput;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;

pub struct DummyStoredFieldsWriter;
impl StoredFieldsWriter for DummyStoredFieldsWriter {
  fn start_document(&mut self) -> Result<()> {
    dummy_unreachable!()
  }

  fn finish_document(&mut self) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_i32(&mut self, _field_info: &FieldInfo, _value: i32) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_i64(&mut self, _field_info: &FieldInfo, _value: i64) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_f32(&mut self, _field_info: &FieldInfo, _value: f32) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_f64(&mut self, _field_info: &FieldInfo, _value: f64) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_with_input(
    &mut self,
    _field_info: &FieldInfo,
    _input: &mut impl DataInput,
    _length: i32,
  ) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_bytes(
    &mut self,
    _field_info: &FieldInfo,
    _value: &BytesRef<Vec<u8>>,
  ) -> Result<()> {
    dummy_unreachable!()
  }

  fn write_field_str(&mut self, _field_info: &FieldInfo, _value: &str) -> Result<()> {
    dummy_unreachable!()
  }

  fn finish<D>(&mut self, _num_docs: i32, _dir: &D) -> Result<()>
  where
    D: Directory,
  {
    dummy_unreachable!()
  }

  fn merge<D, D1, CR>(&mut self, _merge_state: &mut MergeState<D, CR>, _dir: &D1) -> Result<i32>
  where
    D: Directory,
    D1: Directory,
    CR: CodecReader,
    Self: Sized,
  {
    dummy_unreachable!()
  }
}
