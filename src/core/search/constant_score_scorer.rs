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

pub struct DocIdSetIteratorWrapper<D>
where
    D: DocIdSetIterator,
{
    doc: i32,
    delegate: D,
}

impl<D> DocIdSetIteratorWrapper<D>
where
    D: DocIdSetIterator,
{
    pub fn new(delegate: D) -> Self {
        Self { doc: -1, delegate }
    }
}

impl<D> DocIdSetIterator for DocIdSetIteratorWrapper<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.doc = self.delegate.next_doc()?;
        Ok(self.doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.doc = self.delegate.advance(target)?;
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        self.delegate.cost()
    }
}
