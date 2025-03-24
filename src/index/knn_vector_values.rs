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
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;

pub struct DocIndexIterator<D>
where
    D: DocIdSetIterator + DocIndexIteratorBase,
{
    delegate: D,
}
impl<D> DocIndexIterator<D>
where
    D: DocIdSetIterator + DocIndexIteratorBase,
{
    pub fn new(delegate: D) -> DocIndexIterator<D> {
        Self { delegate }
    }
}
impl<D> DocIdSetIterator for DocIndexIterator<D>
where
    D: DocIdSetIterator + DocIndexIteratorBase,
{
    fn doc_id(&self) -> i32 {
        self.delegate.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.delegate.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.delegate.advance(target)
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        self.delegate.slow_advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.delegate.cost()
    }
}
pub trait DocIndexIteratorBase {
    fn index(&self) -> Result<i32>;
}
