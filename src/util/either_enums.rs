/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::codecs::mutable_point_tree::MutablePointTree;
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::impact::Impact;
use crate::index::impacts::Impacts;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::impacts_source::ImpactsSource;
use crate::index::index_sorter::DocComparator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::point_values::{IntersectVisitor, PointTree};
use crate::index::postings_enum::PostingsEnum;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::accountable::Accountable;
use crate::util::attribute_source::AttributeSource;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

/// # Either Enums for Unified Trait Implementations
///
/// This module defines `EitherXXX` enums that act as static-dispatch-compatible
/// wrappers over two different types implementing the same trait.
///
/// These enums are useful when an algorithm or component can operate over
/// multiple implementations of a common trait (e.g., `PostingsEnum`,
/// `ImpactsEnum`), and a concrete type is needed without resorting to dynamic
/// dispatch.
///
/// ## Examples
/// - [`EitherPostingsEnum`] wraps two types implementing `PostingsEnum`.
/// - [`EitherImpactsEnum`] wraps two types implementing `ImpactsEnum`.
/// - [`EitherImpacts`] wraps two types implementing `Impacts`.
/// - [`EitherSortedNumericDocValues`] wraps two types implementing `SortedNumericDocValues`.
/// - [`EitherNumericDocValues`] wraps two types implementing `NumericDocValues`.
/// - [`EitherSortedDocValues`] wraps two types implementing `SortedDocValues`.
/// - [`EitherTermsEnum`] wraps two types implementing `TermsEnum`.
///
/// Each enum forwards all trait method calls to the underlying variant,
/// enabling seamless use in performance-critical paths without heap allocation
/// or virtual dispatch.
///
/// This approach avoids the overhead of `Box<dyn Trait>` and keeps all behavior
/// statically resolved by the compiler.

// Either2

// ImpactsEnum
pub enum EitherImpactsEnum<F, S> {
    F(F),
    S(S),
}

impl<F, S> PostingsEnum for EitherImpactsEnum<F, S>
where
    F: ImpactsEnum,
    S: ImpactsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.freq(),
            EitherImpactsEnum::S(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.next_position(),
            EitherImpactsEnum::S(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.start_offset(),
            EitherImpactsEnum::S(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.end_offset(),
            EitherImpactsEnum::S(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        match self {
            EitherImpactsEnum::F(t) => t.get_payload(),
            EitherImpactsEnum::S(s) => s.get_payload(),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherImpactsEnum<F, S>
where
    F: ImpactsEnum,
    S: ImpactsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherImpactsEnum::F(t) => t.doc_id(),
            EitherImpactsEnum::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.next_doc(),
            EitherImpactsEnum::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.advance(target),
            EitherImpactsEnum::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.slow_advance(target),
            EitherImpactsEnum::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherImpactsEnum::F(t) => t.cost(),
            EitherImpactsEnum::S(s) => s.cost(),
        }
    }
}

impl<F, S> ImpactsSource for EitherImpactsEnum<F, S>
where
    S: ImpactsEnum,
    F: ImpactsEnum,
{
    fn advance_shallow(&mut self, target: i32) -> Result<()> {
        match self {
            EitherImpactsEnum::F(t) => t.advance_shallow(target),
            EitherImpactsEnum::S(s) => s.advance_shallow(target),
        }
    }

    type Impacts = EitherImpacts<F::Impacts, S::Impacts>;

    fn get_impacts(&mut self) -> Result<Self::Impacts> {
        match self {
            EitherImpactsEnum::F(t) => {
                let impacts = t.get_impacts()?;
                Ok(EitherImpacts::F(impacts))
            },
            EitherImpactsEnum::S(s) => {
                let impacts = s.get_impacts()?;
                Ok(EitherImpacts::S(impacts))
            },
        }
    }
}

impl<F, S> ImpactsEnum for EitherImpactsEnum<F, S>
where
    F: ImpactsEnum,
    S: ImpactsEnum,
{
}

// Impacts
pub enum EitherImpacts<F, S>
where
    F: Impacts,
    S: Impacts,
{
    F(F),
    S(S),
}
impl<F, S> Impacts for EitherImpacts<F, S>
where
    F: Impacts,
    S: Impacts,
{
    fn num_levels(&self) -> i32 {
        match self {
            EitherImpacts::F(t) => t.num_levels(),
            EitherImpacts::S(s) => s.num_levels(),
        }
    }

    fn get_doc_id_up_to(&self, level: i32) -> i32 {
        match self {
            EitherImpacts::F(t) => t.get_doc_id_up_to(level),
            EitherImpacts::S(s) => s.get_doc_id_up_to(level),
        }
    }

    fn get_impacts(&mut self, level: i32) -> Result<Cow<[Impact]>> {
        match self {
            EitherImpacts::F(t) => t.get_impacts(level),
            EitherImpacts::S(s) => s.get_impacts(level),
        }
    }
}

// PostingsEnum
pub enum EitherPostingsEnum<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocIdSetIterator for EitherPostingsEnum<F, S>
where
    F: PostingsEnum,
    S: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherPostingsEnum::F(t) => t.doc_id(),
            EitherPostingsEnum::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.next_doc(),
            EitherPostingsEnum::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.advance(target),
            EitherPostingsEnum::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.slow_advance(target),
            EitherPostingsEnum::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherPostingsEnum::F(t) => t.cost(),
            EitherPostingsEnum::S(s) => s.cost(),
        }
    }
}

