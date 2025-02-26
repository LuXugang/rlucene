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
use crate::index::BytesRef;
use crate::util::bit_util::BitUtil;
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::point_value::PointValue;

/// Reusable implementation for a point value offline.
pub(crate) struct OfflinePointValue {
    pub(crate) offset: i32,
    pub(crate) packed_value_length: i32,
    pub(crate) packed_value_doc_id_length: i32,
}
impl OfflinePointValue {
    pub fn new(config: &BKDConfig) -> Self {
        Self {
            offset: 0,
            packed_value_length: config.packed_bytes_length(),
            packed_value_doc_id_length: config.bytes_per_doc(),
        }
    }
}
impl PointValue for OfflinePointValue {
    fn set_offset(&mut self, offset: i32) {
        self.offset = offset;
    }

    fn packed_value(&self) -> (i32, i32) {
        (self.offset, self.packed_value_length)
    }

    fn doc_id(&self, bytes: &[u8]) -> i32 {
        let position = (self.offset + self.packed_value_length) as usize;
        BitUtil::get_i32_be(&bytes[position..], 0)
    }

    fn packed_value_doc_id_bytes(&self) -> (i32, i32) {
        (self.offset, self.packed_value_doc_id_length)
    }
}
