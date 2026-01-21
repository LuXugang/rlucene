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
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{
    DocIdSetIterator, DocIdSetIteratorEnum2, DocIdSetIteratorEnum3, DocIdSetIteratorEnum4,
    DocIdSetIteratorEnum5, DocIdSetIteratorEnum6, DocIdSetIteratorEnum7, DocIdSetIteratorEnum8,
    DocIdSetIteratorEnum9, DocIdSetIteratorEnum10, DocIdSetIteratorEnum11,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub trait TwoPhaseIterator {
    type DocIdSetIterator: DocIdSetIterator;
    type DocIdSetIteratorRef<'a>: DocIdSetIterator
    where
        Self: 'a;
    type DocIdSetIteratorMut<'a>: DocIdSetIterator
    where
        Self: 'a;

    /// Return the approximation [`DocIdSetIterator`].
    ///
    /// The returned iterator must advance synchronously with this
    /// `TwoPhaseIterator`.
    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>>;
    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>>;

    /// Set the approximation to an empty iterator
    fn set_empty(&mut self) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    /// Return whether the current doc ID that `approximation()` is on matches.
    ///
    /// This method should only be called when the iterator is positioned
    /// (i.e. not when `doc_id()` is `-1` or `NO_MORE_DOCS`) and at most once.
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

impl<T> TwoPhaseIterator for &mut T
where
    T: TwoPhaseIterator + ?Sized,
{
    type DocIdSetIterator = T::DocIdSetIterator;

    type DocIdSetIteratorRef<'a>
        = T::DocIdSetIteratorRef<'a>
    where
        Self: 'a;

    type DocIdSetIteratorMut<'a>
        = T::DocIdSetIteratorMut<'a>
    where
        Self: 'a;

    #[inline]
    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        (**self).approximation_mut()
    }

    #[inline]
    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        (**self).approximation()
    }

    #[inline]
    fn set_empty(&mut self) -> Result<()> {
        (**self).set_empty()
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
impl<T> TwoPhaseIterator for &T
where
    T: TwoPhaseIterator + ?Sized,
{
    type DocIdSetIterator = T::DocIdSetIterator;

    type DocIdSetIteratorRef<'a>
        = T::DocIdSetIteratorRef<'a>
    where
        Self: 'a;

    type DocIdSetIteratorMut<'a>
        = T::DocIdSetIteratorMut<'a>
    where
        Self: 'a;

    #[inline]
    fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
        Err(LuceneError::unsupported_operation(""))
    }

    #[inline]
    fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
        (**self).approximation()
    }

    #[inline]
    fn set_empty(&mut self) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    #[inline]
    fn matches(&mut self) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }

    #[inline]
    fn match_cost(&self) -> f32 {
        (**self).match_cost()
    }
}
pub struct TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
    TPI: TwoPhaseIterator,
{
    pub(crate) two_phase_iterator: TPI,
}

impl<TPI> TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
    TPI: TwoPhaseIterator,
{
    pub fn new(two_phase_iterator: TPI) -> Self {
        Self { two_phase_iterator }
    }

    fn do_next(&mut self, mut doc: i32) -> Result<i32> {
        loop {
            if doc == NO_MORE_DOCS {
                return Ok(NO_MORE_DOCS);
            } else if self.two_phase_iterator.matches()? {
                return Ok(doc);
            }
            doc = self.two_phase_iterator.approximation_mut()?.next_doc()?;
        }
    }
}

impl<TPI> DocIdSetIterator for TwoPhaseIteratorAsDocIdSetIterator<TPI>
where
    TPI: TwoPhaseIterator,
{
    fn doc_id(&self) -> i32 {
        self.two_phase_iterator
            .approximation()
            .expect("approximation should not fail")
            .doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc = self.two_phase_iterator.approximation_mut()?.next_doc()?;
        self.do_next(doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc = self
            .two_phase_iterator
            .approximation_mut()?
            .advance(target)?;
        self.do_next(doc)
    }

    fn cost(&self) -> Result<i64> {
        self.two_phase_iterator.approximation()?.cost()
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
        $vis:vis $name:ident
        => { disi: $disi:ident }
        { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> TwoPhaseIterator for $name<$( $T ),+>
        where
            $( $T: TwoPhaseIterator ),+
        {
            type DocIdSetIterator = $disi::<$( <$T as TwoPhaseIterator>::DocIdSetIterator ),+>;

            type DocIdSetIteratorRef<'a> = $disi::<$( <$T as TwoPhaseIterator>::DocIdSetIteratorRef<'a> ),+>
            where
                Self: 'a;

            type DocIdSetIteratorMut<'a> = $disi::<$( <$T as TwoPhaseIterator>::DocIdSetIteratorMut<'a> ),+>
            where
                Self: 'a;

            #[inline]
            fn approximation_mut(&mut self) -> Result<Self::DocIdSetIteratorMut<'_>> {
                match self {
                    $( Self::$Variant(inner) => Ok($disi::$Variant(inner.approximation_mut()?)), )+
                }
            }

            #[inline]
            fn approximation(&self) -> Result<Self::DocIdSetIteratorRef<'_>> {
                match self {
                    $( Self::$Variant(inner) => Ok($disi::$Variant(inner.approximation()?)), )+
                }
            }

            #[inline]
            fn set_empty(&mut self) -> Result<()>{
                match self {
                    $( Self::$Variant(inner) => inner.set_empty(), )+
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
    pub TwoPhaseIteratorEnum2
    => { disi: DocIdSetIteratorEnum2 }
    { A: A, B: B}
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum3
    => { disi: DocIdSetIteratorEnum3 }
    { A: A, B: B, C: C}
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum4
    => { disi: DocIdSetIteratorEnum4 }
    { A: A, B: B, C: C,D:D}
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum5
    => { disi: DocIdSetIteratorEnum5 }
    { A: A, B: B, C: C, D: D, E: E }
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum6
    => { disi: DocIdSetIteratorEnum6 }
    { A: A, B: B, C: C, D: D, E: E, F: F }
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum7
    => { disi: DocIdSetIteratorEnum7 }
    { A: A, B: B, C: C, D: D, E: E, F: F, G: G }
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum8
    => { disi: DocIdSetIteratorEnum8 }
    { A: A, B: B, C: C, D: D, E: E, F: F, G: G, H: H }
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum9
    => { disi: DocIdSetIteratorEnum9 }
    { A: A, B: B, C: C, D: D, E: E, F: F, G: G, H: H, I: I }
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum10
    => { disi: DocIdSetIteratorEnum10 }
    { A: A, B: B, C: C, D: D, E: E, F: F, G: G, H: H, I: I, J: J }
);
either_two_phase_iterator_gat!(
    pub TwoPhaseIteratorEnum11
    => { disi: DocIdSetIteratorEnum11 }
    { A: A, B: B, C: C, D: D, E: E, F: F, G: G, H: H, I: I, J: J, K: K }
);
