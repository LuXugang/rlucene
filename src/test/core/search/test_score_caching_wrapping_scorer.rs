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
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::score_caching_wrapping_scorer::ScoreCachingWrappingLeafCollector;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use std::cell::Cell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

#[allow(dead_code)] // for quick search
struct TestScoreCachingWrappingScorer;

struct SimpleScorer {
  idx: usize,
  doc: Rc<Cell<i32>>,
}

impl SimpleScorer {
  fn new() -> Self {
    Self {
      idx: 0,
      doc: Rc::new(Cell::new(-1)),
    }
  }
}

impl Scorable for SimpleScorer {
  fn score(&mut self) -> Result<f32> {
    // advance idx on purpose, so that consecutive calls to score will get
    // different results. This is to emulate computation of a score. If
    // ScoreCachingWrappingScorer is used, this should not be called more than
    // once per document.
    let score = if self.idx == SCORES.len() {
      f32::NAN
    } else {
      let score = SCORES[self.idx];
      self.idx += 1;
      score
    };
    Ok(score)
  }

  fn cost(&self) -> Result<i64> {
    Ok(SCORES.len() as i64)
  }
}

impl FixedScore for SimpleScorer {}

impl Scorer for SimpleScorer {
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.doc.get())
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(SimpleDocIdSetIterator::new(self.doc.clone()))
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(SimpleDocIdSetIterator::new(self.doc.clone()))
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    let SimpleScorer { doc, .. } = *self;
    Box::new(SimpleDocIdSetIterator::new(doc))
  }

  fn get_max_score(&mut self, _up_to: i32) -> Result<f32> {
    Ok(f32::INFINITY)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    TwoPhaseState::No
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(SimpleDocIdSetIterator::new(self.doc.clone()))
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(SimpleDocIdSetIterator::new(self.doc.clone()))
  }
}

struct SimpleDocIdSetIterator {
  doc: Rc<Cell<i32>>,
}

impl SimpleDocIdSetIterator {
  fn new(doc: Rc<Cell<i32>>) -> Self {
    Self { doc }
  }
}

impl crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for SimpleDocIdSetIterator
{
}
impl DocIdSetIterator for SimpleDocIdSetIterator {
  fn doc_id(&self) -> i32 {
    self.doc.get()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let doc = self.doc.get() + 1;
    self.doc.set(doc);
    if doc < SCORES.len() as i32 {
      Ok(doc)
    } else {
      Ok(NO_MORE_DOCS)
    }
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    self.doc.set(target);
    if target < SCORES.len() as i32 {
      Ok(target)
    } else {
      Ok(NO_MORE_DOCS)
    }
  }

  fn cost(&self) -> Result<i64> {
    Ok(SCORES.len() as i64)
  }
}

struct ScoreCachingCollector {
  idx: usize,
  mscores: Vec<f32>,
}

impl ScoreCachingCollector {
  fn new(num_to_collect: usize) -> Self {
    Self {
      idx: 0,
      mscores: vec![0.0; num_to_collect],
    }
  }
}

impl Collector for ScoreCachingCollector {
  type LeafCollector<'a, IRC>
    = ScoreCachingWrappingLeafCollector<&'a mut Self>
  where
    Self: 'a,
    IRC: IndexReaderContext + 'a;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _weight: Option<&W>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    Ok(ScoreCachingWrappingLeafCollector::new(self))
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::Complete
  }
}

impl Display for ScoreCachingCollector {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "ScoreCachingCollector")
  }
}

impl LeafCollector for ScoreCachingCollector {
  fn collect(&mut self, _doc: i32, scorer: &mut dyn Scorable) -> Result<()> {
    // just a sanity check to avoid IOOB.
    if self.idx == self.mscores.len() {
      return Ok(());
    }

    // just call score() a couple of times and record the score.
    self.mscores[self.idx] = scorer.score()?;
    self.mscores[self.idx] = scorer.score()?;
    self.mscores[self.idx] = scorer.score()?;
    self.idx += 1;
    Ok(())
  }
}

const SCORES: [f32; 13] = [
  0.7767749f32,
  1.7839992f32,
  8.9925785f32,
  7.9608946f32,
  0.07948637f32,
  2.6356435f32,
  7.4950366f32,
  7.1490803f32,
  8.108544f32,
  4.961808f32,
  2.2423935f32,
  7.285586f32,
  4.6699767f32,
];

#[test]
fn test_get_scores() -> Result<()> {
  let mut s = SimpleScorer::new();
  let mut scc = ScoreCachingCollector::new(SCORES.len());

  {
    let mut lc = ScoreCachingWrappingLeafCollector::new(&mut scc);

    // We need to iterate on the scorer so that its doc() advances.
    loop {
      let doc = {
        let mut it = s.iterator_mut();
        it.next_doc()?
      };
      if doc == NO_MORE_DOCS {
        break;
      }
      lc.collect(doc, &mut s)?;
    }
  }

  for (expected, actual) in SCORES.iter().zip(scc.mscores.iter()) {
    assert_eq!(*expected, *actual);
  }

  Ok(())
}
