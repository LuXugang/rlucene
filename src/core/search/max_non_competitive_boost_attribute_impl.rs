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
use crate::core::search::max_non_competitive_boost_attribute::MaxNonCompetitiveBoostAttribute;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
/// Implementation class for [`MaxNonCompetitiveBoostAttribute`].
#[derive(Clone)]
pub struct MaxNonCompetitiveBoostAttributeImpl {
  max_non_competitive_boost: f32,
  competitive_term: Option<BytesRef<Vec<u8>>>,
}
impl Default for MaxNonCompetitiveBoostAttributeImpl {
  fn default() -> Self {
    Self::new()
  }
}
impl MaxNonCompetitiveBoostAttributeImpl {
  pub fn new() -> Self {
    Self {
      max_non_competitive_boost: f32::NEG_INFINITY,
      competitive_term: None,
    }
  }
}
impl Attribute for MaxNonCompetitiveBoostAttributeImpl {}
impl AttributeImpl for MaxNonCompetitiveBoostAttributeImpl {
  fn clear(&mut self) {
    self.max_non_competitive_boost = f32::NEG_INFINITY;
    self.competitive_term = None;
  }

  type AttributeImpl = Self;

  fn copy_to(&self, other: &mut Self::AttributeImpl) -> Result<()> {
    other.set_max_non_competitive_boost(self.max_non_competitive_boost);
    other.set_competitive_term(self.competitive_term.clone());
    Ok(())
  }
}

impl MaxNonCompetitiveBoostAttribute for MaxNonCompetitiveBoostAttributeImpl {
  fn set_max_non_competitive_boost(&mut self, max_non_competitive_boost: f32) {
    self.max_non_competitive_boost = max_non_competitive_boost;
  }

  fn get_max_non_competitive_boost(&self) -> f32 {
    self.max_non_competitive_boost
  }

  fn set_competitive_term(&mut self, competitive_term: Option<BytesRef<Vec<u8>>>) {
    self.competitive_term = competitive_term;
  }

  fn get_competitive_term(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.competitive_term.as_ref()
  }
}
