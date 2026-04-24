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
use crate::core::index::BytesRef;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

pub struct DummyAttributeSource;
impl AttributeSource for DummyAttributeSource {
  fn start_offset(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn end_offset(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn get_position_increment(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
    dummy_unreachable!()
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dummy_unreachable!()
  }

  fn get_term_frequency(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  fn end_attributes(&mut self) {
    dummy_unreachable!()
  }

  fn clear_attributes(&mut self) {
    dummy_unreachable!()
  }
}
