/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/

use crate::util::bkd::heap_point_write::HeapPointValue;
use crate::util::bkd::offline_point_reader::OfflinePointValue;

/// Represents a dimensional point value written in the BKD tree.
#[allow(unused)]
pub(crate) trait PointValue {
    /// Sets a new value by changing the offset.
    fn set_offset(&mut self, offset: i32);

    /// Returns the packed values for the dimensions.
    fn packed_value(&self) -> (&[u8], i32, i32);

    /// Returns the docID.
    fn doc_id(&self) -> i32;

    /// Returns the byte representation of the packed value together with the
    /// docID.
    fn packed_value_doc_id_bytes(&self) -> (&[u8], i32, i32);
}

pub(crate) enum PointValueEnum {
    Heap(HeapPointValue),
    Offline(OfflinePointValue),
}

impl PointValue for PointValueEnum {
    fn set_offset(&mut self, offset: i32) {
        match self {
            PointValueEnum::Heap(heap) => heap.set_offset(offset),
            PointValueEnum::Offline(offline) => offline.set_offset(offset),
        }
    }

    fn packed_value(&self) -> (&[u8], i32, i32) {
        match self {
            PointValueEnum::Heap(heap) => heap.packed_value(),
            PointValueEnum::Offline(offline) => offline.packed_value(),
        }
    }

    fn doc_id(&self) -> i32 {
        match self {
            PointValueEnum::Heap(heap) => heap.doc_id(),
            PointValueEnum::Offline(offline) => offline.doc_id(),
        }
    }

    fn packed_value_doc_id_bytes(&self) -> (&[u8], i32, i32) {
        match self {
            PointValueEnum::Heap(heap) => heap.packed_value_doc_id_bytes(),
            PointValueEnum::Offline(offline) => offline.packed_value_doc_id_bytes(),
        }
    }
}
