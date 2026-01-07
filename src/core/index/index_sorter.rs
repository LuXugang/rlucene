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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::ordinal_map::{OrdinalMap, SegmentToGlobalOrds};
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::sort_field::MissingValueEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use crate::core::util::numeric_utils::NumericUtils;
use crate::core::util::packed::PackedInts;
use crate::core::util::{ToInt, TryIntoInt};
use std::rc::Rc;
/// Handles how documents should be sorted in an index, both within a segment
/// and between segments.
pub trait IndexSorter {
    fn get_provider_name(&self) -> &str;

    type ComparableProvider<LR>: ComparableProvider
    where
        LR: LeafReader;
    fn get_comparable_providers<LR>(
        &self,
        _readers: &[LR],
    ) -> Result<Vec<Self::ComparableProvider<LR>>>
    where
        LR: LeafReader,
    {
        Err(LuceneError::unsupported_operation(""))
    }
    fn get_comparable_providers_per_reader<LR>(
        &self,
        _reader: &LR,
        _reader_index: usize,
        _formated_missing_value: &MissingValueEnum,
        _ordinal_map: Option<&OrdinalMap>,
    ) -> Result<Self::ComparableProvider<LR>>
    where
        LR: LeafReader,
    {
        Err(LuceneError::unsupported_operation(""))
    }

    fn get_ordinal_map<LR>(&self, _readers: &[LR]) -> Result<Option<OrdinalMap>>
    where
        LR: LeafReader,
    {
        Ok(None)
    }
    fn get_missing_value(&self) -> MissingValueEnum;
    type DocComparator: DocComparator;
    /// Get a comparator that determines the sort order of documents within a single reader.
    ///
    /// **NB**: We cannot simply use the `FieldComparator` API because it requires
    /// document IDs to be provided in-order. The default implementations allocate
    /// an array of size `max_doc` to store native values for comparison, but:
    ///
    /// 1. They are transient, only living while sorting a single segment.
    /// 2. In typical index-sorting scenarios, they are only used to sort newly
    ///    flushed segments, which are usually much smaller than merged segments.
    ///
    /// # Parameters
    ///
    /// - `reader`: the reader whose documents should be sorted.
    /// - `max_doc`: the number of documents in the reader.
    fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
    where
        LR: LeafReader;
}

// DoubleSorter
/// Sorts documents based on double values from a NumericDocValues instance.
pub struct DoubleSorter<NP> {
    provider_name: String,
    missing_value: Option<f64>,
    reverse_mul: i32,
    values_provider: NP,
}
impl<NP> DoubleSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    pub fn new(
        provider_name: String,
        missing_value: Option<MissingValueEnum>,
        reverse: bool,
        values_provider: NP,
    ) -> Result<Self> {
        let missing_value = if let Some(mv) = missing_value {
            match mv {
                MissingValueEnum::Double(value) => Some(value),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "Missing value type mismatch for Double.",
                    ));
                },
            }
        } else {
            None
        };
        Ok(Self {
            provider_name,
            missing_value,
            reverse_mul: if reverse { -1 } else { 1 },
            values_provider,
        })
    }
}
impl<NP> IndexSorter for DoubleSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }

    type ComparableProvider<LR>
        = DoubleComparableProvider<ProviderNumeric<NP, LR>>
    where
        LR: LeafReader;

    fn get_comparable_providers_per_reader<LR>(
        &self,
        reader: &LR,
        _reader_index: usize,
        formated_missing_value: &MissingValueEnum,
        _ordinal_map: Option<&OrdinalMap>,
    ) -> Result<Self::ComparableProvider<LR>>
    where
        LR: LeafReader,
    {
        let missing_value_bits = match formated_missing_value {
            MissingValueEnum::Long(value) => value,
            _ => {
                return Err(LuceneError::illegal_state(
                    "formated Missing value type mismatch for i64.",
                ));
            },
        };
        let values = self.values_provider.get(reader)?;
        Ok(DoubleComparableProvider::new(values, *missing_value_bits))
    }

    fn get_missing_value(&self) -> MissingValueEnum {
        (self.missing_value.unwrap_or(0.0).to_bits() as i64).into()
    }

    type DocComparator = DocComparatorImplDouble;

    fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        let mut dvs = self.values_provider.get(leaf_reader)?;
        let mut values = vec![0f64; max_doc as usize];
        if self.missing_value.is_some() {
            values.fill(*self.missing_value.as_ref().unwrap())
        }
        loop {
            let doc_id = dvs.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            values[doc_id as usize] = f64::from_bits(dvs.long_value()? as u64);
        }
        Ok(DocComparatorImplDouble::new(values, self.reverse_mul))
    }
}
pub struct DocComparatorImplDouble {
    values: Vec<f64>,
    reverse_mul: i32,
}
impl DocComparatorImplDouble {
    pub fn new(values: Vec<f64>, reverse_mul: i32) -> Self {
        Self {
            values,
            reverse_mul,
        }
    }
}
impl DocComparator for DocComparatorImplDouble {
    fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32 {
        self.reverse_mul
            * self.values[doc_id1]
                .total_cmp(&self.values[doc_id2])
                .to_int()
    }
}
pub struct DoubleComparableProvider<N>
where
    N: NumericDocValues,
{
    values: N,
    missing_value_bits: i64,
}

