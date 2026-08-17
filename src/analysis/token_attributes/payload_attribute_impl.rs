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
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
#[cfg(any(test, debug_assertions))]
use std::collections::HashSet;
/// Default implementation of [`PayloadAttribute`].
#[derive(PartialEq, Eq, Clone)]
pub struct PayloadAttributeImpl {
  payload: Option<BytesRef<Vec<u8>>>,
  #[cfg(any(test, debug_assertions))]
  attribute: HashSet<String>,
}
impl Default for PayloadAttributeImpl {
  fn default() -> Self {
    Self::new()
  }
}

impl PayloadAttributeImpl {
  pub fn new() -> Self {
    #[cfg(any(test, debug_assertions))]
    let mut attribute = HashSet::new();
    #[cfg(any(test, debug_assertions))]
    {
      attribute.insert(<Self as PayloadAttribute>::ATTRIBUTE_NAME.to_string());
    }
    Self {
      payload: None,
      #[cfg(any(test, debug_assertions))]
      attribute,
    }
  }
}

impl Attribute for PayloadAttributeImpl {
  #[cfg(any(test, debug_assertions))]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl PayloadAttribute for PayloadAttributeImpl {
  fn get_payload(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.payload.as_ref()
  }

  fn set_payload(&mut self, payload: Option<BytesRef<Vec<u8>>>) {
    self.payload = payload;
  }
}

impl AttributeImpl for PayloadAttributeImpl {
  fn clear(&mut self) {
    self.payload = None;
  }

  type AttributeImpl = Self;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    match self.payload {
      Some(ref payload) => other.payload = Some(BytesRef::deep_copy_of(payload)),
      None => {
        other.payload = None;
      },
    }
    Ok(())
  }
}
