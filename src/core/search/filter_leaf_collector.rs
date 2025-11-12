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
use std::marker::PhantomData;

/// `LeafCollector` delegator.
pub trait FilterSource<'a, L: LeafCollector> {
    fn as_mut(&mut self) -> &mut L;
}

pub struct OwnedSource<L>(pub L);
impl<'a, L: LeafCollector> FilterSource<'a, L> for OwnedSource<L> {
    #[inline]
    fn as_mut(&mut self) -> &mut L {
        &mut self.0
    }
}

impl<L: LeafCollector> From<L> for OwnedSource<L> {
    #[inline]
    fn from(inner: L) -> Self {
        OwnedSource(inner)
    }
}

pub struct BorrowedSource<'a, L>(pub &'a mut L);
impl<'a, L: LeafCollector> FilterSource<'a, L> for BorrowedSource<'a, L> {
    #[inline]
    fn as_mut(&mut self) -> &mut L {
        self.0
    }
}

pub struct FilterLeafCollector<'a, L, S>
where
    L: LeafCollector,
    S: FilterSource<'a, L>,
{
    pub(crate) inner: S,
    _phantom: PhantomData<&'a ()>,
    _phantom1: PhantomData<L>,
}

impl<'a, L, S> FilterLeafCollector<'a, L, S>
where
    L: LeafCollector,
    S: FilterSource<'a, L>,
{
    #[inline]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
            _phantom1: PhantomData,
        }
    }

    #[inline]
    fn inner_mut(&mut self) -> &mut L {
        self.inner.as_mut()
    }
}

pub type FilterLeafCollectorOwned<L> = FilterLeafCollector<'static, L, OwnedSource<L>>;
pub type FilterLeafCollectorRef<'a, L> = FilterLeafCollector<'a, L, BorrowedSource<'a, L>>;

impl<L> From<L> for FilterLeafCollectorOwned<L>
where
    L: LeafCollector,
{
    #[inline]
    fn from(inner: L) -> Self {
        FilterLeafCollector::new(OwnedSource::from(inner))
    }
}

impl<'a, L> From<&'a mut L> for FilterLeafCollectorRef<'a, L>
where
    L: LeafCollector,
{
    #[inline]
    fn from(inner: &'a mut L) -> Self {
        FilterLeafCollector::new(BorrowedSource(inner))
    }
}

impl<'a, L, S> LeafCollector for FilterLeafCollector<'a, L, S>
where
    L: LeafCollector,
    S: FilterSource<'a, L>,
{
    fn set_scorer<T>(&mut self, scorer: &mut T) -> Result<()>
    where
        T: Scorable,
    {
        self.inner_mut().set_scorer(scorer)
    }

    fn collect<T>(&mut self, doc: i32, scorer: &mut T) -> Result<()>
    where
        T: Scorable,
    {
        self.inner_mut().collect(doc, scorer)
    }

    type DocIdSetIteratorRef<'b>
        = L::DocIdSetIteratorRef<'b>
    where
        Self: 'b,
        L: 'b;

    fn competitive_iterator(&mut self) -> Result<Option<Self::DocIdSetIteratorRef<'_>>> {
        self.inner_mut().competitive_iterator()
    }

    fn finish(&mut self) -> Result<()> {
        self.inner_mut().finish()
    }
}

impl<'a, L, S> Display for FilterLeafCollector<'a, L, S>
where
    L: LeafCollector + Display,
    S: FilterSource<'a, L>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}<{}>",
            std::any::type_name::<Self>(),
            std::any::type_name::<L>()
        )
    }
}
