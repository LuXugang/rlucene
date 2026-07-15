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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::simple_collector::SimpleCollector;
use crate::core::search::weight::Weight;
use crate::core::util::bit_set::BitSet;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use std::fmt::{Display, Formatter};
/// Collector that accumulates matching docs in a FixedBitSet
pub struct FixedBitSetCollector {
  bit_set: FixedBitSet,
  doc_base: i32,
}

impl FixedBitSetCollector {
  pub fn new(max_doc: i32) -> Self {
    Self {
      bit_set: FixedBitSet::new(max_doc as usize),
      doc_base: 0,
    }
  }

  pub fn create_manager(max_doc: i32) -> FixedBitSetCollectorManager {
    FixedBitSetCollectorManager { max_doc }
  }
}

impl Collector for FixedBitSetCollector {
  type LeafCollector<'a, IRC>
    = &'a mut Self
  where
    Self: 'a,
    IRC: IndexReaderContext + 'a;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    SimpleCollector::do_set_next_reader(self, context)?;
    Ok(self)
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::CompleteNoScores
  }
}

impl LeafCollector for FixedBitSetCollector {
  fn collect(&mut self, doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    self.bit_set.set((self.doc_base + doc) as usize);
    Ok(())
  }
}

impl SimpleCollector for FixedBitSetCollector {
  fn do_set_next_reader<LR>(&mut self, context: &LeafReaderContext<LR>) -> Result<()>
  where
    LR: LeafReader,
  {
    self.doc_base = context.doc_base as i32;
    Ok(())
  }
}

impl Display for FixedBitSetCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

pub struct FixedBitSetCollectorManager {
  max_doc: i32,
}

impl CollectorManager for FixedBitSetCollectorManager {
  type C = FixedBitSetCollector;
  type T = FixedBitSet;

  fn new_collector(&self) -> Result<Self::C> {
    Ok(FixedBitSetCollector::new(self.max_doc))
  }

  fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
    let mut reduced = FixedBitSet::new(self.max_doc as usize);

    for collector in collectors {
      reduced.or(&collector.bit_set);
    }

    Ok(reduced)
  }
}
