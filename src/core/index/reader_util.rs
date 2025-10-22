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
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use std::sync::Arc;

pub struct ReaderUtil;
impl ReaderUtil {
    pub fn get_top_level_context<LR>(
        leaf_reader: &LeafReaderContext<LR>,
    ) -> Option<Arc<CompositeReaderContext<<LR as LeafReader>::ParentReader>>>
    where
        LR: LeafReader,
    {
        leaf_reader.top_parent()
    }
    pub fn sub_index(n: i32, doc_starts: &[i32]) -> usize {
        debug_assert!(doc_starts.len() <= i32::MAX as usize);
        let size = doc_starts.len();
        let mut lo: i32 = 0;
        let mut hi: i32 = (size as i32) - 1;

        while hi >= lo {
            let mid = (lo + hi) >> 1;
            let mid_value = doc_starts[mid as usize];
            if n < mid_value {
                hi = mid - 1;
            } else if n > mid_value {
                lo = mid + 1;
            } else {
                let mut mid = mid;
                while (mid + 1) < size as i32 && doc_starts[(mid + 1) as usize] == mid_value {
                    mid += 1;
                }
                return mid as usize;
            }
        }
        hi.max(0) as usize
    }
}
