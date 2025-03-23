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
use crate::index::index_writer::DocMapIndexWriter;
#[cfg(test)]
use crate::test::util::bkd::test_bkd::DocMapImpl;

/// A map of doc IDs.
pub trait DocMap {
    /// Return the mapped docID or -1 if the given doc is not mapped.
    fn get(&self, doc_id: i32) -> i32;
}

pub enum DocMapEnum {
    #[cfg(test)]
    DocMapMock(DocMapImpl),
    DocMapImpl(DocMapIndexWriter),
}
impl DocMap for DocMapEnum {
    fn get(&self, doc_id: i32) -> i32 {
        match self {
            #[cfg(test)]
            DocMapEnum::DocMapMock(doc_map) => doc_map.get(doc_id),
            DocMapEnum::DocMapImpl(doc_map) => doc_map.get(doc_id),
        }
    }
}
