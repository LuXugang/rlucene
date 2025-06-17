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
pub struct Sorter;

pub trait DocMap {
    /// Given a doc ID from the original index, return its ordinal in the sorted
    /// index.
    fn old_to_new(&self, doc_id: i32) -> i32;

    /// Given the ordinal of a doc ID, return its doc ID in the original index.
    fn new_to_old(&self, doc_id: i32) -> i32;

    /// Return the number of documents in this map.
    /// This must equal the number of documents in the sorted `LeafReader`.
    fn size(&self) -> usize;
}

pub struct DummyDocMap;
impl DocMap for DummyDocMap {
    fn old_to_new(&self, _doc_id: i32) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn new_to_old(&self, _doc_id: i32) -> i32 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn size(&self) -> usize {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
