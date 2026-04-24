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
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::util::attribute::Attribute;

pub struct DummyOffsetAttribute;

impl Attribute for DummyOffsetAttribute {}

impl OffsetAttribute for DummyOffsetAttribute {
  fn start_offset(&self) -> i32 {
    dummy_unreachable!()
  }

  fn set_offset(
    &mut self,
    _start_offset: i32,
    _end_offset: i32,
  ) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }

  fn end_offset(&self) -> i32 {
    dummy_unreachable!()
  }
}
