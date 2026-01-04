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
use crate::core::codecs::mutable_point_tree::MutablePointTree;
use crate::core::index::BytesRef;
use crate::core::index::point_values::{IntersectVisitor, PointTree};
use crate::core::util::clone::TryClone;
use crate::core::util::error::lucene_error::Result;

pub struct DummyMutablePointTree;

impl TryClone for DummyMutablePointTree {
    fn try_clone(&self) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl PointTree for DummyMutablePointTree {
    fn move_to_child(&mut self) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn move_to_sibling(&mut self) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn move_to_parent(&mut self) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_min_packed_value(&self) -> Result<&[u8]> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_max_packed_value(&self) -> Result<&[u8]> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn size(&self) -> Result<usize> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn visit_doc_ids<IV>(&mut self, _visitor: &mut IV) -> Result<()>
    where
        IV: IntersectVisitor,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn visit_doc_values<IV>(&mut self, _visitor: &mut IV) -> Result<()>
    where
        IV: IntersectVisitor,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl MutablePointTree for DummyMutablePointTree {
    fn get_value(&self, _i: usize, _packed_value: &mut BytesRef<Vec<u8>>) {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_byte_at(&self, _i: usize, _k: usize) -> u8 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_doc_id(&self, _i: usize) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn swap(&mut self, _i: usize, _j: usize) {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn save(&mut self, _i: usize, _j: usize) {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn restore(&mut self, _i: usize, _j: usize) {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
