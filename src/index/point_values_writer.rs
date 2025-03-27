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
use crate::codecs::mutable_point_tree::MutablePointTree;
use crate::index::point_values::PointTree;
use crate::index::BytesRef;

pub struct PointValuesWriter;

pub struct MutableSortingPointValues;
impl MutablePointTree for MutableSortingPointValues {
    fn get_value(&self, i: i32, packed_value: &mut BytesRef) {
        todo!()
    }

    fn get_byte_at(&self, i: i32, k: i32) -> u8 {
        todo!()
    }

    fn get_doc_id(&self, i: i32) -> i32 {
        todo!()
    }

    fn swap(&mut self, i: i32, j: i32) {
        todo!()
    }

    fn save(&mut self, i: i32, j: i32) {
        todo!()
    }

    fn restore(&mut self, i: i32, j: i32) {
        todo!()
    }
}

impl Clone for MutableSortingPointValues {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl PointTree for MutableSortingPointValues {}
