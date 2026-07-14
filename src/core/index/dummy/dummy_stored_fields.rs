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
use crate::core::document::document::Document;
use crate::core::index::stored_field_visitor::StoredFieldVisitor;
use crate::core::index::stored_fields::{RawStoredFieldsReader, StoredFields};
use crate::core::store::dummy::dummy_index_input::DummyIndexInput;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashSet;

pub struct DummyStoredFields;
impl StoredFields for DummyStoredFields {
  fn prefetch(&mut self, _doc_id: i32) -> Result<()> {
    dummy_unreachable!()
  }

  fn document(&mut self, _doc_id: i32) -> Result<Document> {
    dummy_unreachable!()
  }

  fn document_with_visitor<S>(
    &mut self,
    _doc_id: i32,
    _visitor: &mut impl StoredFieldVisitor,
    _writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    dummy_unreachable!()
  }

  fn document_with_fields(
    &mut self,
    _doc_id: i32,
    _fields_to_load: &HashSet<String>,
  ) -> Result<Document> {
    dummy_unreachable!()
  }
}

impl RawStoredFieldsReader for DummyStoredFields {
  type IndexInput = DummyIndexInput;
}
