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
use crate::core::search::doc_id_stream::{DocIdStream, DocIdStreamConsumer};
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Display;

pub trait LeafCollector: Display {
    /// Called before successive calls to [`LeafCollector::collect`].
    ///
    /// Implementations that need the score of the current document (passed in
    /// to `collect`) should save the passed-in `Scorer`(crate::core::search::scorer::Scorer) and call
    /// `scorer.score()` when needed.
    fn set_scorer(&mut self, _scorer: &mut dyn Scorable) -> Result<()> {
        Ok(())
    }
    /// Called once for every document matching a query, with the unbased document number.
    ///
    /// # Notes
    ///
    /// - The collection of the current segment can be terminated by returning an
    ///   error such as `LuceneError::CollectionTerminated`. In this case, the last
    ///   docs of the current [`LeafReaderContext`](crate::core::index::leaf_reader_context::LeafReaderContext) will be skipped and
    ///   [`IndexSearcher`](crate::core::search::index_searcher::IndexSearcher) will swallow the exception and continue collection with
    ///   the next leaf.
    ///
    /// - This is called in an inner search loop. For good search performance,
    ///   implementations of this method should **not** call
    ///   [`StoredFields::document`](crate::core::index::stored_fields::StoredFields::document) on every hit. Doing so can slow searches by an
    ///   order of magnitude or more.
    fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()>;

    /// Bulk-collect doc IDs.
    ///
    /// # Notes
    ///
    /// - The provided [`DocIdStream`] may be reused across calls and should be
    ///   consumed immediately.
    /// - The provided [`DocIdStream`] typically only holds a small subset of query
    ///   matches. This method may be called multiple times per segment.
    /// - Like [`LeafCollector::collect`], it is guaranteed that doc IDs get
    ///   collected in order. Doc IDs are collected in order within a
    ///   [`DocIdStream`], and if called twice, all doc IDs from the second
    ///   [`DocIdStream`] will be greater than all doc IDs from the first
    ///   [`DocIdStream`].
    /// - It is legal for callers to mix calls to
    ///   [`LeafCollector::collect_stream`] and [`LeafCollector::collect`].
    ///
    /// # Default
    ///
    /// The default implementation calls `stream.for_each(|doc| self.collect(doc))`.
    fn collect_stream(&mut self, stream: &mut dyn DocIdStream) -> Result<()> {
        struct CollectorConsumer<'a, LC>
        where
            LC: LeafCollector + ?Sized,
        {
            collector: &'a mut LC,
        }

        impl<'a, LC> DocIdStreamConsumer for CollectorConsumer<'a, LC>
        where
            LC: LeafCollector + ?Sized,
        {
            fn visit(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
                self.collector.collect(doc, scorer)
            }
        }

        let mut consumer = CollectorConsumer { collector: self };
        stream.for_each(&mut consumer)
    }
    /// Optionally returns an iterator over competitive documents.
    ///
    /// Collectors should delegate this method to their comparators if their
    /// comparators provide skipping functionality over non-competitive docs.
    ///
    /// The default is `None`, meaning no competitive iterator is provided.
    fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
        Ok(None)
    }

    /// Hook that gets called once the leaf associated with this collector has
    /// finished collecting successfully, including when a
    /// [`CollectionTerminatedError`](crate::core::util::error::CollectionTerminatedError) is thrown.
    ///
    /// This is typically useful to compile data that has been collected on this
    /// leaf, e.g. to convert facet counts on leaf ordinals to facet counts on
    /// global ordinals.
    ///
    /// The default implementation does nothing.
    ///
    /// # Notes
    ///
    /// - It can be assumed that this method will only be called once per
    ///   [`LeafCollector`] instance.
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}
impl<T> LeafCollector for &mut T
where
    T: LeafCollector + ?Sized,
{
    fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
        (**self).set_scorer(scorer)
    }

    fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
        (**self).collect(doc, scorer)
    }

    fn collect_stream(&mut self, stream: &mut dyn DocIdStream) -> Result<()> {
        (**self).collect_stream(stream)
    }

    fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
        (**self).competitive_iterator()
    }

    fn finish(&mut self) -> Result<()> {
        (**self).finish()
    }
}

macro_rules! either_leaf_collector {
    (
        $vis:vis $name:ident
        { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Display for $name<$( $T ),+>
        where
            $( $T: LeafCollector ),+
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$Variant(inner) => Display::fmt(inner, f), )+
                }
            }
        }

        impl<$( $T ),+> LeafCollector for $name<$( $T ),+>
        where
            $( $T: LeafCollector ),+
        {
            fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.set_scorer(scorer), )+
                }
            }

            fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.collect(doc, scorer), )+
                }
            }

            fn collect_stream(&mut self, stream: &mut dyn DocIdStream) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.collect_stream(stream), )+
                }
            }

            fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
                match self {
                    $( Self::$Variant(inner) => inner.competitive_iterator(), )+
                }
            }

            fn finish(&mut self) -> Result<()> {
                match self {
                    $( Self::$Variant(inner) => inner.finish(), )+
                }
            }
        }
    };
}

either_leaf_collector!(
    pub LeafCollectorEnum2
    { A: A, B: B }
);
