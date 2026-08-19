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
use crate::core::codecs::dummy::dummy_mutable_point_tree::DummyMutablePointTree;
use crate::core::index::dummy::dummy_point_tree::DummyPointTree;
use crate::core::index::point_values::{PointTreeEnum, PointValues};
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

#[derive(Clone)]
pub struct DummyPointValues;
impl PointValues for DummyPointValues {
  fn get_min_packed_value(&self) -> Result<Option<Cow<'_, [u8]>>> {
    dummy_unreachable!()
  }

  fn get_max_packed_value(&self) -> Result<Option<Cow<'_, [u8]>>> {
    dummy_unreachable!()
  }

  fn get_num_dimensions(&self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn get_num_index_dimensions(&self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn get_bytes_per_dimension(&self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn size(&self) -> Result<usize> {
    dummy_unreachable!()
  }

  fn get_doc_count(&self) -> Result<i32> {
    dummy_unreachable!()
  }

  type PointTree = DummyPointTree;
  type MutablePointTree = DummyMutablePointTree;

  fn get_point_tree(&self) -> Result<PointTreeEnum<Self::MutablePointTree, Self::PointTree>> {
    dummy_unreachable!()
  }
}