impl<F, S> PostingsEnum for EitherPostingsEnum<F, S>
where
    F: PostingsEnum,
    S: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.freq(),
            EitherPostingsEnum::S(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.next_position(),
            EitherPostingsEnum::S(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.start_offset(),
            EitherPostingsEnum::S(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self {
            EitherPostingsEnum::F(t) => t.end_offset(),
            EitherPostingsEnum::S(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        match self {
            EitherPostingsEnum::F(t) => t.get_payload(),
            EitherPostingsEnum::S(s) => s.get_payload(),
        }
    }
}

// TermState
pub enum EitherTermState<F, S> {
    F(F),
    S(S),
}

impl<F, S> Display for EitherTermState<F, S>
where
    F: TermState,
    S: TermState,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EitherTermState::F(t) => write!(f, "EitherTermState::F({})", t),
            EitherTermState::S(s) => write!(f, "EitherTermState::S({})", s),
        }
    }
}

impl<F, S> Clone for EitherTermState<F, S>
where
    F: TermState,
    S: TermState,
{
    fn clone(&self) -> Self {
        match self {
            EitherTermState::F(t) => EitherTermState::F(t.clone()),
            EitherTermState::S(s) => EitherTermState::S(s.clone()),
        }
    }
}

impl<F, S> TermState for EitherTermState<F, S>
where
    F: TermState,
    S: TermState,
{
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()> {
        match self {
            EitherTermState::F(t) => t.copy_from(other),
            EitherTermState::S(s) => s.copy_from(other),
        }
    }
}

// NumericDocValues
pub enum EitherNumericDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherNumericDocValues<F, S>
where
    F: NumericDocValues,
    S: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            EitherNumericDocValues::F(t) => t.advance_exact(target),
            EitherNumericDocValues::S(s) => s.advance_exact(target),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherNumericDocValues<F, S>
where
    F: NumericDocValues,
    S: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherNumericDocValues::F(t) => t.doc_id(),
            EitherNumericDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherNumericDocValues::F(t) => t.next_doc(),
            EitherNumericDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherNumericDocValues::F(t) => t.advance(target),
            EitherNumericDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherNumericDocValues::F(t) => t.slow_advance(target),
            EitherNumericDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherNumericDocValues::F(t) => t.cost(),
            EitherNumericDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> NumericDocValues for EitherNumericDocValues<F, S>
where
    F: NumericDocValues,
    S: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        match self {
            EitherNumericDocValues::F(t) => t.long_value(),
            EitherNumericDocValues::S(s) => s.long_value(),
        }
    }
}

// SortedNumericDocValues
pub enum EitherSortedNumericDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherSortedNumericDocValues<F, S>
where
    F: SortedNumericDocValues,
    S: SortedNumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.advance_exact(target),
            EitherSortedNumericDocValues::S(s) => s.advance_exact(target),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherSortedNumericDocValues<F, S>
