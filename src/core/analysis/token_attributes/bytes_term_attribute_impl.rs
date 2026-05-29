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
use crate::core::analysis::token_attributes::bytes_term_attribute::BytesTermAttribute;
use crate::core::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::Result;
#[cfg(test)]
use crate::test::core::analysis::base_token_stream_test_case::{
  CheckClearAttributesAttribute, CheckClearAttributesAttributeImpl,
};
use std::borrow::Cow;
#[cfg(debug_assertions)]
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

/// Implementation class for BytesTermAttribute.
pub struct BytesTermAttributeImpl {
  bytes: Option<BytesRef<Vec<u8>>>,
  #[cfg(test)]
  check_clear_attributes: CheckClearAttributesAttributeImpl,
  #[cfg(debug_assertions)]
  attribute: HashSet<String>,
}
impl Default for BytesTermAttributeImpl {
  fn default() -> Self {
    Self::new()
  }
}
#[cfg(test)]
impl CheckClearAttributesAttribute for BytesTermAttributeImpl {
  fn get_and_reset_clear_called(&mut self) -> bool {
    self.check_clear_attributes.get_and_reset_clear_called()
  }
}
impl BytesTermAttributeImpl {
  pub fn new() -> Self {
    // TODO is there a better way to do this?
    #[cfg(debug_assertions)]
    let mut attribute = HashSet::new();
    #[cfg(debug_assertions)]
    {
      attribute.insert(<Self as BytesTermAttribute>::ATTRIBUTE_NAME.to_string());
      attribute.insert(<Self as TermToBytesRefAttribute>::ATTRIBUTE_NAME.to_string());
    }
    Self {
      bytes: None,
      #[cfg(test)]
      check_clear_attributes: CheckClearAttributesAttributeImpl::new(),
      #[cfg(debug_assertions)]
      attribute,
    }
  }
}

impl Attribute for BytesTermAttributeImpl {
  #[cfg(debug_assertions)]
  fn get_attribute_name(&self) -> Result<&HashSet<String>> {
    Ok(&self.attribute)
  }
}

impl Clone for BytesTermAttributeImpl {
  fn clone(&self) -> Self {
    let mut c = BytesTermAttributeImpl::new();
    self.copy_to(&mut c).expect("copy_to should not fail");
    c
  }
}

impl AttributeImpl for BytesTermAttributeImpl {
  fn clear(&mut self) {
    let _ = self.bytes.take();
  }

  type AttributeImpl = BytesTermAttributeImpl;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    match self.bytes {
      Some(ref bytes) => other.bytes = Some(BytesRef::deep_copy_of(bytes)),
      None => other.bytes = None,
    }
    Ok(())
  }
}

impl TermToBytesRefAttribute for BytesTermAttributeImpl {
  fn get_bytes_ref(&mut self) -> Option<Cow<'_, BytesRef<Vec<u8>>>> {
    self.bytes.as_ref().map(Cow::Borrowed)
  }
}

impl BytesTermAttribute for BytesTermAttributeImpl {
  fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    self.bytes = bytes;
    Ok(())
  }
}
impl Hash for BytesTermAttributeImpl {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.bytes.hash(state);
  }
}
impl PartialEq for BytesTermAttributeImpl {
  fn eq(&self, other: &Self) -> bool {
    self.bytes == other.bytes
  }
}
impl Display for BytesTermAttributeImpl {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl AttributeSource for BytesTermAttributeImpl {
  fn set_bytes_ref(&mut self, bytes: Option<BytesRef<Vec<u8>>>) -> Result<()> {
    BytesTermAttribute::set_bytes_ref(self, bytes)
  }

  fn get_bytes_ref(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    Ok(TermToBytesRefAttribute::get_bytes_ref(self))
  }

  fn clear_attributes(&mut self) {
    self.clear()
  }
}