impl<N> DoubleComparableProvider<N>
where
    N: NumericDocValues,
{
    pub fn new(values: N, missing_value_bits: i64) -> Self {
        Self {
            values,
            missing_value_bits,
        }
    }
}

impl<N> ComparableProvider for DoubleComparableProvider<N>
where
    N: NumericDocValues,
{
    fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64> {
        let v = if self.values.advance_exact(doc_id)? {
            self.values.long_value()?
        } else {
            self.missing_value_bits
        };
        Ok(NumericUtils::sortable_double_bits(v))
    }
}

// IntSorter
/// Sorts documents based on integer values from a NumericDocValues instance  */
pub struct IntSorter<NP> {
    provider_name: String,
    missing_value: Option<i32>,
    reverse_mul: i32,
    values_provider: NP,
}
impl<NP> IntSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    pub fn new(
        provider_name: String,
        missing_value: Option<MissingValueEnum>,
        reverse: bool,
        values_provider: NP,
    ) -> Result<Self> {
        let missing_value = if let Some(mv) = missing_value {
            match mv {
                MissingValueEnum::Int(value) => Some(value),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "Missing value type mismatch for INT.",
                    ));
                },
            }
        } else {
            None
        };
        Ok(Self {
            provider_name,
            missing_value,
            reverse_mul: if reverse { -1 } else { 1 },
            values_provider,
        })
    }
}

impl<NP> IndexSorter for IntSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }

    type ComparableProvider<LR>
        = IntComparableProvider<ProviderNumeric<NP, LR>>
    where
        LR: LeafReader;

    fn get_comparable_providers_per_reader<LR>(
        &self,
        reader: &LR,
        _reader_index: usize,
        formated_missing_value: &MissingValueEnum,
        _ordinal_map: Option<&OrdinalMap>,
    ) -> Result<Self::ComparableProvider<LR>>
    where
        LR: LeafReader,
    {
        let missing_value = match formated_missing_value {
            MissingValueEnum::Long(value) => value,
            _ => {
                return Err(LuceneError::illegal_state(
                    "formated Missing value type mismatch for i64.",
                ));
            },
        };
        let values = self.values_provider.get(reader)?;
        Ok(IntComparableProvider::new(values, *missing_value))
    }

    fn get_missing_value(&self) -> MissingValueEnum {
        (self.missing_value.unwrap_or(0) as i64).into()
    }

    type DocComparator = DocComparatorImplInt;

    fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        let mut dvs = self.values_provider.get(leaf_reader)?;
        let mut values = vec![0i32; max_doc as usize];
        if let Some(mv) = self.missing_value {
            values.fill(mv);
        }
        loop {
            let doc_id = dvs.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            values[doc_id as usize] = dvs.long_value()? as i32;
        }
        Ok(DocComparatorImplInt::new(values, self.reverse_mul))
    }
}
pub struct DocComparatorImplInt {
    values: Vec<i32>,
    reverse_mul: i32,
}
impl DocComparatorImplInt {
    pub fn new(values: Vec<i32>, reverse_mul: i32) -> Self {
        Self {
            values,
            reverse_mul,
        }
    }
}
impl DocComparator for DocComparatorImplInt {
    fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32 {
        self.reverse_mul * self.values[doc_id1].cmp(&self.values[doc_id2]).to_int()
    }
}
pub struct IntComparableProvider<N>
where
    N: NumericDocValues,
{
    values: N,
    missing_value: i64,
}
impl<N> IntComparableProvider<N>
where
    N: NumericDocValues,
{
    pub fn new(values: N, missing_value: i64) -> Self {
        Self {
            values,
            missing_value,
        }
    }
}
impl<N> ComparableProvider for IntComparableProvider<N>
where
    N: NumericDocValues,
{
    fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64> {
        if self.values.advance_exact(doc_id)? {
            Ok(self.values.long_value()?)
        } else {
            Ok(self.missing_value)
        }
    }
}

