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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::doc_id_stream::DocIdStream;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::asserting_leaf_collector::AssertingLeafCollector;
use std::cell::Cell;
use std::fmt::{Display, Formatter};

/// A collector that asserts that it is used correctly.
pub(crate) struct AssertingCollector<'a, C> {
  in_: &'a mut C,
  weight_set: Cell<bool>,
  max_doc: i32,
  previous_leaf_max_doc: i32,
  // public visibility for drill-sideways testing, since drill-sideways can't directly use
  // AssertingIndexSearcher
  pub(crate) has_finished_collecting_previous_leaf: bool,
}

impl<'a, C> AssertingCollector<'a, C> {
  /// Wrap the given collector in order to add assertions.
  pub(crate) fn wrap(in_: &'a mut C) -> Self {
    Self {
      in_,
      weight_set: Cell::new(false),
      max_doc: -1,
      previous_leaf_max_doc: 0,
      has_finished_collecting_previous_leaf: true,
    }
  }
}

impl<C> Collector for AssertingCollector<'_, C>
where
  C: Collector,
{
  type LeafCollector<'a, IRC>
    = AssertingCollectorLeafCollector<'a, C::LeafCollector<'a, IRC>>
  where
    Self: 'a,
    IRC: IndexReaderContext + 'a;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    assert!(self.weight_set.get(), "Set the weight first");
    assert!(context.doc_base <= i32::MAX as usize);
    let doc_base = context.doc_base as i32;
    assert!(doc_base >= self.previous_leaf_max_doc);
    self.previous_leaf_max_doc = doc_base + context.reader().max_doc()?;

    assert!(self.has_finished_collecting_previous_leaf);
    let Self {
      in_,
      max_doc,
      has_finished_collecting_previous_leaf,
      ..
    } = self;
    let leaf_collector = in_.get_leaf_collector(context, weight, searcher)?;
    *has_finished_collecting_previous_leaf = false;
    Ok(AssertingCollectorLeafCollector {
      in_: AssertingLeafCollector::new(leaf_collector, 0, NO_MORE_DOCS),
      doc_base,
      max_doc,
      has_finished_collecting_previous_leaf,
    })
  }

  fn score_mode(&self) -> ScoreMode {
    self.in_.score_mode()
  }

  fn set_weight<W, IRC>(&self, weight: Option<&W>) -> Result<()>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    assert!(!self.weight_set.replace(true), "Weight set twice");
    assert!(weight.is_some());
    self.in_.set_weight(weight)
  }
}

pub(crate) struct AssertingCollectorLeafCollector<'a, L> {
  in_: AssertingLeafCollector<L>,
  doc_base: i32,
  max_doc: &'a mut i32,
  has_finished_collecting_previous_leaf: &'a mut bool,
}

impl<L> Display for AssertingCollectorLeafCollector<'_, L>
where
  L: LeafCollector,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.in_, f)
  }
}

impl<L> LeafCollector for AssertingCollectorLeafCollector<'_, L>
where
  L: LeafCollector,
{
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.in_.set_scorer(scorer)
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    assert!(
      self.doc_base + doc >= *self.max_doc,
      "collection is not in order: current doc={} while {} has already been collected",
      self.doc_base + doc,
      self.max_doc
    );

    self.in_.collect(doc, scorer)?;
    *self.max_doc = self.doc_base + doc;
    Ok(())
  }

  fn collect_stream(
    &mut self,
    stream: &mut dyn DocIdStream,
    scorer: &mut dyn Scorable,
  ) -> Result<()> {
    self.default_collect_stream(stream, scorer)
  }

  fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
    self.in_.competitive_iterator()
  }

  fn finish(&mut self) -> Result<()> {
    *self.has_finished_collecting_previous_leaf = true;
    self.in_.finish()
  }
}