where
    F: SortedNumericDocValues,
    S: SortedNumericDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherSortedNumericDocValues::F(t) => t.doc_id(),
            EitherSortedNumericDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.next_doc(),
            EitherSortedNumericDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.advance(target),
            EitherSortedNumericDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.slow_advance(target),
            EitherSortedNumericDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.cost(),
            EitherSortedNumericDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> SortedNumericDocValues for EitherSortedNumericDocValues<F, S>
where
    F: SortedNumericDocValues,
    S: SortedNumericDocValues,
{
    fn next_value(&mut self) -> Result<i64> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.next_value(),
            EitherSortedNumericDocValues::S(s) => s.next_value(),
        }
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        match self {
            EitherSortedNumericDocValues::F(t) => t.doc_value_count(),
            EitherSortedNumericDocValues::S(s) => s.doc_value_count(),
        }
    }

    fn is_single_valued(&self) -> bool {
        match self {
            EitherSortedNumericDocValues::F(t) => t.is_single_valued(),
            EitherSortedNumericDocValues::S(s) => s.is_single_valued(),
        }
    }

    type NumericDocValues = EitherNumericDocValues<F::NumericDocValues, S::NumericDocValues>;

    fn get_numeric_doc_values(&mut self) -> Result<Option<Self::NumericDocValues>> {
        match self {
            EitherSortedNumericDocValues::F(t) => {
                let sorted_doc_values = t.get_numeric_doc_values()?;
                Ok(sorted_doc_values.map(EitherNumericDocValues::F))
            },
            EitherSortedNumericDocValues::S(s) => {
                let sorted_doc_values = s.get_numeric_doc_values()?;
                Ok(sorted_doc_values.map(EitherNumericDocValues::S))
            },
        }
    }
}
// SortedDocValues
pub enum EitherSortedDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherSortedDocValues<F, S>
where
    F: SortedDocValues,
    S: SortedDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            EitherSortedDocValues::F(t) => t.advance_exact(target),
            EitherSortedDocValues::S(s) => s.advance_exact(target),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherSortedDocValues<F, S>
where
    F: SortedDocValues,
    S: SortedDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherSortedDocValues::F(t) => t.doc_id(),
            EitherSortedDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherSortedDocValues::F(t) => t.next_doc(),
            EitherSortedDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedDocValues::F(t) => t.advance(target),
            EitherSortedDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedDocValues::F(t) => t.slow_advance(target),
            EitherSortedDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherSortedDocValues::F(t) => t.cost(),
            EitherSortedDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> SortedDocValues for EitherSortedDocValues<F, S>
where
    F: SortedDocValues,
    S: SortedDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        match self {
            EitherSortedDocValues::F(t) => t.ord_value(),
            EitherSortedDocValues::S(s) => s.ord_value(),
        }
    }

    fn lookup_ord(&mut self, _ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        match self {
            EitherSortedDocValues::F(t) => t.lookup_ord(_ord),
            EitherSortedDocValues::S(s) => s.lookup_ord(_ord),
        }
    }

    fn get_value_count(&mut self) -> Result<i32> {
        match self {
            EitherSortedDocValues::F(t) => t.get_value_count(),
            EitherSortedDocValues::S(s) => s.get_value_count(),
        }
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        match self {
            EitherSortedDocValues::F(t) => t.lookup_term(key),
            EitherSortedDocValues::S(s) => s.lookup_term(key),
        }
    }

    type TermsEnum = EitherTermsEnum<F::TermsEnum, S::TermsEnum>;

    fn terms_enum(&mut self) -> Result<Self::TermsEnum> {
        match self {
            EitherSortedDocValues::F(t) => {
                let terms_enum = t.terms_enum()?;
                Ok(EitherTermsEnum::F(terms_enum))
            },
            EitherSortedDocValues::S(s) => {
                let terms_enum = s.terms_enum()?;
                Ok(EitherTermsEnum::S(terms_enum))
            },
        }
    }
}

