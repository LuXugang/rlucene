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
use crate::util::bkd::point_reader::PointReaderEnum;
use crate::util::bkd::point_value::PointValueEnum;
use crate::util::error::lucene_error::LuceneError;

/// Appends many points, and then at the end provides a PointReader to iterate those points.
/// This abstracts away whether we write to disk, or use simple arrays in heap.
pub trait PointWriter {
    /// Add a new point from the packed value and docId
    fn append_bytes(&mut self, packed_value: &[u8], doc_id: i32) -> Result<(), LuceneError>;

    /// Add a new point from a PointValue
    fn append_point_value(&mut self, point_value: &PointValueEnum) -> Result<(), LuceneError>;

    /// Returns a PointReader iterator to step through all previously added points
    fn get_reader(&mut self, start_point: i64, length: i64)
        -> Result<PointReaderEnum, LuceneError>;

    /// Return the number of points in this writer
    fn count(&self) -> i64;

    /// Removes any temp files behind this writer
    fn destroy(&mut self) -> Result<(), LuceneError>;
}
