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
use crate::util::bkd::heap_point_write::HeapPointValue;
use crate::util::bkd::offline_point_reader::OfflinePointValue;
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a dimensional point value written in the BKD tree.
#[allow(unused)]
pub(crate) trait PointValue {
    /// Sets a new value by changing the offset.
    fn set_offset(&mut self, offset: i32);

    /// Returns the packed values for the dimensions.
    fn packed_value(&self) -> (Rc<RefCell<Vec<u8>>>, i32, i32);

    /// Returns the docID.
    fn doc_id(&self) -> i32;

    /// Returns the byte representation of the packed value together with the docID.
    fn packed_value_doc_id_bytes(&self) -> (Rc<RefCell<Vec<u8>>>, i32, i32);
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

    fn packed_value(&self) -> (Rc<RefCell<Vec<u8>>>, i32, i32) {
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

    fn packed_value_doc_id_bytes(&self) -> (Rc<RefCell<Vec<u8>>>, i32, i32) {
        match self {
            PointValueEnum::Heap(heap) => heap.packed_value_doc_id_bytes(),
            PointValueEnum::Offline(offline) => {
                offline.packed_value_doc_id_bytes()
            },
        }
    }
}
