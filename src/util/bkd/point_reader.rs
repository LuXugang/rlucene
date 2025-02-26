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
use crate::util::bkd::heap_point_reader::HeapPointReader;
use crate::util::bkd::offline_point_reader::OfflinePointReader;
use crate::util::bkd::point_value::PointValueEnum;
use crate::util::error::lucene_error::LuceneError;
use std::cell::RefCell;
use std::rc::Rc;

/// One-pass iterator through all points previously written with a PointWriter,
/// abstracting away whether points are read from offline disk or from arrays in heap.
pub trait PointReader {
    /// Advances the iterator.
    ///
    /// Returns `Ok(true)` if there is another point available,
    /// or `Ok(false)` if iteration is complete.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if an I/O error occurs during iteration.
    fn next(&mut self) -> Result<bool, LuceneError>;

    /// Returns the current point value.
    fn point_value(&self) -> Rc<RefCell<PointValueEnum>>;
}

pub enum PointReaderEnum {
    Offline(OfflinePointReader),
    Heap(HeapPointReader),
}