// LongSorter
/// Sorts documents based on long values from a NumericDocValues instance
pub struct LongSorter<NP> {
    provider_name: String,
    missing_value: Option<i64>,
    reverse_mul: i32,
    values_provider: NP,
}
impl<NP> LongSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    pub fn new(
        provider_name: String,
        missing_value: Option<MissingValueEnum>,
        reverse: bool,
        values_provider: NP,
    ) -> Result<Self> {
        let missing_value = if let Some(mv) = missing_value {
            match mv {
                MissingValueEnum::Long(value) => Some(value),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "Missing value type mismatch for Long.",
                    ));
                },
            }
        } else {
            None
        };
        Ok(Self {
            provider_name,
            missing_value,
            reverse_mul: if reverse { -1 } else { 1 },
            values_provider,
        })
    }
}

impl<NP> IndexSorter for LongSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }

    type ComparableProvider<LR>
        = LongComparableProvider<ProviderNumeric<NP, LR>>
    where
        LR: LeafReader;

    fn get_comparable_providers_per_reader<LR>(
        &self,
        reader: &LR,
        _reader_index: usize,
        formated_missing_value: &MissingValueEnum,
        _ordinal_map: Option<&OrdinalMap>,
    ) -> Result<Self::ComparableProvider<LR>>
    where
        LR: LeafReader,
    {
        let missing_value = match formated_missing_value {
            MissingValueEnum::Long(value) => value,
            _ => {
                return Err(LuceneError::illegal_state(
                    "formated Missing value type mismatch for i64.",
                ));
            },
        };
        let values = self.values_provider.get(reader)?;
        Ok(LongComparableProvider::new(values, *missing_value))
    }

    fn get_missing_value(&self) -> MissingValueEnum {
        (self.missing_value.unwrap_or(0)).into()
    }

    type DocComparator = DocComparatorImplLong;

    fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        let mut dvs = self.values_provider.get(leaf_reader)?;
        let mut values = vec![0i64; max_doc as usize];
        if let Some(mv) = self.missing_value {
            values.fill(mv);
        }
        loop {
            let doc_id = dvs.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            values[doc_id as usize] = dvs.long_value()?;
        }
        Ok(DocComparatorImplLong::new(values, self.reverse_mul))
    }
}

pub struct DocComparatorImplLong {
    values: Vec<i64>,
    reverse_mul: i32,
}

impl DocComparatorImplLong {
    pub fn new(values: Vec<i64>, reverse_mul: i32) -> Self {
        Self {
            values,
            reverse_mul,
        }
    }
}

impl DocComparator for DocComparatorImplLong {
    fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32 {
        self.reverse_mul * self.values[doc_id1].cmp(&self.values[doc_id2]).to_int()
    }
}
pub struct LongComparableProvider<N>
where
    N: NumericDocValues,
{
    values: N,
    missing_value: i64,
}

