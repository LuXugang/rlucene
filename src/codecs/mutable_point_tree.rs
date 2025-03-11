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
use crate::index::point_values::{IntersectVisitor, PointTree};
use crate::index::BytesRef;
use crate::util::error::lucene_error::LuceneError;

/// One leaf [PointTree] whose order of points can be changed.
/// This trait is useful for codecs to optimize flush.
pub trait MutablePointTree: PointTree {
    /// Set `packed_value` with a reference to the packed bytes of the i-th value.
    fn get_value(&self, i: i32, packed_value: &mut BytesRef);

    /// Get the k-th byte of the i-th value.
    fn get_byte_at(&self, i: i32, k: i32) -> u8;

    /// Return the doc ID of the i-th value.
    fn get_doc_id(&self, i: i32) -> i32;

    /// Swap the i-th and j-th values.
    fn swap(&mut self, i: i32, j: i32);

    /// Save the i-th value into the j-th position in temporary storage.
    fn save(&mut self, i: i32, j: i32);

    /// Restore values between i-th and j-th (excluding) in temporary storage into original storage.
    fn restore(&mut self, i: i32, j: i32);
}

pub enum MutablePointTreeEnum {}

impl PointTree for MutablePointTreeEnum {
    fn clone_tree(&self) -> Self {
        todo!()
    }

    fn move_to_child(&mut self) -> Result<bool, LuceneError> {
        todo!()
    }

    fn move_to_sibling(&mut self) -> Result<bool, LuceneError> {
        todo!()
    }

    fn move_to_parent(&mut self) -> Result<bool, LuceneError> {
        todo!()
    }

    fn get_min_packed_value(&self) -> &[u8] {
        todo!()
    }

    fn get_max_packed_value(&self) -> &[u8] {
        todo!()
    }

    fn size(&self) -> u64 {
        todo!()
    }

    fn visit_doc_ids(&self, visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        todo!()
    }

    fn visit_doc_values(&self, visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        todo!()
    }
}

impl Clone for MutablePointTreeEnum {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl MutablePointTree for MutablePointTreeEnum {
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
