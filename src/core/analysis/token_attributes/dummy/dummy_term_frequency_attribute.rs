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
use crate::core::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::core::util::attribute::Attribute;
use crate::core::util::error::lucene_error::Result;

pub struct DummyTermFrequencyAttribute;

impl Attribute for DummyTermFrequencyAttribute {}

impl TermFrequencyAttribute for DummyTermFrequencyAttribute {
  fn set_term_frequency(&mut self, _term_frequency: i32) -> Result<()> {
    dummy_unreachable!()
  }

  fn get_term_frequency(&self) -> i32 {
    dummy_unreachable!()
  }
}
