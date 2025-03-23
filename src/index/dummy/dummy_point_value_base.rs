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
use crate::index::dummy::dummy_point_tree::DummyPointTree;
use crate::index::point_values::PointValuesBase;
use crate::util::error::lucene_error::Result;

pub struct DummyPointValuesBase;
impl PointValuesBase for DummyPointValuesBase {
    fn get_min_packed_value(&self) -> Result<Option<Vec<u8>>> {
        unreachable!("should not be called")
    }

    fn get_max_packed_value(&self) -> Result<Option<Vec<u8>>> {
        unreachable!("should not be called")
    }

    fn get_num_dimensions(&self) -> Result<i32> {
        unreachable!("should not be called")
    }

    fn get_num_index_dimensions(&self) -> Result<i32> {
        unreachable!("should not be called")
    }

    fn get_bytes_per_dimension(&self) -> Result<i32> {
        unreachable!("should not be called")
    }

    fn size(&self) -> Result<i64> {
        unreachable!("should not be called")
    }

    fn get_doc_count(&self) -> Result<i32> {
        unreachable!("should not be called")
    }

    type PointTreeType = DummyPointTree;

    fn get_point_tree(&self) -> Result<Self::PointTreeType> {
        Ok(DummyPointTree)
    }
}
