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

use crate::core::store::IndexInput;
use crate::core::util::bkd::heap_point_reader::HeapPointReader;
use crate::core::util::bkd::offline_point_reader::OfflinePointReader;
use crate::core::util::bkd::point_value::PointValueEnum;
use crate::core::util::error::lucene_error::Result;

/// One-pass iterator through all points previously written with a PointWriter,
/// abstracting away whether points are read from offline disk or from arrays in
/// heap.
pub trait PointReader {
  /// Advances the iterator.
  ///
  /// Returns `Ok(true)` if there is another point available,
  /// or `Ok(false)` if iteration is complete.
  ///
  /// # Errors
  ///
  /// Returns an `io::Error` if an I/O error occurs during iteration.
  fn next(&mut self) -> Result<bool>;

  /// Returns the current point value.
  fn point_value(&mut self) -> Result<&PointValueEnum>;
}

pub enum PointReaderEnum<I>
where
  I: IndexInput,
{
  Offline(OfflinePointReader<I>),
  Heap(HeapPointReader),
}
impl<I> PointReaderEnum<I>
where
  I: IndexInput,
{
  pub fn remove_points(&mut self) -> Option<PointValueEnum> {
    match self {
      PointReaderEnum::Offline(_) => None,
      PointReaderEnum::Heap(heap) => heap.remove_points(),
    }
  }
}
impl<I> PointReader for PointReaderEnum<I>
where
  I: IndexInput,
{
  fn next(&mut self) -> Result<bool> {
    match self {
      PointReaderEnum::Offline(offline) => offline.next(),
      PointReaderEnum::Heap(heap) => heap.next(),
    }
  }

  fn point_value(&mut self) -> Result<&PointValueEnum> {
    match self {
      PointReaderEnum::Offline(offline) => offline.point_value(),
      PointReaderEnum::Heap(heap) => heap.point_value(),
    }
  }
}
