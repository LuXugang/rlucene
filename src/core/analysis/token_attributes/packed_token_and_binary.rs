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
use crate::core::analysis::token_attributes::bytes_term_attribute_impl::BytesTermAttributeImpl;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::util::attribute_impl::AttributeImpl;

#[derive(Default)]
pub struct BinaryTokenStreamAttributeImpl {
  packed_token: PackedTokenAttributeImpl,
  binary: BytesTermAttributeImpl,
}
impl BinaryTokenStreamAttributeImpl {
  pub fn get_packed_token(&self) -> &PackedTokenAttributeImpl {
    &self.packed_token
  }
  pub fn get_packed_token_mut(&mut self) -> &mut PackedTokenAttributeImpl {
    &mut self.packed_token
  }
  pub fn get_binary(&self) -> &BytesTermAttributeImpl {
    &self.binary
  }
  pub fn get_binary_mut(&mut self) -> &mut BytesTermAttributeImpl {
    &mut self.binary
  }
  pub fn clear(&mut self) {
    self.binary.clear();
    self.packed_token.clear()
  }
}
