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
use crate::core::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImplBase;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
pub struct DummyAttributeImpl;

impl Attribute for DummyAttributeImpl {}

impl Clone for DummyAttributeImpl {
  fn clone(&self) -> Self {
    unimplemented!("Dummy implementation: this method should never be called in real usage")
  }
}

impl AttributeImpl for DummyAttributeImpl {
  fn clear(&mut self) {
    unimplemented!("Dummy implementation: this method should never be called in real usage")
  }

  fn end(&mut self) {
    unimplemented!("Dummy implementation: this method should never be called in real usage")
  }

  type AttributeImpl = DummyAttributeImpl;

  fn copy_to(&self, _other: &mut Self::AttributeImpl) {
    unimplemented!("Dummy implementation: this method should never be called in real usage")
  }
}
impl CharTermAttributeImplBase for DummyAttributeImpl {}
