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
use crate::core::search::approximation_disi::ApproximationBitSetAccess;
use crate::core::search::approximation_disi::ApproximationDISI;
use crate::core::search::doc_id_set_iterator::BitSetIteratorAccess;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::util::error::lucene_error::Result;

/// Candidate advancement and confirmation on one owner.
///
/// Unlike full matching iteration, `approx_*` operations may stop on a document
/// that `matches` rejects. Scorers without two-phase matching return `Ok(true)`
/// from `matches` and `0.0` from `match_cost`.
pub trait TwoPhaseIterator: ApproximationDISI + ApproximationBitSetAccess {
  /// Return whether the current doc ID that `approx_doc_id()` identifies matches.
  ///
  /// This method should only be called when the iterator is positioned
  /// (i.e. not when `approx_doc_id()` is `-1` or [`NO_MORE_DOCS`]) and at most once.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs.
  fn matches(&mut self) -> Result<bool>;

  /// An estimate of the expected cost to determine that a single
  /// document matches.
  ///
  /// This can be called before iterating the documents of
  /// the approximation. Returns an expected cost in number of simple
  /// operations (add, multiply, compare, array index). Positive for two-phase
  /// matching; zero when no confirmation is needed.
  fn match_cost(&self) -> f32;
}
#[derive(Default)]
pub struct EmptyTPI {
  approximation: EmptyDISI,
}
impl ApproximationDISI for EmptyTPI {
  fn approx_doc_id(&self) -> i32 {
    ({
      static EMPTY: std::sync::LazyLock<EmptyDISI> = std::sync::LazyLock::new(EmptyDISI::new);
      &*EMPTY
    })
    .doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    ({
      // Preserve the fresh empty cursor returned by this placeholder on every access.
      self.approximation = EmptyDISI::new();
      &mut self.approximation
    })
    .next_doc()
  }

  fn approx_advance(&mut self, target: i32) -> Result<i32> {
    ({
      // Preserve the fresh empty cursor returned by this placeholder on every access.
      self.approximation = EmptyDISI::new();
      &mut self.approximation
    })
    .advance(target)
  }

  fn approx_slow_advance(&mut self, target: i32) -> Result<i32> {
    ({
      // Preserve the fresh empty cursor returned by this placeholder on every access.
      self.approximation = EmptyDISI::new();
      &mut self.approximation
    })
    .slow_advance(target)
  }

  fn approx_cost(&self) -> Result<i64> {
    ({
      static EMPTY: std::sync::LazyLock<EmptyDISI> = std::sync::LazyLock::new(EmptyDISI::new);
      &*EMPTY
    })
    .cost()
  }
}
impl ApproximationBitSetAccess for EmptyTPI {
  fn approx_is_bit_iter(&self) -> bool {
    ({
      static EMPTY: std::sync::LazyLock<EmptyDISI> = std::sync::LazyLock::new(EmptyDISI::new);
      &*EMPTY
    })
    .is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    ({
      static EMPTY: std::sync::LazyLock<EmptyDISI> = std::sync::LazyLock::new(EmptyDISI::new);
      &*EMPTY
    })
    .get(index)
  }

  fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> {
    ({
      // Preserve the fresh empty cursor returned by this placeholder on every access.
      self.approximation = EmptyDISI::new();
      &mut self.approximation
    })
    .set_doc_id(doc)
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    ({
      static EMPTY: std::sync::LazyLock<EmptyDISI> = std::sync::LazyLock::new(EmptyDISI::new);
      &*EMPTY
    })
    .bit_set_length()
  }
}

impl TwoPhaseIterator for EmptyTPI {
  fn matches(&mut self) -> Result<bool> {
    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    0f32
  }
}

impl<T> TwoPhaseIterator for &mut T
where
  T: TwoPhaseIterator + ?Sized,
{
  #[inline]
  fn matches(&mut self) -> Result<bool> {
    (**self).matches()
  }

  #[inline]
  fn match_cost(&self) -> f32 {
    (**self).match_cost()
  }
}

impl<T> TwoPhaseIterator for Box<T>
where
  T: TwoPhaseIterator + ?Sized,
{
  #[inline]
  fn matches(&mut self) -> Result<bool> {
    (**self).matches()
  }

  #[inline]
  fn match_cost(&self) -> f32 {
    (**self).match_cost()
  }
}

pub struct TwoPhaseIteratorAsDocIdSetIterator<TPI> {
  pub(crate) two_phase_iterator: TPI,
}

impl<TPI> TwoPhaseIteratorAsDocIdSetIterator<TPI> {
  pub fn new(two_phase_iterator: TPI) -> Self {
    Self { two_phase_iterator }
  }
}

impl<TPI> TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn do_next(&mut self, mut doc: i32) -> Result<i32> {
    loop {
      if doc == NO_MORE_DOCS {
        return Ok(NO_MORE_DOCS);
      } else if self.two_phase_iterator.matches()? {
        return Ok(doc);
      }
      doc = self.two_phase_iterator.approx_next_doc()?;
    }
  }
}

impl<TPI> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions
  for TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
  TPI: TwoPhaseIterator,
{
}
impl<TPI> crate::core::search::doc_id_set_iterator::BitSetIteratorAccess
  for TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
  TPI: TwoPhaseIterator,
{
}

