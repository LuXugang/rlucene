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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::{LeafReaderContext, TopParentMeta};

pub struct ReaderUtil;
impl ReaderUtil {
    pub fn get_top_level_context<LR>(leaf_reader: &LeafReaderContext<LR>) -> &TopParentMeta
    where
        LR: LeafReader,
    {
        leaf_reader.top_parent()
    }
    /// Returns index of the searcher/reader for document n in the array used to construct this searcher/reader.
    pub fn sub_index(n: i32, doc_starts: &[i32]) -> i32 {
        debug_assert!(doc_starts.len() <= i32::MAX as usize);
        // find searcher/reader for doc n:
        let size = doc_starts.len();
        let mut lo: i32 = 0; // search starts array
        let mut hi: i32 = (size as i32) - 1; // for first element less than n, return its index

        while hi >= lo {
            let mid = (lo + hi) >> 1;
            let mid_value = doc_starts[mid as usize];
            if n < mid_value {
                hi = mid - 1;
            } else if n > mid_value {
                lo = mid + 1;
            } else {
                // found a match
                let mut mid = mid;
                while (mid + 1) < size as i32 && doc_starts[(mid + 1) as usize] == mid_value {
                    mid += 1; // scan to last match
                }
                return mid;
            }
        }
        hi
    }
    /// Returns index of the searcher/reader for document n in the array used to construct this searcher/reader.
    pub fn sub_index_with_leaves<LR>(n: i32, leaves: &[LeafReaderContext<LR>]) -> usize
    where
        LR: LeafReader,
    {
        // find searcher/reader for doc n:
        let size = leaves.len();
        let mut lo: i32 = 0; // search starts array
        let mut hi: i32 = size as i32 - 1; // for first element less than n, return its index

        while hi >= lo {
            let mid = (lo + hi) >> 1;
            let mid_value = leaves[mid as usize].doc_base;

            if n < mid_value {
                hi = mid - 1;
            } else if n > mid_value {
                lo = mid + 1;
            } else {
                let mut mid = mid;
                while (mid + 1) < size as i32 && leaves[(mid + 1) as usize].doc_base == mid_value {
                    mid += 1;
                }
                return mid as usize;
            }
        }

        hi.max(0) as usize
    }
}
