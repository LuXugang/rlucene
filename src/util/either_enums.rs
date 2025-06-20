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
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::impact::Impact;
use crate::index::impacts::Impacts;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::impacts_source::ImpactsSource;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::postings_enum::PostingsEnum;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
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

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            EitherSortedSetDocValues::F(t) => t.advance(_target),
            EitherSortedSetDocValues::S(s) => s.advance(_target),
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
