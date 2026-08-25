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
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::scorable::{FixedScore, Scorable};
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::queries::span::spans;
use crate::queries::span::spans::Spans;

/// A basic [`Scorer`] over [`Spans`].
///
/// @lucene.experimental
pub struct SpanScorer<S, SS, N> {
  spans: S,
  scorer: Option<SS>,
  norms: Option<N>,
  /// accumulated sloppy freq (computed in setFreqCurrentDoc)
  freq: f32,
  /// last doc we called setFreqCurrentDoc() for
  last_scored_doc: i32,
}

impl<S, SS, N> SpanScorer<S, SS, N> {
  /// Creates a new instance.
  pub fn new(spans: S, scorer: Option<SS>, norms: Option<N>) -> Self {
    SpanScorer {
      spans,
      scorer,
      norms,
      freq: 0.0,
      last_scored_doc: -1,
    }
  }

  /// return the Spans for this Scorer
  pub fn get_spans(&self) -> &S {
    &self.spans
  }
}

impl<S, SS, N> SpanScorer<S, SS, N>
where
  S: Spans,
  SS: SimScorer,
  N: NumericDocValues,
{
  /// Score the current doc. The default implementation scores the doc
  /// with the similarity using a slop-adjusted frequency derived from [`Spans::width`].
  pub fn score_current_doc(&mut self) -> Result<f32> {
    let mut norm: i64 = 1;
    if let Some(ref mut norms) = self.norms
      && norms.advance_exact(self.spans.doc_id())?
    {
      norm = norms.long_value()?;
    }
    let scorer = self
      .scorer
      .as_ref()
      .ok_or_else(|| LuceneError::illegal_state("scorer is None"))?;
    Ok(scorer.score(self.freq, norm))
  }

  /// Sets [`freq`](SpanScorer::freq) for the current document.
  ///
  /// This will be called at most once per document.
  fn set_freq_current_doc(&mut self) -> Result<()> {
    self.freq = 0.0;

    self.spans.do_start_current_doc()?;

    debug_assert_eq!(self.spans.start_position(), -1);
    debug_assert_eq!(self.spans.end_position(), -1);
    let mut prev_start_pos = -1;
    let mut prev_end_pos = -1;

    let mut start_pos = self.spans.next_start_position()?;
    debug_assert_ne!(start_pos, spans::NO_MORE_POSITIONS);
    while start_pos != spans::NO_MORE_POSITIONS {
      debug_assert!(start_pos >= prev_start_pos);
      let end_pos = self.spans.end_position();
      debug_assert_ne!(end_pos, spans::NO_MORE_POSITIONS);
      // This assertion can fail for Or spans on the same term:
      // assert (startPos != prevStartPos) || (endPos > prevEndPos)
      //   : "non increased endPos="+endPos;
      debug_assert!((start_pos != prev_start_pos) || (end_pos >= prev_end_pos));
      if self.scorer.is_none() {
        // scores not required, break out here
        self.freq = 1.0;
        return Ok(());
      }
      self.freq += 1.0 / (1.0 + self.spans.width() as f32);
      self.spans.do_current_spans()?;
      prev_start_pos = start_pos;
      prev_end_pos = end_pos;
      start_pos = self.spans.next_start_position()?;
    }

    debug_assert_eq!(self.spans.start_position(), spans::NO_MORE_POSITIONS);
    debug_assert_eq!(self.spans.end_position(), spans::NO_MORE_POSITIONS);
    Ok(())
  }

  /// Ensure setFreqCurrentDoc is called, if not already called for the
  /// current doc.
  fn ensure_freq(&mut self) -> Result<()> {
    let current_doc = self.spans.doc_id();
    if self.last_scored_doc != current_doc {
      self.set_freq_current_doc()?;
      self.last_scored_doc = current_doc;
    }
    Ok(())
  }

  /// Returns the intermediate "sloppy freq" adjusted for edit distance
  ///
  /// @lucene.internal
  pub fn sloppy_freq(&mut self) -> Result<f32> {
    self.ensure_freq()?;
    Ok(self.freq)
  }
}

impl<S, SS, N> FixedScore for SpanScorer<S, SS, N> {}

impl<S, SS, N> Scorable for SpanScorer<S, SS, N>
where
  S: Spans,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn score(&mut self) -> Result<f32> {
    self.ensure_freq()?;
    self.score_current_doc()
  }

  fn cost(&self) -> Result<i64> {
    self.spans.cost()
  }
}

impl<S, SS, N> Scorer for SpanScorer<S, SS, N>
where
  S: Spans + 'static,
  SS: SimScorer,
  N: NumericDocValues,
{
  fn doc_id(&mut self) -> Result<i32> {
    Ok(self.spans.doc_id())
  }

  fn iterator(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.spans)
  }

  fn iterator_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.spans)
  }

  fn take_iterator(self: Box<Self>) -> Box<dyn DocIdSetIterator> {
    Box::new(self.spans)
  }

  fn two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator + '_>> {
    self.spans.as_two_phase_iterator()
  }

  fn get_max_score(&mut self, _upto: i32) -> Result<f32> {
    Ok(f32::INFINITY)
  }

  fn has_two_phase_iterator(&self) -> TwoPhaseState {
    if self.spans.as_two_phase_iterator().is_some() {
      TwoPhaseState::Yes
    } else {
      TwoPhaseState::No
    }
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    self.iterator()
  }

  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    self.iterator_mut()
  }
}