impl<TPI> DocIdSetIterator for TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
  TPI: TwoPhaseIterator,
{
  fn doc_id(&self) -> i32 {
    self.two_phase_iterator.approx_doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let doc = self.two_phase_iterator.approx_next_doc()?;
    self.do_next(doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let doc = self.two_phase_iterator.approx_advance(target)?;
    self.do_next(doc)
  }

  fn cost(&self) -> Result<i64> {
    self.two_phase_iterator.approx_cost()
  }
}
/// Return a DocIdSetIterator view of the provided TwoPhaseIterator.
pub fn as_doc_id_set_iterator<TPI>(tpi: TPI) -> TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
  TPI: TwoPhaseIterator,
{
  TwoPhaseIteratorAsDocIdSetIterator::new(tpi)
}

pub fn unwrap<TPI>(tp: TwoPhaseIteratorAsDocIdSetIterator<TPI>) -> TPI
where
  TPI: TwoPhaseIterator,
{
  tp.two_phase_iterator
}

macro_rules! either_two_phase_iterator_gat {
    (
        $vis:vis $name:ident, $iter:ident
        { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> ApproximationDISI for $name<$( $T ),+>
        where
            $( $T: TwoPhaseIterator ),+
        {
fn approx_doc_id(&self) -> i32 { match self { $( Self::$Variant(inner) => inner.approx_doc_id(), )+ } }

fn approx_next_doc(&mut self) -> Result<i32> { match self { $( Self::$Variant(inner) => inner.approx_next_doc(), )+ } }

fn approx_advance(&mut self, target: i32) -> Result<i32> { match self { $( Self::$Variant(inner) => inner.approx_advance(target), )+ } }

fn approx_slow_advance(&mut self, target: i32) -> Result<i32> { match self { $( Self::$Variant(inner) => inner.approx_slow_advance(target), )+ } }

fn approx_cost(&self) -> Result<i64> { match self { $( Self::$Variant(inner) => inner.approx_cost(), )+ } }
}

        impl<$( $T ),+> ApproximationBitSetAccess for $name<$( $T ),+>
        where
            $( $T: TwoPhaseIterator ),+
        {
fn approx_is_bit_iter(&self) -> bool { match self { $( Self::$Variant(inner) => inner.approx_is_bit_iter(), )+ } }

fn approx_get(&self, index: usize) -> Result<bool> { match self { $( Self::$Variant(inner) => inner.approx_get(index), )+ } }

fn approx_set_doc_id(&mut self, doc: i32) -> Result<()> { match self { $( Self::$Variant(inner) => inner.approx_set_doc_id(doc), )+ } }

fn approx_bit_set_length(&self) -> Result<usize> { match self { $( Self::$Variant(inner) => inner.approx_bit_set_length(), )+ } }
}


        impl<$( $T ),+> TwoPhaseIterator for $name<$( $T ),+>
        where
            $( $T: TwoPhaseIterator ),+
        {




            #[inline]
            fn matches(&mut self) -> Result<bool> {
                match self {
                    $( Self::$Variant(inner) => inner.matches(), )+
                }
            }

            #[inline]
            fn match_cost(&self) -> f32 {
                match self {
                    $( Self::$Variant(inner) => inner.match_cost(), )+
                }
            }
        }

    };
}
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum2, DocIdSetIteratorEnum2
    { A: A, B: B}
);

either_two_phase_iterator_gat!(pub TwoPhaseIteratorEnum3, DocIdSetIteratorEnum3 { A: A, B: B, C: C });
either_two_phase_iterator_gat!(pub TwoPhaseIteratorEnum4, DocIdSetIteratorEnum4 { A: A, B: B, C: C, D: D });
impl<T: TwoPhaseIterator + ?Sized> ApproximationDISI for &T {
  fn approx_doc_id(&self) -> i32 {
    (**self).approx_doc_id()
  }

  fn approx_next_doc(&mut self) -> Result<i32> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "immutable two-phase view",
      ),
    )
  }

  fn approx_advance(&mut self, _target: i32) -> Result<i32> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "immutable two-phase view",
      ),
    )
  }

  fn approx_slow_advance(&mut self, _target: i32) -> Result<i32> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "immutable two-phase view",
      ),
    )
  }

  fn approx_cost(&self) -> Result<i64> {
    (**self).approx_cost()
  }
}
impl<T: TwoPhaseIterator + ?Sized> ApproximationBitSetAccess for &T {
  fn approx_is_bit_iter(&self) -> bool {
    (**self).approx_is_bit_iter()
  }

  fn approx_get(&self, index: usize) -> Result<bool> {
    (**self).approx_get(index)
  }

  fn approx_set_doc_id(&mut self, _doc: i32) -> Result<()> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "immutable two-phase view",
      ),
    )
  }

  fn approx_bit_set_length(&self) -> Result<usize> {
    (**self).approx_bit_set_length()
  }
}

impl<T: TwoPhaseIterator + ?Sized> TwoPhaseIterator for &T {
  fn matches(&mut self) -> Result<bool> {
    Err(
      crate::core::util::error::lucene_error::LuceneError::unsupported_operation(
        "immutable two-phase view",
      ),
    )
  }
  fn match_cost(&self) -> f32 {
    (**self).match_cost()
  }
}