// SortedSetDocValues
pub enum EitherSortedSetDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherSortedSetDocValues<F, S>
where
    F: SortedSetDocValues,
    S: SortedSetDocValues,
{
}

impl<F, S> DocIdSetIterator for EitherSortedSetDocValues<F, S>
where
    F: SortedSetDocValues,
    S: SortedSetDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherSortedSetDocValues::F(t) => t.doc_id(),
            EitherSortedSetDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherSortedSetDocValues::F(t) => t.next_doc(),
            EitherSortedSetDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedSetDocValues::F(t) => t.advance(target),
            EitherSortedSetDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherSortedSetDocValues::F(t) => t.slow_advance(target),
            EitherSortedSetDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherSortedSetDocValues::F(t) => t.cost(),
            EitherSortedSetDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> SortedSetDocValues for EitherSortedSetDocValues<F, S>
where
    F: SortedSetDocValues,
    S: SortedSetDocValues,
{
    fn next_ord(&mut self) -> Result<i64> {
        match self {
            EitherSortedSetDocValues::F(t) => t.next_ord(),
            EitherSortedSetDocValues::S(s) => s.next_ord(),
        }
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        match self {
            EitherSortedSetDocValues::F(t) => t.doc_value_count(),
            EitherSortedSetDocValues::S(s) => s.doc_value_count(),
        }
    }

    fn lookup_ord(&mut self, _ord: i64) -> Result<Cow<BytesRef<Vec<u8>>>> {
        match self {
            EitherSortedSetDocValues::F(t) => t.lookup_ord(_ord),
            EitherSortedSetDocValues::S(s) => s.lookup_ord(_ord),
        }
    }

    fn get_value_count(&mut self) -> Result<i64> {
        match self {
            EitherSortedSetDocValues::F(t) => t.get_value_count(),
            EitherSortedSetDocValues::S(s) => s.get_value_count(),
        }
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i64> {
        match self {
            EitherSortedSetDocValues::F(t) => t.lookup_term(key),
            EitherSortedSetDocValues::S(s) => s.lookup_term(key),
        }
    }

    type TermsEnum = EitherTermsEnum<F::TermsEnum, S::TermsEnum>;

    fn terms_enum(&mut self) -> Result<Self::TermsEnum> {
        match self {
            EitherSortedSetDocValues::F(t) => {
                let terms_enum = t.terms_enum()?;
                Ok(EitherTermsEnum::F(terms_enum))
            },
            EitherSortedSetDocValues::S(s) => {
                let terms_enum = s.terms_enum()?;
                Ok(EitherTermsEnum::S(terms_enum))
            },
        }
    }

    fn is_single_valued(&self) -> bool {
        match self {
            EitherSortedSetDocValues::F(t) => t.is_single_valued(),
            EitherSortedSetDocValues::S(s) => s.is_single_valued(),
        }
    }

    type SortedDocValues = EitherSortedDocValues<F::SortedDocValues, S::SortedDocValues>;

    fn get_sorted_doc_values(&mut self) -> Result<Option<Self::SortedDocValues>> {
        match self {
            EitherSortedSetDocValues::F(t) => {
                let sorted_doc_values = t.get_sorted_doc_values()?;
                Ok(sorted_doc_values.map(EitherSortedDocValues::F))
            },
            EitherSortedSetDocValues::S(s) => {
                let sorted_doc_values = s.get_sorted_doc_values()?;
                Ok(sorted_doc_values.map(EitherSortedDocValues::S))
            },
        }
    }
}

// BinaryDocValues
pub enum EitherBinaryDocValues<F, S> {
    F(F),
    S(S),
}

impl<F, S> DocValuesIterator for EitherBinaryDocValues<F, S>
where
    F: BinaryDocValues,
    S: BinaryDocValues,
{
}

impl<F, S> DocIdSetIterator for EitherBinaryDocValues<F, S>
where
    F: BinaryDocValues,
    S: BinaryDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherBinaryDocValues::F(t) => t.doc_id(),
            EitherBinaryDocValues::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherBinaryDocValues::F(t) => t.next_doc(),
            EitherBinaryDocValues::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherBinaryDocValues::F(t) => t.advance(target),
            EitherBinaryDocValues::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherBinaryDocValues::F(t) => t.slow_advance(target),
            EitherBinaryDocValues::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherBinaryDocValues::F(t) => t.cost(),
            EitherBinaryDocValues::S(s) => s.cost(),
        }
    }
}

