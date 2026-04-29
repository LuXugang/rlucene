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
use crate::core::search::boost_attribute::{BoostAttribute, DEFAULT_BOOST};
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
/// Implementation class for [`BoostAttribute`].
pub struct BoostAttributeImpl {
  boost: f32,
}
impl Default for BoostAttributeImpl {
  fn default() -> Self {
    Self::new()
  }
}

impl BoostAttributeImpl {
  pub fn new() -> Self {
    Self {
      boost: DEFAULT_BOOST,
    }
  }
}

impl Attribute for BoostAttributeImpl {}

impl Clone for BoostAttributeImpl {
  fn clone(&self) -> Self {
    Self { boost: self.boost }
  }
}

impl AttributeImpl for BoostAttributeImpl {
  fn clear(&mut self) {
    self.boost = 1.0f32
  }

  type AttributeImpl = Self;

  fn copy_to(&self, other: &mut Self::AttributeImpl) {
    other.boost = self.boost
  }
}
impl BoostAttribute for BoostAttributeImpl {
  fn set_boost(&mut self, boost: f32) {
    self.boost = boost;
  }

  fn get_boost(&self) -> f32 {
    self.boost
  }
}
