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
use crate::util::error::lucene_error::LuceneError;

pub struct CommonUtil;
impl CommonUtil {
    pub fn check_from_index_size(
        from_index: i32,
        size: i32,
        length: i32,
    ) -> Result<i32, LuceneError> {
        if from_index < 0 || size < 0 || length < 0 {
            Err(LuceneError::array_index_out_of_bounds(format!(
                "from_index: {}, size: {}, and length {} must be non-negative",
                from_index, size, length
            )))
        } else if size > length - from_index {
            Err(LuceneError::array_index_out_of_bounds(format!(
                "size: {} is too large, from_index: {}, length: {}",
                size, from_index, length
            )))
        } else {
            Ok(from_index)
        }
    }
    pub(crate) fn miss_match(prior: &[u8], current: &[u8]) -> i32 {
        let miss_match = prior.iter().zip(current.iter()).position(|(a, b)| a != b);
        match miss_match {
            Some(miss_match) => {
                debug_assert!(miss_match <= i32::MAX as usize);
                miss_match as i32
            }
            None => -1,
        }
    }
}
