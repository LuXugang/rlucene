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
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::util::error::lucene_error::Result;

pub trait TwoPhaseIterator {
  type ApproximationRef<'a>: DocIdSetIterator
  where
    Self: 'a;
  type ApproximationMut<'a>: DocIdSetIterator
  where
    Self: 'a;
  /// Return the approximation [`DocIdSetIterator`].
  ///
  /// The returned iterator must advance synchronously with this
  /// [`TwoPhaseIterator`].
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_>;
  fn approximation(&self) -> Self::ApproximationRef<'_>;

  /// Return whether the current doc ID that `approximation()` is on matches.
  ///
  /// This method should only be called when the iterator is positioned
  /// (i.e. not when `doc_id()` is `-1` or [`NO_MORE_DOCS`]) and at most once.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs.
  fn matches(&mut self) -> Result<bool>;

  /// An estimate of the expected cost to determine that a single
  /// document matches.
  ///
  /// This can be called before iterating the documents of
  /// `approximation()`. Returns an expected cost in number of simple
  /// operations (add, multiply, compare, array index). Must be positive.
  fn match_cost(&self) -> f32;
}
#[derive(Default)]
pub struct EmptyTPI {
  approximation: EmptyDISI,
}
impl TwoPhaseIterator for EmptyTPI {
  type ApproximationRef<'a>
    = &'a EmptyDISI
  where
    Self: 'a;
  type ApproximationMut<'a>
    = &'a mut EmptyDISI
  where
    Self: 'a;

  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    // Preserve the fresh empty cursor returned by this placeholder on every access.
    self.approximation = EmptyDISI::new();
    &mut self.approximation
  }

  fn approximation(&self) -> Self::ApproximationRef<'_> {
    static EMPTY: std::sync::LazyLock<EmptyDISI> = std::sync::LazyLock::new(EmptyDISI::new);
    &*EMPTY
  }

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
  type ApproximationRef<'a>
    = <T as TwoPhaseIterator>::ApproximationRef<'a>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = <T as TwoPhaseIterator>::ApproximationMut<'a>
  where
    Self: 'a;

  #[inline]
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    (**self).approximation_mut()
  }

  #[inline]
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    (**self).approximation()
  }

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
  type ApproximationRef<'a>
    = <T as TwoPhaseIterator>::ApproximationRef<'a>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = <T as TwoPhaseIterator>::ApproximationMut<'a>
  where
    Self: 'a;

  #[inline]
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    (**self).approximation_mut()
  }

  #[inline]
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    (**self).approximation()
  }

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
      doc = self.two_phase_iterator.approximation_mut().next_doc()?;
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
    self.two_phase_iterator.approximation().doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let doc = self.two_phase_iterator.approximation_mut().next_doc()?;
    self.do_next(doc)
  }

  fn advance(&mut self, target: i32) -> Result<i32> {
    let doc = self
      .two_phase_iterator
      .approximation_mut()
      .advance(target)?;
    self.do_next(doc)
  }

  fn cost(&self) -> Result<i64> {
    self.two_phase_iterator.approximation().cost()
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

        impl<$( $T ),+> TwoPhaseIterator for $name<$( $T ),+>
        where
            $( $T: TwoPhaseIterator ),+
        {
            type ApproximationRef<'a> = crate::core::search::doc_id_set_iterator::$iter<$( $T::ApproximationRef<'a> ),+> where Self: 'a;
            type ApproximationMut<'a> = crate::core::search::doc_id_set_iterator::$iter<$( $T::ApproximationMut<'a> ),+> where Self: 'a;

            #[inline]
            fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
                match self {
                    $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::$iter::$Variant(inner.approximation_mut()), )+
                }
            }

            #[inline]
            fn approximation(&self) -> Self::ApproximationRef<'_> {
                match self {
                    $( Self::$Variant(inner) => crate::core::search::doc_id_set_iterator::$iter::$Variant(inner.approximation()), )+
                }
            }

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
impl<T: TwoPhaseIterator + ?Sized> TwoPhaseIterator for &T {
  type ApproximationRef<'a>
    = T::ApproximationRef<'a>
  where
    Self: 'a;
  type ApproximationMut<'a>
    = T::ApproximationRef<'a>
  where
    Self: 'a;
  fn approximation(&self) -> Self::ApproximationRef<'_> {
    (**self).approximation()
  }
  fn approximation_mut(&mut self) -> Self::ApproximationMut<'_> {
    (**self).approximation()
  }
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