impl<F, S> BinaryDocValues for EitherBinaryDocValues<F, S>
where
    F: BinaryDocValues,
    S: BinaryDocValues,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        match self {
            EitherBinaryDocValues::F(t) => t.binary_value(),
            EitherBinaryDocValues::S(s) => s.binary_value(),
        }
    }
}

// TermsEnum
pub enum EitherTermsEnum<F, S> {
    F(F),
    S(S),
}

impl<F, S> BytesRefIterator for EitherTermsEnum<F, S>
where
    F: TermsEnum,
    S: TermsEnum,
{
}

impl<F, S> TermsEnum for EitherTermsEnum<F, S>
where
    F: TermsEnum,
    S: TermsEnum,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        match self {
            EitherTermsEnum::F(t) => t.attributes(),
            EitherTermsEnum::S(s) => s.attributes(),
        }
    }

    fn seek_exact(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<bool> {
        match self {
            EitherTermsEnum::F(t) => t.seek_exact(_term),
            EitherTermsEnum::S(s) => s.seek_exact(_term),
        }
    }

    fn prepare_seek_exact(&mut self, _text: &BytesRef<Vec<u8>>) -> Result<bool> {
        match self {
            EitherTermsEnum::F(t) => t.prepare_seek_exact(_text),
            EitherTermsEnum::S(s) => s.prepare_seek_exact(_text),
        }
    }

    fn seek_ceil(&mut self, _term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        match self {
            EitherTermsEnum::F(t) => t.seek_ceil(_term),
            EitherTermsEnum::S(s) => s.seek_ceil(_term),
        }
    }

    fn seek_exact_with_ord(&mut self, _ord: i64) -> Result<()> {
        match self {
            EitherTermsEnum::F(t) => t.seek_exact_with_ord(_ord),
            EitherTermsEnum::S(s) => s.seek_exact_with_ord(_ord),
        }
    }

    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<Vec<u8>>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        match self {
            EitherTermsEnum::F(t) => t.seek_exact_with_state(_term, _state),
            EitherTermsEnum::S(s) => s.seek_exact_with_state(_term, _state),
        }
    }

    fn term(&self) -> Result<Cow<BytesRef<Vec<u8>>>> {
        match self {
            EitherTermsEnum::F(t) => t.term(),
            EitherTermsEnum::S(s) => s.term(),
        }
    }

    fn ord(&self) -> Result<i64> {
        match self {
            EitherTermsEnum::F(t) => t.ord(),
            EitherTermsEnum::S(s) => s.ord(),
        }
    }

    fn doc_freq(&mut self) -> Result<i32> {
        match self {
            EitherTermsEnum::F(t) => t.doc_freq(),
            EitherTermsEnum::S(s) => s.doc_freq(),
        }
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        match self {
            EitherTermsEnum::F(t) => t.total_term_freq(),
            EitherTermsEnum::S(s) => s.total_term_freq(),
        }
    }

    type PostingsEnum = EitherPostingsEnum<F::PostingsEnum, S::PostingsEnum>;

    fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
        match self {
            EitherTermsEnum::F(t) => match reuse {
                Some(EitherPostingsEnum::F(v)) => {
                    let postings_enum = t.postings(Some(v))?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                None => {
                    let postings_enum = t.postings(None)?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::F expected EitherPostingsEnum::F for reuse".to_string(),
                )),
            },
            EitherTermsEnum::S(s) => match reuse {
                Some(EitherPostingsEnum::S(v)) => {
                    let postings_enum = s.postings(Some(v))?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                None => {
                    let postings_enum = s.postings(None)?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::S expected EitherPostingsEnum::S for reuse".to_string(),
                )),
            },
        }
    }

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        match self {
            EitherTermsEnum::F(t) => match reuse {
                Some(EitherPostingsEnum::F(v)) => {
                    let postings_enum = t.postings_with_flags(Some(v), flags)?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                None => {
                    let postings_enum = t.postings_with_flags(None, flags)?;
                    Ok(EitherPostingsEnum::F(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::F expected EitherPostingsEnum::F for reuse".to_string(),
                )),
            },
            EitherTermsEnum::S(s) => match reuse {
                Some(EitherPostingsEnum::S(v)) => {
                    let postings_enum = s.postings_with_flags(Some(v), flags)?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                None => {
                    let postings_enum = s.postings_with_flags(None, flags)?;
                    Ok(EitherPostingsEnum::S(postings_enum))
                },
                _ => Err(LuceneError::illegal_state(
                    "EitherTermsEnum::S expected EitherPostingsEnum::S for reuse".to_string(),
                )),
            },
        }
    }

    type ImpactsEnum = EitherImpactsEnum<F::ImpactsEnum, S::ImpactsEnum>;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        match self {
            EitherTermsEnum::F(t) => {
                let impacts_enum = t.impacts(flags)?;
                Ok(EitherImpactsEnum::F(impacts_enum))
            },
            EitherTermsEnum::S(s) => {
                let impacts_enum = s.impacts(flags)?;
                Ok(EitherImpactsEnum::S(impacts_enum))
            },
        }
    }

    type TermState = EitherTermState<F::TermState, S::TermState>;

    fn term_state(&mut self) -> Result<Self::TermState> {
        match self {
            EitherTermsEnum::F(t) => {
                let term_state = t.term_state()?;
                Ok(EitherTermState::F(term_state))
            },
            EitherTermsEnum::S(s) => {
                let term_state = s.term_state()?;
                Ok(EitherTermState::S(term_state))
            },
        }
    }
}
// MutablePointTree
pub enum EitherMutablePointTree<F, S> {
    F(F),
    S(S),
}

