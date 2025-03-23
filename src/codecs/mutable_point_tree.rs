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
use crate::index::point_values_writer::MutableSortingPointValues;
use crate::index::BytesRef;
#[cfg(test)]
use crate::test::util::bkd::test_bkd::{MutablePointTreeMock1, MutablePointTreeMock2};
#[cfg(test)]
use crate::util::bkd::mutable_point_tree_reader_utils::tests::DummyPointsReader;
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

pub enum MutablePointTreeEnum {
    #[cfg(test)]
    Dummy(DummyPointsReader),
    #[cfg(test)]
    Mock1(MutablePointTreeMock1),
    #[cfg(test)]
    Mock2(MutablePointTreeMock2),
    MutableSorting(MutableSortingPointValues),
}

impl Clone for MutablePointTreeEnum {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl MutablePointTree for MutablePointTreeEnum {
    fn get_value(&self, i: i32, packed_value: &mut BytesRef) {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.get_value(i, packed_value),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.get_value(i, packed_value),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.get_value(i, packed_value),
            MutablePointTreeEnum::MutableSorting(reader) => reader.get_value(i, packed_value),
        }
    }

    fn get_byte_at(&self, i: i32, k: i32) -> u8 {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.get_byte_at(i, k),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.get_byte_at(i, k),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.get_byte_at(i, k),
            MutablePointTreeEnum::MutableSorting(reader) => reader.get_byte_at(i, k),
        }
    }

    fn get_doc_id(&self, i: i32) -> i32 {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.get_doc_id(i),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.get_doc_id(i),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.get_doc_id(i),
            MutablePointTreeEnum::MutableSorting(reader) => reader.get_doc_id(i),
        }
    }

    fn swap(&mut self, i: i32, j: i32) {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.swap(i, j),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.swap(i, j),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.swap(i, j),
            MutablePointTreeEnum::MutableSorting(reader) => reader.swap(i, j),
        }
    }

    fn save(&mut self, i: i32, j: i32) {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.save(i, j),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.save(i, j),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.save(i, j),
            MutablePointTreeEnum::MutableSorting(reader) => reader.save(i, j),
        }
    }

    fn restore(&mut self, i: i32, j: i32) {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.restore(i, j),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.restore(i, j),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.restore(i, j),
            MutablePointTreeEnum::MutableSorting(reader) => reader.restore(i, j),
        }
    }
}

impl PointTree for MutablePointTreeEnum {
    fn move_to_child(&mut self) -> Result<bool, LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.move_to_child(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.move_to_child(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.move_to_child(),
            MutablePointTreeEnum::MutableSorting(reader) => reader.move_to_child(),
        }
    }

    fn move_to_sibling(&mut self) -> Result<bool, LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.move_to_sibling(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.move_to_sibling(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.move_to_sibling(),
            MutablePointTreeEnum::MutableSorting(reader) => reader.move_to_sibling(),
        }
    }

    fn move_to_parent(&mut self) -> Result<bool, LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.move_to_parent(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.move_to_parent(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.move_to_parent(),
            MutablePointTreeEnum::MutableSorting(reader) => reader.move_to_parent(),
        }
    }

    fn get_min_packed_value(&self) -> Result<&[u8], LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.get_min_packed_value(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.get_min_packed_value(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.get_min_packed_value(),
            MutablePointTreeEnum::MutableSorting(reader) => reader.get_min_packed_value(),
        }
    }

    fn get_max_packed_value(&self) -> Result<&[u8], LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.get_max_packed_value(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.get_max_packed_value(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.get_max_packed_value(),
            MutablePointTreeEnum::MutableSorting(reader) => reader.get_max_packed_value(),
        }
    }

    fn size(&self) -> Result<i64, LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.size(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.size(),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.size(),
            MutablePointTreeEnum::MutableSorting(reader) => reader.size(),
        }
    }

    fn visit_doc_ids(&mut self, _visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.visit_doc_ids(_visitor),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.visit_doc_ids(_visitor),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.visit_doc_ids(_visitor),
            MutablePointTreeEnum::MutableSorting(reader) => reader.visit_doc_ids(_visitor),
        }
    }

    fn visit_doc_values(
        &mut self,
        _visitor: &mut impl IntersectVisitor,
    ) -> Result<(), LuceneError> {
        match self {
            #[cfg(test)]
            MutablePointTreeEnum::Dummy(reader) => reader.visit_doc_values(_visitor),
            #[cfg(test)]
            MutablePointTreeEnum::Mock1(reader) => reader.visit_doc_values(_visitor),
            #[cfg(test)]
            MutablePointTreeEnum::Mock2(reader) => reader.visit_doc_values(_visitor),
            MutablePointTreeEnum::MutableSorting(reader) => reader.visit_doc_values(_visitor),
        }
    }
}