impl<N> LongComparableProvider<N>
where
    N: NumericDocValues,
{
    pub fn new(values: N, missing_value: i64) -> Self {
        Self {
            values,
            missing_value,
        }
    }
}

impl<N> ComparableProvider for LongComparableProvider<N>
where
    N: NumericDocValues,
{
    fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64> {
        if self.values.advance_exact(doc_id)? {
            Ok(self.values.long_value()?)
        } else {
            Ok(self.missing_value)
        }
    }
}

// FloatSorter
/// Sorts documents based on float values from a NumericDocValues instance
pub struct FloatSorter<NP> {
    provider_name: String,
    missing_value: Option<f32>,
    reverse_mul: i32,
    values_provider: NP,
}

impl<NP> FloatSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    pub fn new(
        provider_name: String,
        missing_value: Option<MissingValueEnum>,
        reverse: bool,
        values_provider: NP,
    ) -> Result<Self> {
        let missing_value = if let Some(mv) = missing_value {
            match mv {
                MissingValueEnum::Float(value) => Some(value),
                _ => {
                    return Err(LuceneError::illegal_state(
                        "Missing value type mismatch for Float.",
                    ));
                },
            }
        } else {
            None
        };
        Ok(Self {
            provider_name,
            missing_value,
            reverse_mul: if reverse { -1 } else { 1 },
            values_provider,
        })
    }
}

impl<NP> IndexSorter for FloatSorter<NP>
where
    NP: NumericDocValuesProvider,
{
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }

    type ComparableProvider<LR>
        = FloatComparableProvider<ProviderNumeric<NP, LR>>
    where
        LR: LeafReader;

    fn get_comparable_providers_per_reader<LR>(
        &self,
        reader: &LR,
        _reader_index: usize,
        formated_missing_value: &MissingValueEnum,
        _ordinal_map: Option<&OrdinalMap>,
    ) -> Result<Self::ComparableProvider<LR>>
    where
        LR: LeafReader,
    {
        let missing_value_bits = match formated_missing_value {
            MissingValueEnum::Int(value) => value,
            _ => {
                return Err(LuceneError::illegal_state(
                    "formated Missing value type mismatch for i32.",
                ));
            },
        };
        let values = self.values_provider.get(reader)?;
        Ok(FloatComparableProvider::new(values, *missing_value_bits))
    }

    fn get_missing_value(&self) -> MissingValueEnum {
        (self.missing_value.unwrap_or(0.0).to_bits() as i32).into()
    }

    type DocComparator = DocComparatorImplFloat;

    fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        let mut dvs = self.values_provider.get(leaf_reader)?;
        let mut values = vec![0f32; max_doc as usize];
        if let Some(mv) = self.missing_value {
            values.fill(mv);
        }
        loop {
            let doc_id = dvs.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            let bits = dvs.long_value()? as u32;
            values[doc_id as usize] = f32::from_bits(bits);
        }
        Ok(DocComparatorImplFloat::new(values, self.reverse_mul))
    }
}

pub struct DocComparatorImplFloat {
    values: Vec<f32>,
    reverse_mul: i32,
}

impl DocComparatorImplFloat {
    pub fn new(values: Vec<f32>, reverse_mul: i32) -> Self {
        Self {
            values,
            reverse_mul,
        }
    }
}

impl DocComparator for DocComparatorImplFloat {
    fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32 {
        let v1 = self.values[doc_id1];
        let v2 = self.values[doc_id2];
        let ord = v1.total_cmp(&v2).to_int();
        self.reverse_mul * ord
    }
}
pub struct FloatComparableProvider<N>
where
    N: NumericDocValues,
{
    values: N,
    missing_value_bits: i32,
}

impl<N> FloatComparableProvider<N>
where
    N: NumericDocValues,
{
    pub fn new(values: N, missing_value_bits: i32) -> Self {
        Self {
            values,
            missing_value_bits,
        }
    }
}

