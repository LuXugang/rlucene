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
use crate::core::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImpl;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
#[cfg(debug_assertions)]
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
#[cfg(test)]
use crate::test::core::analysis::base_token_stream_test_case::CheckClearAttributesAttribute;
#[cfg(test)]
use crate::test::core::analysis::base_token_stream_test_case::CheckClearAttributesAttributeImpl;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::fmt::Display;

pub struct BinaryTokenStreamAttributeImpl {
  packed_token: CharTermAttributeImpl<PackedTokenAttributeImpl>,
  binary: BytesTermAttributeImpl,
  #[cfg(test)]
  check_clear_attributes: CheckClearAttributesAttributeImpl,
  #[cfg(debug_assertions)]
  attribute: HashSet<String>,
}

impl BinaryTokenStreamAttributeImpl {
  pub fn new() -> Result<Self> {
    let packed_token = PackedTokenAttributeImpl::new()?;
    let binary = BytesTermAttributeImpl::default();
    // TODO is there a better way to do this?
    #[cfg(debug_assertions)]
    let mut attribute = HashSet::new();
    #[cfg(debug_assertions)]
    {
      attribute.extend(packed_token.get_attribute_name()?.clone());
      attribute.extend(binary.get_attribute_name()?.clone());
    }
    Ok(Self {
      packed_token,
      binary,
      #[cfg(test)]
      check_clear_attributes: CheckClearAttributesAttributeImpl::new(),
      #[cfg(debug_assertions)]
      attribute,
    })
  }
}
impl BinaryTokenStreamAttributeImpl {
  pub fn get_packed_token(&self) -> &CharTermAttributeImpl<PackedTokenAttributeImpl> {
    &self.packed_token
  }
  pub fn get_packed_token_mut(&mut self) -> &mut CharTermAttributeImpl<PackedTokenAttributeImpl> {
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
impl Display for BinaryTokenStreamAttributeImpl {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.packed_token.fmt(f)
  }
}

#[cfg(test)]
impl AttributeImpl for BinaryTokenStreamAttributeImpl {
  fn clear(&mut self) {
    self.check_clear_attributes.clear();
  }

  type AttributeImpl = CheckClearAttributesAttributeImpl;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    self.check_clear_attributes.copy_to(other)
  }
}

#[cfg(debug_assertions)]
impl Attribute for BinaryTokenStreamAttributeImpl {
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

#[cfg(test)]
impl Clone for BinaryTokenStreamAttributeImpl {
  fn clone(&self) -> Self {
    unreachable!("")
  }
}

#[cfg(test)]
impl CheckClearAttributesAttribute for BinaryTokenStreamAttributeImpl {
  fn get_and_reset_clear_called(&mut self) -> bool {
    self.check_clear_attributes.get_and_reset_clear_called()
  }
}
