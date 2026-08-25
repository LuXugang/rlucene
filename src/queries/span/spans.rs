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
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;
use crate::queries::span::span_collector::SpanCollector;

/// Iterates through combinations of start/end positions per-doc.
/// Each start/end position represents a range of term positions within
/// the current document. These are enumerated in order, by increasing
/// document number, within that by increasing start position and finally
/// by increasing end position.
pub trait Spans: DocIdSetIterator {
  /// Returns the next start position for the current doc.
  /// There is always at least one start/end position per doc.
  /// After the last start/end position at the current doc this returns
  /// [`NO_MORE_POSITIONS`].
  fn next_start_position(&mut self) -> Result<i32>;

  /// Returns the start position in the current doc, or -1 when
  /// [`next_start_position`](Spans::next_start_position) was not yet
  /// called on the current doc. After the last start/end position at the
  /// current doc this returns
  /// [`NO_MORE_POSITIONS`].
  fn start_position(&self) -> i32;

  /// Returns the end position for the current start position, or -1 when
  /// [`next_start_position`](Spans::next_start_position) was not yet
  /// called on the current doc. After the last start/end position at the
  /// current doc this returns
  /// [`NO_MORE_POSITIONS`].
  fn end_position(&self) -> i32;

  /// Return the width of the match, which is typically used to sloppy
  /// freq. It is only legal to call this method when the iterator is on a
  /// valid doc ID and positioned. The return value must be positive, and
  /// lower values means that the match is better.
  fn width(&self) -> i32;

  /// Collect postings data from the leaves of the current Spans.
  ///
  /// This method should only be called after
  /// [`next_start_position`](Spans::next_start_position), and before
  /// [`NO_MORE_POSITIONS`] has been reached.
  ///
  /// * `collector` a SpanCollector
  fn collect(&self, collector: &mut impl SpanCollector) -> Result<()>;

  /// Return an estimation of the cost of using the positions of this
  /// [`Spans`] for any single document, but only after
  /// [`as_two_phase_iterator`](Spans::as_two_phase_iterator) returned
  /// `None`. Otherwise this method should not be called. The returned
  /// value is independent of the current document.
  ///
  /// @lucene.experimental
  fn positions_cost(&self) -> f32;

  /// Optional method: Return a [`TwoPhaseIterator`] view of this
  /// [`Scorer`](crate::core::search::scorer::Scorer). A return value of
  /// `None` indicates that two-phase iteration is not supported.
  ///
  /// See [`Scorer::two_phase_iterator`](crate::core::search::scorer::Scorer::two_phase_iterator).
  fn as_two_phase_iterator(&self) -> Option<Box<dyn TwoPhaseIterator>> {
    None
  }

  /// Called before the current doc's frequency is calculated
  fn do_start_current_doc(&mut self) -> Result<()> {
    Ok(())
  }

  /// Called each time the scorer's SpanScorer is advanced during
  /// frequency calculation
  fn do_current_spans(&mut self) -> Result<()> {
    Ok(())
  }
}
pub const NO_MORE_POSITIONS: i32 = i32::MAX;
