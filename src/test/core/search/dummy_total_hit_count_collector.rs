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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;

use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

pub struct DummyTotalHitCountCollector {
  total_hits: i32,
}
impl Default for DummyTotalHitCountCollector {
  fn default() -> Self {
    Self::new()
  }
}

impl DummyTotalHitCountCollector {
  pub fn new() -> Self {
    Self { total_hits: 0 }
  }
  /// Get the number of hits.
  pub fn get_total_hits(&self) -> i32 {
    self.total_hits
  }
  pub fn create_manager() -> CollectorManagerImpl {
    CollectorManagerImpl
  }
}
impl Collector for DummyTotalHitCountCollector {
  type LeafCollector<'a, IRC>
    = DummyLeafCollectorImpl<'a>
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(DummyLeafCollectorImpl::new(self))
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::CompleteNoScores
  }
}
pub struct DummyLeafCollectorImpl<'a> {
  base: &'a mut DummyTotalHitCountCollector,
}
impl<'a> DummyLeafCollectorImpl<'a> {
  pub fn new(base: &'a mut DummyTotalHitCountCollector) -> Self {
    Self { base }
  }
}

impl Display for DummyLeafCollectorImpl<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "LeafCollectorImpl")
  }
}

impl<'a> LeafCollector for DummyLeafCollectorImpl<'a> {
  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    self.base.total_hits += 1;
    Ok(())
  }
}

/// Create a collector manager.
pub struct CollectorManagerImpl;
impl Default for CollectorManagerImpl {
  fn default() -> Self {
    Self::new()
  }
}

impl CollectorManagerImpl {
  pub fn new() -> Self {
    Self {}
  }
}
impl CollectorManager for CollectorManagerImpl {
  type C = DummyTotalHitCountCollector;
  type T = i32;

  fn new_collector(&self) -> Result<Self::C> {
    Ok(DummyTotalHitCountCollector::new())
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    let mut sum = 0;
    for coll in collectors {
      sum += coll.total_hits;
    }
    Ok(sum)
  }
}
