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
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::search::asserting_scorable::AssertingScorable;
use std::fmt::{Display, Formatter};

/// Wraps another Collector and checks that order is respected.
pub(crate) struct AssertingLeafCollector<'a> {
  in_: &'a mut dyn LeafCollector,
  min: i32,
  max: i32,
  last_collected: i32,
  finish_called: bool,
}

impl<'a> AssertingLeafCollector<'a> {
  pub(crate) fn new(in_: &'a mut dyn LeafCollector, min: i32, max: i32) -> Self {
    Self {
      in_,
      min,
      max,
      last_collected: -1,
      finish_called: false,
    }
  }
}

impl Display for AssertingLeafCollector<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "AssertingLeafCollector({})", self.in_)
  }
}

impl LeafCollector for AssertingLeafCollector<'_> {
  fn set_scorer(&mut self, scorer: &mut dyn Scorable) -> Result<()> {
    self.in_.set_scorer(&mut AssertingScorable::wrap(scorer))
  }

  fn collect(&mut self, doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    assert!(
      doc > self.last_collected,
      "Out of order : {} {}",
      self.last_collected,
      doc
    );
    assert!(doc >= self.min, "Out of range: {} < {}", doc, self.min);
    assert!(doc < self.max, "Out of range: {} >= {}", doc, self.max);
    self
      .in_
      .collect(doc, &mut AssertingScorable::wrap(scorer))?;
    self.last_collected = doc;
    Ok(())
  }

  fn collect_stream(
    &mut self,
    stream: &mut dyn DocIdStream,
    scorer: &mut dyn Scorable,
  ) -> Result<()> {
    let mut asserting_stream =
      AssertingDocIdStream::new(stream, &mut self.last_collected, self.min, self.max);
    let mut asserting_scorable = AssertingScorable::wrap(scorer);
    self
      .in_
      .collect_stream(&mut asserting_stream, &mut asserting_scorable)
  }

  fn competitive_iterator(&mut self) -> Result<Option<Box<dyn DocIdSetIterator + '_>>> {
    Ok(self.in_.competitive_iterator()?.map(|in_| {
      Box::new(AssertingCompetitiveIterator { in_, max: self.max }) as Box<dyn DocIdSetIterator>
    }))
  }

  fn finish(&mut self) -> Result<()> {
    assert!(!self.finish_called);
    self.finish_called = true;
    self.in_.finish()
  }
}

struct AssertingCompetitiveIterator<'a> {
  in_: Box<dyn DocIdSetIterator + 'a>,
  max: i32,
}

impl DocIdSetIterator for AssertingCompetitiveIterator<'_> {
  fn doc_id(&self) -> i32 {
    self.in_.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    assert!(
      self.in_.doc_id() < self.max,
      "advancing beyond the end of the scored window: docID={}, max={}",
      self.in_.doc_id(),
      self.max
    );
    self.in_.next_doc()
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    assert!(
      target <= self.max,
      "advancing beyond the end of the scored window: target={}, max={}",
      target,
      self.max
    );
    self.in_.advance(target)
  }

  fn cost(&self) -> Result<i64> {
    self.in_.cost()
  }
}

struct AssertingDocIdStream<'a> {
  stream: &'a mut dyn DocIdStream,
  last_collected: &'a mut i32,
  min: i32,
  max: i32,
  consumed: bool,
}

impl<'a> AssertingDocIdStream<'a> {
  fn new(stream: &'a mut dyn DocIdStream, last_collected: &'a mut i32, min: i32, max: i32) -> Self {
    Self {
      stream,
      last_collected,
      min,
      max,
      consumed: false,
    }
  }
}

impl DocIdStream for AssertingDocIdStream<'_> {
  fn for_each(&mut self, consumer: &mut dyn DocIdStreamConsumer) -> Result<()> {
    assert!(
      !self.consumed,
      "A terminal operation has already been called"
    );
    let mut asserting_consumer = AssertingDocIdStreamConsumer {
      consumer,
      last_collected: self.last_collected,
      min: self.min,
      max: self.max,
    };
    self.stream.for_each(&mut asserting_consumer)?;
    self.consumed = true;
    Ok(())
  }

  fn count(&mut self, scorer: &mut dyn Scorable) -> Result<i32> {
    assert!(
      !self.consumed,
      "A terminal operation has already been called"
    );
    let count = self.stream.count(&mut AssertingScorable::wrap(scorer))?;
    self.consumed = true;
    Ok(count)
  }
}

struct AssertingDocIdStreamConsumer<'a> {
  consumer: &'a mut dyn DocIdStreamConsumer,
  last_collected: &'a mut i32,
  min: i32,
  max: i32,
}

impl AssertingDocIdStreamConsumer<'_> {
  fn check_doc(&self, doc: i32) {
    assert!(
      doc > *self.last_collected,
      "Out of order : {} {}",
      self.last_collected,
      doc
    );
    assert!(doc >= self.min, "Out of range: {} < {}", doc, self.min);
    assert!(doc < self.max, "Out of range: {} >= {}", doc, self.max);
  }
}

impl DocIdStreamConsumer for AssertingDocIdStreamConsumer<'_> {
  fn accept(&mut self, doc: i32) -> Result<()> {
    self.check_doc(doc);
    self.consumer.accept(doc)?;
    *self.last_collected = doc;
    Ok(())
  }

  fn accept_with_score(&mut self, doc: i32, score: f32) -> Result<()> {
    self.check_doc(doc);
    self.consumer.accept_with_score(doc, score)?;
    *self.last_collected = doc;
    Ok(())
  }
}