impl<F, S> PointTree for EitherMutablePointTree<F, S>
where
    F: MutablePointTree,
    S: MutablePointTree,
{
    fn move_to_child(&mut self) -> Result<bool> {
        match self {
            EitherMutablePointTree::F(t) => t.move_to_child(),
            EitherMutablePointTree::S(s) => s.move_to_child(),
        }
    }

    fn move_to_sibling(&mut self) -> Result<bool> {
        match self {
            EitherMutablePointTree::F(t) => t.move_to_sibling(),
            EitherMutablePointTree::S(s) => s.move_to_sibling(),
        }
    }

    fn move_to_parent(&mut self) -> Result<bool> {
        match self {
            EitherMutablePointTree::F(t) => t.move_to_parent(),
            EitherMutablePointTree::S(s) => s.move_to_parent(),
        }
    }

    fn get_min_packed_value(&self) -> Result<&[u8]> {
        match self {
            EitherMutablePointTree::F(t) => t.get_min_packed_value(),
            EitherMutablePointTree::S(s) => s.get_min_packed_value(),
        }
    }

    fn get_max_packed_value(&self) -> Result<&[u8]> {
        match self {
            EitherMutablePointTree::F(t) => t.get_max_packed_value(),
            EitherMutablePointTree::S(s) => s.get_max_packed_value(),
        }
    }

    fn size(&self) -> Result<i64> {
        match self {
            EitherMutablePointTree::F(t) => t.size(),
            EitherMutablePointTree::S(s) => s.size(),
        }
    }

    fn visit_doc_ids<IV>(&mut self, visitor: &mut IV) -> Result<()>
    where
        IV: IntersectVisitor,
    {
        match self {
            EitherMutablePointTree::F(t) => t.visit_doc_ids(visitor),
            EitherMutablePointTree::S(s) => s.visit_doc_ids(visitor),
        }
    }

    fn visit_doc_values<IV>(&mut self, visitor: &mut IV) -> Result<()>
    where
        IV: IntersectVisitor,
    {
        match self {
            EitherMutablePointTree::F(t) => t.visit_doc_values(visitor),
            EitherMutablePointTree::S(s) => s.visit_doc_values(visitor),
        }
    }
}

