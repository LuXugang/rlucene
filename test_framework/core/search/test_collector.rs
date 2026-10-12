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
//! Heterogeneous fixtures for unit tests; excluded from production builds.
use crate::core::search::bulk_scorer::BulkScorer;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_stream::{DocIdStream, DocIdStreamConsumer};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Display;

pub trait TestLeafCollector: Display {
  fn erased_set_scorer(
    &mut self,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<()>;
  fn erased_collect(
    &mut self,
    doc: i32,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<()>;
  fn erased_collect_stream(
    &mut self,
    stream: &mut dyn TestDocIdStream,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<()>;
  fn erased_finish(&mut self) -> Result<()>;
  fn erased_competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>>;
}
impl<LC: LeafCollector> TestLeafCollector for LC {
  fn erased_set_scorer(
    &mut self,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<()> {
    self.set_scorer(scorer)
  }
  fn erased_collect(
    &mut self,
    doc: i32,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<()> {
    self.collect(doc, scorer)
  }
  fn erased_collect_stream(
    &mut self,
    stream: &mut dyn TestDocIdStream,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<()> {
    self.collect_stream(stream, scorer)
  }
  fn erased_finish(&mut self) -> Result<()> {
    self.finish()
  }
  fn erased_competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
    Ok(
      self
        .competitive_iterator()?
        .map(|iterator| Box::new(iterator) as Box<dyn DocIdSetIterator>),
    )
  }
}
impl LeafCollector for dyn TestLeafCollector + '_ {
  type CompetitiveIterator<'a>
    = Box<dyn DocIdSetIterator + 'a>
  where
    Self: 'a;
  fn set_scorer<SC: Scorable + ?Sized>(&mut self, mut scorer: &mut SC) -> Result<()> {
    self.erased_set_scorer(&mut scorer)
  }
  fn collect<SC: Scorable + ?Sized>(&mut self, doc: i32, mut scorer: &mut SC) -> Result<()> {
    self.erased_collect(doc, &mut scorer)
  }
  fn collect_stream<DS: DocIdStream + ?Sized, SC: Scorable + ?Sized>(
    &mut self,
    mut stream: &mut DS,
    mut scorer: &mut SC,
  ) -> Result<()> {
    self.erased_collect_stream(&mut stream, &mut scorer)
  }
  fn finish(&mut self) -> Result<()> {
    self.erased_finish()
  }
  fn competitive_iterator(&mut self) -> Result<Option<Self::CompetitiveIterator<'_>>> {
    self.erased_competitive_iterator()
  }
}

pub trait TestBulkScorer {
  fn erased_score(
    &mut self,
    collector: &mut dyn TestLeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32>;
  fn erased_cost(&mut self) -> Result<i64>;
  #[cfg(test)]
  fn erased_kind(&self) -> crate::core::search::bulk_scorer::BulkScorerKind;
}
impl<BS: BulkScorer> TestBulkScorer for BS {
  fn erased_score(
    &mut self,
    mut collector: &mut dyn TestLeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    self.score(&mut collector, accept_docs, min, max)
  }
  fn erased_cost(&mut self) -> Result<i64> {
    self.cost()
  }
  #[cfg(test)]
  fn erased_kind(&self) -> crate::core::search::bulk_scorer::BulkScorerKind {
    self.kind()
  }
}
impl BulkScorer for dyn TestBulkScorer + '_ {
  fn score<LC: LeafCollector, AB: Bits + ?Sized>(
    &mut self,
    collector: &mut LC,
    accept_docs: Option<&AB>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    self.erased_score(
      collector,
      accept_docs.as_ref().map(|bits| bits as &dyn Bits),
      min,
      max,
    )
  }
  fn cost(&mut self) -> Result<i64> {
    self.erased_cost()
  }
  #[cfg(test)]
  fn kind(&self) -> crate::core::search::bulk_scorer::BulkScorerKind {
    self.erased_kind()
  }
}

/// Object-safe stream adapter for heterogeneous test fixtures, absent from production builds.
pub trait TestDocIdStream {
  fn erased_for_each(&mut self, consumer: &mut dyn DocIdStreamConsumer) -> Result<()>;
  fn erased_count(
    &mut self,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<i32>;
}
impl<DS: DocIdStream> TestDocIdStream for DS {
  fn erased_for_each(&mut self, consumer: &mut dyn DocIdStreamConsumer) -> Result<()> {
    self.for_each(consumer)
  }
  fn erased_count(
    &mut self,
    scorer: &mut dyn crate::test_framework::core::search::test_scorer::TestScorable,
  ) -> Result<i32> {
    self.count(scorer)
  }
}
impl DocIdStream for dyn TestDocIdStream + '_ {
  fn for_each<DC: DocIdStreamConsumer + ?Sized>(&mut self, mut consumer: &mut DC) -> Result<()> {
    self.erased_for_each(&mut consumer)
  }
  fn count<SC: Scorable + ?Sized>(&mut self, mut scorer: &mut SC) -> Result<i32> {
    self.erased_count(&mut scorer)
  }
}
