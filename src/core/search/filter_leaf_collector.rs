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
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
/// `LeafCollector` delegator.
pub struct FilterLeafCollector<L>
where
    L: LeafCollector,
{
    in_: L,
}
impl<L> FilterLeafCollector<L>
where
    L: LeafCollector,
{
    pub fn new(in_: L) -> Self {
        Self { in_ }
    }
}
impl<L> LeafCollector for FilterLeafCollector<L>
where
    L: LeafCollector,
{
    fn set_scorer<S>(&mut self, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.in_.set_scorer(scorer)
    }

    fn collect<S>(&mut self, doc: i32, scorer: &mut S) -> Result<()>
    where
        S: Scorable,
    {
        self.in_.collect(doc, scorer)
    }

    type DocIdSetIterator = L::DocIdSetIterator;

    fn competitive_iterator(&mut self) -> Result<Option<Self::DocIdSetIterator>> {
        self.in_.competitive_iterator()
    }

    fn finish(&mut self) -> Result<()> {
        self.in_.finish()
    }
}
impl<L> Display for FilterLeafCollector<L>
where
    L: LeafCollector,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", std::any::type_name::<Self>(), self.in_)
    }
}
