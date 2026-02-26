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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::util::error::lucene_error::Result;
use std::marker::PhantomData;

pub struct DummySegmentCacheable<IRC>
where
    IRC: IndexReaderContext,
{
    _irc: PhantomData<IRC>,
}

impl<IRC> Default for DummySegmentCacheable<IRC>
where
    IRC: IndexReaderContext,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<IRC> DummySegmentCacheable<IRC>
where
    IRC: IndexReaderContext,
{
    pub fn new() -> Self {
        Self { _irc: PhantomData }
    }
}

impl<IRC> SegmentCacheable<IRC> for DummySegmentCacheable<IRC>
where
    IRC: IndexReaderContext,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
