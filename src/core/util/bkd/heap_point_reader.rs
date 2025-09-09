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

use crate::core::util::bkd::point_reader::PointReader;
use crate::core::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::core::util::error::lucene_error::Result;

pub struct HeapPointReader {
    points: Option<PointValueEnum>,
    cur_read: i32,
    end: i32,
    bytes_per_doc: i32,
}

impl HeapPointReader {
    pub fn new(
        get_slice: Option<PointValueEnum>,
        start: i32,
        end: i32,
        bytes_per_doc: i32,
    ) -> Self {
        HeapPointReader {
            points: get_slice,
            cur_read: start - 1,
            end,
            bytes_per_doc,
        }
    }
    pub fn remove_points(&mut self) -> Option<PointValueEnum> {
        self.points.take()
    }
}
impl PointReader for HeapPointReader {
    fn next(&mut self) -> Result<bool> {
        self.cur_read += 1;
        Ok(self.cur_read < self.end)
    }

    fn point_value(&mut self) -> &PointValueEnum {
        debug_assert!(self.points.is_some());
        self.points
            .as_mut()
            .unwrap()
            .set_offset(self.bytes_per_doc * self.cur_read);
        self.points.as_ref().unwrap()
    }
}