impl<N> ComparableProvider for FloatComparableProvider<N>
where
    N: NumericDocValues,
{
    fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64> {
        let v = if self.values.advance_exact(doc_id)? {
            self.values.long_value()?.try_convert()?
        } else {
            self.missing_value_bits
        };
        Ok(NumericUtils::sortable_float_bits(v) as i64)
    }
}

// StringSorter
/// Sorts documents based on short values from a NumericDocValues instance
pub struct StringSorter<SP> {
    provider_name: String,
    missing_value: Option<MissingValueEnum>,
    reverse_mul: i32,
    values_provider: SP,
}

impl<SP> StringSorter<SP>
where
    SP: SortedDocValuesProvider,
{
    pub fn new(
        provider_name: String,
        missing_value: Option<MissingValueEnum>,
        reverse: bool,
        values_provider: SP,
    ) -> Self {
        Self {
            provider_name,
            missing_value,
            reverse_mul: if reverse { -1 } else { 1 },
            values_provider,
        }
    }
}

impl<SP> IndexSorter for StringSorter<SP>
where
    SP: SortedDocValuesProvider,
{
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }

    type ComparableProvider<LR>
        = StringComparableProvider<ProviderString<SP, LR>>
    where
        LR: LeafReader;

    fn get_comparable_providers_per_reader<LR>(
        &self,
        reader: &LR,
        reader_index: usize,
        formated_missing_value: &MissingValueEnum,
        ordinal_map: Option<&OrdinalMap>,
    ) -> Result<Self::ComparableProvider<LR>>
    where
        LR: LeafReader,
    {
        let missing_ord = match formated_missing_value {
            MissingValueEnum::Int(value) => value,
            _ => {
                return Err(LuceneError::illegal_state(
                    "formated Missing value type mismatch for i32.",
                ));
            },
        };
        match ordinal_map {
            Some(omap) => {
                let global_ords = omap.get_global_ords(reader_index).clone();
                let reader_values = self.values_provider.get(reader)?;
                Ok(StringComparableProvider {
                    reader_values,
                    global_ords,
                    missing_ord: *missing_ord,
                })
            },
            None => Err(LuceneError::illegal_state(
                "ordinal_map is required for StringSorter.",
            )),
        }
    }

    fn get_ordinal_map<LR>(&self, readers: &[LR]) -> Result<Option<OrdinalMap>>
    where
        LR: LeafReader,
    {
        let mut values = Vec::with_capacity(readers.len());
        for reader in readers {
            values.push(self.values_provider.get(reader)?);
        }
        Ok(Some(OrdinalMap::build_from_sorted(
            None,
            values.as_mut(),
            PackedInts::DEFAULT,
        )?))
    }

    fn get_missing_value(&self) -> MissingValueEnum {
        match self.missing_value {
            Some(MissingValueEnum::StringLast) => i32::MAX.into(),
            _ => i32::MIN.into(),
        }
    }

    type DocComparator = DocComparatorImplString;

    fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        let mut sorted = self.values_provider.get(leaf_reader)?;
        let missing_ord = match self.missing_value {
            Some(MissingValueEnum::StringLast) => i32::MAX as usize,
            _ => i32::MIN as usize,
        };

        let mut ords = vec![missing_ord; max_doc as usize];
        let mut doc_id;
        loop {
            doc_id = sorted.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            ords[doc_id as usize] = sorted.ord_value()? as usize;
        }
        Ok(DocComparatorImplString::new(ords, self.reverse_mul))
    }
}
pub struct StringComparableProvider<SDV>
where
    SDV: SortedDocValues,
{
    reader_values: SDV,
    global_ords: Rc<SegmentToGlobalOrds>,
    missing_ord: i32,
}

impl<SDV> ComparableProvider for StringComparableProvider<SDV>
where
    SDV: SortedDocValues,
{
    fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64> {
        if self.reader_values.advance_exact(doc_id)? {
            let seg_ord = self.reader_values.ord_value()?;
            Ok(self.global_ords.get(seg_ord as usize)?)
        } else {
            Ok(self.missing_ord as i64)
        }
    }
}

