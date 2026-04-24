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
use crate::core::index::dummy::dummy_terms::DummyTerms;
use crate::core::index::fields::Fields;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::iterator::VecIter;

pub struct DummyFields;
impl Fields for DummyFields {
  type FieldIter<'a> = VecIter<'a, String>;

  fn iterator(&self) -> Result<Self::FieldIter<'_>> {
    dummy_unreachable!()
  }

  type Terms = DummyTerms;

  fn terms(&self, _field: &str) -> Result<Option<Self::Terms>> {
    dummy_unreachable!()
  }

  fn size(&self) -> Result<i32> {
    dummy_unreachable!()
  }
}
