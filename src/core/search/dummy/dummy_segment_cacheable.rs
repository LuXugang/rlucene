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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::util::error::lucene_error::Result;
use std::marker::PhantomData;

pub struct DummySegmentCacheable<LR, IRC>
where
    LR: LeafReader,
    IRC: IndexReaderContext<LeafReader = LR>,
{
    _leaf_reader: PhantomData<LR>,
    _irc: PhantomData<IRC>,
}

impl<LR, IRC> Default for DummySegmentCacheable<LR, IRC>
where
    LR: LeafReader,
    IRC: IndexReaderContext<LeafReader = LR>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<LR, IRC> DummySegmentCacheable<LR, IRC>
where
    LR: LeafReader,
    IRC: IndexReaderContext<LeafReader = LR>,
{
    pub fn new() -> Self {
        Self {
            _leaf_reader: PhantomData,
            _irc: PhantomData,
        }
    }
}

impl<LR, IRC> SegmentCacheable for DummySegmentCacheable<LR, IRC>
where
    LR: LeafReader,
    IRC: IndexReaderContext<LeafReader = LR>,
{
    type LeafReader = LR;
    type IRC = IRC;

    fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