pub struct DocComparatorImplString {
    ords: Vec<usize>,
    reverse_mul: i32,
}

impl DocComparatorImplString {
    pub fn new(ords: Vec<usize>, reverse_mul: i32) -> Self {
        Self { ords, reverse_mul }
    }
}

impl DocComparator for DocComparatorImplString {
    fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32 {
        let o1 = self.ords[doc_id1];
        let o2 = self.ords[doc_id2];
        let cmp = o1.cmp(&o2).to_int();
        self.reverse_mul * cmp
    }
}

/// Used for sorting documents across segments
pub trait ComparableProvider {
    /// Returns a long so that the natural ordering of long values matches the ordering of doc IDs for the given comparator
    fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64>;
}
macro_rules! either_comparable_provider {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> ComparableProvider for $name<$( $T ),+>
        where
            $( $T: ComparableProvider ),+
        {
            #[inline]
            fn get_as_comparable_long(&mut self, doc_id: i32) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.get_as_comparable_long(doc_id), )+
                }
            }
        }
    };
}
either_comparable_provider!(pub ComparableProviderEnum3 { SortedNumeric: A, SortedSet: B, Sorter: C});
either_comparable_provider!(pub ComparableProviderEnum5 { Int: A, Long: B, Float: C, Double: D, String: E });
either_comparable_provider!(pub ComparableProviderEnum4 { Int: A, Long: B, Float: C, Double: D});
pub type CPEnumType1<NP, LR, SP> = ComparableProviderEnum5<
    IntComparableProvider<ProviderNumeric<NP, LR>>,
    LongComparableProvider<ProviderNumeric<NP, LR>>,
    FloatComparableProvider<ProviderNumeric<NP, LR>>,
    DoubleComparableProvider<ProviderNumeric<NP, LR>>,
    StringComparableProvider<ProviderString<SP, LR>>,
>;
pub type CPEnumType2<NP, LR> = ComparableProviderEnum4<
    IntComparableProvider<ProviderNumeric<NP, LR>>,
    LongComparableProvider<ProviderNumeric<NP, LR>>,
    FloatComparableProvider<ProviderNumeric<NP, LR>>,
    DoubleComparableProvider<ProviderNumeric<NP, LR>>,
>;

/// A comparator of doc IDs, used for sorting documents within a segment
pub trait DocComparator {
    /// Compare docID1 against docID2.
    fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32;
}
macro_rules! either_doc_comparator {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> DocComparator for $name<$( $T ),+>
        where
            $( $T: DocComparator ),+
        {
            #[inline]
            fn compare(&self, doc_id1: usize, doc_id2: usize) -> i32 {
                match self {
                    $( Self::$Variant(inner) => inner.compare(doc_id1, doc_id2), )+
                }
            }
        }
    };
}
either_doc_comparator!(pub DocComparatorEnum2 { A: A, B: B });
either_doc_comparator!(pub DocComparatorEnum5 { Int: A, Long: B, Float: C, Double: D, String: E });
pub type DocComparatorImpl = DocComparatorEnum5<
    DocComparatorImplInt,
    DocComparatorImplLong,
    DocComparatorImplFloat,
    DocComparatorImplDouble,
    DocComparatorImplString,