impl<F, S> Clone for EitherMutablePointTree<F, S>
where
    F: MutablePointTree,
    S: MutablePointTree,
{
    fn clone(&self) -> Self {
        match self {
            EitherMutablePointTree::F(t) => EitherMutablePointTree::F(t.clone()),
            EitherMutablePointTree::S(s) => EitherMutablePointTree::S(s.clone()),
        }
    }
}

impl<F, S> MutablePointTree for EitherMutablePointTree<F, S>
where
    F: MutablePointTree,
    S: MutablePointTree,
{
    fn get_value(&self, i: usize, packed_value: &mut BytesRef<Vec<u8>>) {
        match self {
            EitherMutablePointTree::F(t) => t.get_value(i, packed_value),
            EitherMutablePointTree::S(s) => s.get_value(i, packed_value),
        }
    }

    fn get_byte_at(&self, i: usize, k: usize) -> u8 {
        match self {
            EitherMutablePointTree::F(t) => t.get_byte_at(i, k),
            EitherMutablePointTree::S(s) => s.get_byte_at(i, k),
        }
    }

    fn get_doc_id(&self, i: usize) -> i32 {
        match self {
            EitherMutablePointTree::F(t) => t.get_doc_id(i),
            EitherMutablePointTree::S(s) => s.get_doc_id(i),
        }
    }

    fn swap(&mut self, i: usize, j: usize) {
        match self {
            EitherMutablePointTree::F(t) => t.swap(i, j),
            EitherMutablePointTree::S(s) => s.swap(i, j),
        }
    }

    fn save(&mut self, i: usize, j: usize) {
        match self {
            EitherMutablePointTree::F(t) => t.save(i, j),
            EitherMutablePointTree::S(s) => s.save(i, j),
        }
    }

    fn restore(&mut self, i: usize, j: usize) {
        match self {
            EitherMutablePointTree::F(t) => t.restore(i, j),
            EitherMutablePointTree::S(s) => s.restore(i, j),
        }
    }
}
// BitSet
pub enum EitherBitSet<F, S> {
    F(F),
    S(S),
}

impl<F, S> Bits for EitherBitSet<F, S>
where
    F: BitSet,
    S: BitSet,
{
    fn get(&self, index: i32) -> bool {
        match self {
            EitherBitSet::F(t) => t.get(index),
            EitherBitSet::S(s) => s.get(index),
        }
    }

    fn length(&self) -> i32 {
        match self {
            EitherBitSet::F(t) => t.length(),
            EitherBitSet::S(s) => s.length(),
        }
    }
}

impl<F, S> Accountable for EitherBitSet<F, S>
where
    F: BitSet,
    S: BitSet,
{
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            EitherBitSet::F(t) => t.ram_bytes_used(),
            EitherBitSet::S(s) => s.ram_bytes_used(),
        }
    }
}

