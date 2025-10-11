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
use crate::core::search::doc_id_stream::DocIdStream;
use crate::core::search::dummy::dummy_doc_id_set_iterator::DummyDocIdSetIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;

pub struct DummyLeafCollector;
impl LeafCollector for DummyLeafCollector {
    fn set_scorer<S>(&mut self, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn collect<S>(&mut self, _doc: i32, _scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn collect_stream<DS, S>(&mut self, _stream: &mut DS, _scorer: &mut S) -> Result<()>
    where
        DS: DocIdStream,
        S: Scorable,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocIdSetIterator = DummyDocIdSetIterator;

    fn competitive_iterator(&mut self) -> Result<Option<&mut Self::DocIdSetIterator>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn finish(&mut self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
