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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
/// # Note
/// See [`JavaIntermediateBaseClass`](crate::migration_notes::JavaIntermediateBaseClass)
#[allow(dead_code)]
pub struct FilterDocIdSetIterator<D>
where
    D: DocIdSetIterator,
{
    in_: D,
}
impl<D> FilterDocIdSetIterator<D>
where
    D: DocIdSetIterator,
{
    pub fn new(in_: D) -> Self {
        Self { in_ }
    }
}
impl<D> DocIdSetIterator for FilterDocIdSetIterator<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.in_.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.in_.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.in_.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.in_.cost()
    }
}