impl<F, S> BitSet for EitherBitSet<F, S>
where
    F: BitSet,
    S: BitSet,
{
    fn clear(&mut self) {
        match self {
            EitherBitSet::F(t) => t.clear(),
            EitherBitSet::S(s) => s.clear(),
        }
    }

    fn set(&mut self, i: i32) {
        match self {
            EitherBitSet::F(t) => t.set(i),
            EitherBitSet::S(s) => s.set(i),
        }
    }

    fn get_and_set(&mut self, i: i32) -> bool {
        match self {
            EitherBitSet::F(t) => t.get_and_set(i),
            EitherBitSet::S(s) => s.get_and_set(i),
        }
    }

    fn clear_with_index(&mut self, i: i32) {
        match self {
            EitherBitSet::F(t) => t.clear_with_index(i),
            EitherBitSet::S(s) => s.clear_with_index(i),
        }
    }

    fn clear_range(&mut self, start_index: i32, end_index: i32) {
        match self {
            EitherBitSet::F(t) => t.clear_range(start_index, end_index),
            EitherBitSet::S(s) => s.clear_range(start_index, end_index),
        }
    }

    fn cardinality(&self) -> i32 {
        match self {
            EitherBitSet::F(t) => t.cardinality(),
            EitherBitSet::S(s) => s.cardinality(),
        }
    }

    fn approximate_cardinality(&self) -> i32 {
        match self {
            EitherBitSet::F(t) => t.approximate_cardinality(),
            EitherBitSet::S(s) => s.approximate_cardinality(),
        }
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        match self {
            EitherBitSet::F(t) => t.prev_set_bit(index),
            EitherBitSet::S(s) => s.prev_set_bit(index),
        }
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        match self {
            EitherBitSet::F(t) => t.next_set_bit(index),
            EitherBitSet::S(s) => s.next_set_bit(index),
        }
    }

    fn next_set_bit_range(&self, start: i32, end: i32) -> i32 {
        match self {
            EitherBitSet::F(t) => t.next_set_bit_range(start, end),
            EitherBitSet::S(s) => s.next_set_bit_range(start, end),
        }
    }

    fn or<T: DocIdSetIterator>(&mut self, iter: T) -> Result<()> {
        match self {
            EitherBitSet::F(t) => t.or(iter),
            EitherBitSet::S(s) => s.or(iter),
        }
    }

    fn ensure_capacity(&mut self, _num_bits: i32) {
        match self {
            EitherBitSet::F(t) => t.ensure_capacity(_num_bits),
            EitherBitSet::S(s) => s.ensure_capacity(_num_bits),
        }
    }
}

// DocComparator
pub enum EitherDocComparator<F, S> {
    F(F),
    S(S),
}
impl<F, S> DocComparator for EitherDocComparator<F, S>
where
    F: DocComparator,
    S: DocComparator,
{
    fn compare(&self, doc_id1: i32, doc_id2: i32) -> i32 {
        match self {
            EitherDocComparator::F(t) => t.compare(doc_id1, doc_id2),
            EitherDocComparator::S(s) => s.compare(doc_id1, doc_id2),
        }
    }
}

// Either 3
// NumericDocValues
pub enum Either3NumericDocValues<F, S, T> {
    F(F),
    S(S),
    T(T),
}

impl<F, S, T> DocValuesIterator for Either3NumericDocValues<F, S, T>
where
    F: NumericDocValues,
    S: NumericDocValues,
    T: NumericDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        match self {
            Either3NumericDocValues::F(t) => t.advance_exact(target),
            Either3NumericDocValues::S(s) => s.advance_exact(target),
            Either3NumericDocValues::T(t) => t.advance_exact(target),
        }
    }
}

impl<F, S, T> DocIdSetIterator for Either3NumericDocValues<F, S, T>
where
    F: NumericDocValues,
    S: NumericDocValues,
    T: NumericDocValues,
{
    fn doc_id(&self) -> i32 {
        match self {
            Either3NumericDocValues::F(t) => t.doc_id(),
            Either3NumericDocValues::S(s) => s.doc_id(),
            Either3NumericDocValues::T(t) => t.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            Either3NumericDocValues::F(t) => t.next_doc(),
            Either3NumericDocValues::S(s) => s.next_doc(),
            Either3NumericDocValues::T(t) => t.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        match self {
            Either3NumericDocValues::F(t) => t.advance(target),
            Either3NumericDocValues::S(s) => s.advance(target),
            Either3NumericDocValues::T(t) => t.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            Either3NumericDocValues::F(t) => t.slow_advance(target),
            Either3NumericDocValues::S(s) => s.slow_advance(target),
            Either3NumericDocValues::T(t) => t.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            Either3NumericDocValues::F(t) => t.cost(),
            Either3NumericDocValues::S(s) => s.cost(),
            Either3NumericDocValues::T(t) => t.cost(),
        }
    }
}

impl<F, S, T> NumericDocValues for Either3NumericDocValues<F, S, T>
where
    F: NumericDocValues,
    S: NumericDocValues,
    T: NumericDocValues,
{
    fn long_value(&mut self) -> Result<i64> {
        match self {
            Either3NumericDocValues::F(t) => t.long_value(),
            Either3NumericDocValues::S(s) => s.long_value(),
            Either3NumericDocValues::T(t) => t.long_value(),
        }
    }
}