>;
pub type ProviderNumeric<N, LR> = <N as NumericDocValuesProvider>::NumericDocValues<LR>;
pub type ProviderString<S, LR> = <S as SortedDocValuesProvider>::SortedDocValues<LR>;
/// Provide a NumericDocValues instance for a LeafReader
pub trait NumericDocValuesProvider {
    /// Returns the numeric value for the given doc ID.
    type NumericDocValues<LR>: NumericDocValues
    where
        LR: LeafReader;
    /// Returns the NumericDocValues instance for this LeafReader
    fn get<LR>(&self, leaf_reader: &LR) -> Result<Self::NumericDocValues<LR>>
    where
        LR: LeafReader;
}
/// Provide a SortedDocValues instance for a LeafReader
pub trait SortedDocValuesProvider {
    type SortedDocValues<LR>: SortedDocValues
    where
        LR: LeafReader;
    /// Returns the SortedDocValues instance for this LeafReader
    fn get<LR>(&self, leaf_reader: &LR) -> Result<Self::SortedDocValues<LR>>
    where
        LR: LeafReader;
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::core::index::{BytesRef, BytesRefBuilder};
    use crate::core::util::bytes_ref_comparator::{BytesRefComparator, Natural};
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::stable_string_sorter::{StableStringSorter, StableStringSorterBase};
    use crate::core::util::{
        MSBRadixSorterBase, NaturalOrder, SliceCopyOps, Sorter, StringSorter, StringSorterBase,
    };
    use crate::test::util::common_method::assert_vecs_equal;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, random};
    use crate::test::util::test_util::TestUtil;

    #[allow(dead_code)] // for quick search
    struct TestStringSorter;

    fn test(refs: Vec<BytesRef<Vec<u8>>>, len: usize) -> Result<()> {
        test_impl(refs.clone(), len, Natural::default())?;
        test_impl(refs.clone(), len, NaturalOrder)?;
        test_stable(refs.clone(), len, Natural::default())?;
        test_stable(refs.clone(), len, NaturalOrder)?;
        Ok(())
    }

    fn test_impl(
        refs: Vec<BytesRef<Vec<u8>>>,
        len: usize,
        comparator: impl BytesRefComparator,
    ) -> Result<()> {
        let mut expected: Vec<BytesRef<Vec<u8>>> = refs.clone();
        expected.sort();
        let delegate = StringSorterTestImpl::new(refs.clone());
        let mut string_sorter = StringSorter::new(delegate, comparator);
        string_sorter.sort(0, len)?;

        assert_vecs_equal(&expected, &string_sorter.get_delegate().refs);
        Ok(())
    }

    fn test_stable(
        refs: Vec<BytesRef<Vec<u8>>>,
        len: usize,
        comparator: impl BytesRefComparator,
    ) -> Result<()> {
        let mut expected: Vec<BytesRef<Vec<u8>>> = refs[..len].to_vec();
        let mut actual = refs[..len].to_vec();
        expected.sort();

        let actual_before_sorted = actual.clone();
        let mut ord: Vec<i32> = (0..len).map(|i| i as i32).collect();
        let ord_len = ord.len();
        let delegate = StableStringSorterTestImpl {
            tmp: vec![0; ord_len],
            ord: &mut ord,
            refs: &mut actual,
        };
        let string_sorter = StableStringSorter::new(delegate);
        let mut stable_string_sorter = StringSorter::new(string_sorter, comparator);
        stable_string_sorter.sort(0, len)?;
        // `actual` is not sorted, but `ord` is sorted
        assert_vecs_equal(&actual_before_sorted, &actual);
        for i in 0..len {
            assert_eq!(
                &expected[i], &refs[ord[i] as usize],
                "Mismatch at index {}: expected {:?}, found {:?}",
                i, &expected[i], &refs[ord[i] as usize]
            );

            if i > 0 && expected[i] == expected[i - 1] {
                assert!(
                    ord[i] > ord[i - 1],
                    "Not stable: ord[{}] <= ord[{}]",
                    i,
                    i - 1
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_empty() -> Result<()> {
        let mut random = random();
        let len = random.random_range(0..5);
        let refs: Vec<BytesRef<Vec<u8>>> = (0..len).map(|_| BytesRef::default()).collect();
        test(refs, 0)
    }

    #[test]
    fn test_one_value() -> Result<()> {
        let mut random = random();
        let bytes = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        test(vec![bytes], 1)
    }

    #[test]
    fn test_two_values() -> Result<()> {
        let mut random = random();
        let bytes1 = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        let bytes2 = BytesRef::from_string(&TestUtil::random_simple_string(&mut random));
        test(vec![bytes1, bytes2], 2)
    }

    fn test_random_impl<R: Rng + ?Sized>(
        common_prefix_len: usize,
        max_len: usize,
        random: &mut R,
    ) -> Result<()> {
        let mut common_prefix = vec![0u8; common_prefix_len];
        random.fill_bytes(&mut common_prefix);
        let len = random.random_range(0..100000);

        let mut bytes: Vec<BytesRef<Vec<u8>>> =
            Vec::with_capacity(len + random.random_range(0..50));
        for _ in 0..len {
            let mut b = vec![0u8; common_prefix_len + random.random_range(0..max_len)];
            random.fill_bytes(&mut b[common_prefix_len..]);
            b.copy_from(&common_prefix, 0);
            bytes.push(BytesRef::from_bytes(b));
        }

        test(bytes, len)
    }
    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3);
        for _ in 0..num_iters {
            test_random_impl(0, 10, &mut random)?;
        }
        Ok(())
    }
    #[test]
    fn test_random_with_lots_of_duplicates() -> Result<()> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3);
        for _ in 0..num_iters {
            test_random_impl(0, 2, &mut random)?;
        }
        Ok(())
    }
    #[test]
    fn test_random_with_shared_prefix() -> Result<()> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3);
        for _ in 0..num_iters {
            let shared_prefix_len = TestUtil::next_usize(&mut random, 1, 30);
            test_random_impl(shared_prefix_len, 10, &mut random)?;
        }
        Ok(())
    }
    #[test]
    fn test_random_with_shared_prefix_and_lots_of_duplicates() -> Result<()> {
        let mut random = random();
        let num_iters = at_least(&mut random, 3);
        for _ in 0..num_iters {
            let shared_prefix_len = TestUtil::next_usize(&mut random, 1, 30);
            test_random_impl(shared_prefix_len, 2, &mut random)?;
        }
        Ok(())
    }

    struct StringSorterTestImpl {
        refs: Vec<BytesRef<Vec<u8>>>,
    }

    impl StringSorterTestImpl {
        fn new(refs: Vec<BytesRef<Vec<u8>>>) -> Self {
            Self { refs }
        }
    }
    impl Sorter for StringSorterTestImpl {
        fn swap(&mut self, i: usize, j: usize) -> Result<()> {
            self.refs.swap(i, j);
            Ok(())
        }
    }
    impl StringSorterBase for StringSorterTestImpl {
        fn get(
            &mut self,
            _builder: &mut BytesRefBuilder<Vec<u8>>,
            result: &mut BytesRef<Vec<u8>>,
            i: usize,
        ) -> Result<()> {
            let ref_item = &self.refs[i];
            result.offset = ref_item.offset;
            result.length = ref_item.length;
            result.bytes = ref_item.bytes.clone();
            Ok(())
        }
    }

    struct StableStringSorterTestImpl<'a> {
        tmp: Vec<i32>,
        ord: &'a mut Vec<i32>,
        refs: &'a mut [BytesRef<Vec<u8>>],
    }

    impl StringSorterBase for StableStringSorterTestImpl<'_> {
        fn get(
            &mut self,
            _builder: &mut BytesRefBuilder<Vec<u8>>,
            result: &mut BytesRef<Vec<u8>>,
            i: usize,
        ) -> Result<()> {
            let ref_item = &self.refs[self.ord[i] as usize];
            result.offset = ref_item.offset;
            result.length = ref_item.length;
            result.bytes = ref_item.bytes.clone();
            Ok(())
        }
    }

    impl StableStringSorterBase for StableStringSorterTestImpl<'_> {
        fn save(&mut self, i: usize, j: usize) {
            self.tmp[j] = self.ord[i];
        }

        fn restore(&mut self, i: usize, j: usize) {
            self.ord.copy_from(&self.tmp[i..j], i);
        }
    }
    impl Sorter for StableStringSorterTestImpl<'_> {
        fn swap(&mut self, i: usize, j: usize) -> Result<()> {
            self.ord.swap(i, j);
            Ok(())
        }
    }
    impl MSBRadixSorterBase for StableStringSorterTestImpl<'_> {}
}
