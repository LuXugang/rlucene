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
use crate::index::point_values::PointTree;
use crate::index::BytesRef;
#[cfg(test)]
use crate::test::util::bkd::test_bkd::{MutablePointTreeMock1, MutablePointTreeMock2};
use crate::util::access::AccessVec;
#[cfg(test)]
use crate::util::bkd::mutable_point_tree_reader_utils::tests::DummyPointsReader;

/// One leaf [PointTree] whose order of points can be changed.
/// This trait is useful for codecs to optimize flush.
pub trait MutablePointTree: PointTree {
    type AV: AccessVec<u8>;
    /// Set `packed_value` with a reference to the packed bytes of the i-th
    /// value.
    fn get_value(&self, i: i32, packed_value: &mut BytesRef<Self::AV>);

    /// Get the k-th byte of the i-th value.
    fn get_byte_at(&self, i: i32, k: i32) -> u8;

    /// Return the doc ID of the i-th value.
    fn get_doc_id(&self, i: i32) -> i32;

    /// Swap the i-th and j-th values.
    fn swap(&mut self, i: i32, j: i32);

    /// Save the i-th value into the j-th position in temporary storage.
    fn save(&mut self, i: i32, j: i32);

    /// Restore values between i-th and j-th (excluding) in temporary storage
    /// into original storage.
    fn restore(&mut self, i: i32, j: i32);
}
