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
use crate::core::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::core::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::{BinaryDocValues, Either2BinaryDocValues};
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::doc_values::{DocValues, EmptyNumeric};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{
    LRBinaryDocValues, LRNormNumericDocValues, LRNumericDocValues, LRSortedNumericDocValues,
    LeafReader,
};
use crate::core::index::numeric_doc_values::{Either2NumericDocValues, NumericDocValues};
use crate::core::index::ordinal_map::OrdinalMap;
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_doc_values_terms_enum::SortedDocValuesTermsEnum;
use crate::core::index::sorted_numeric_doc_values::{
    Either2SortedNumericDocValues, SortedNumericDocValues,
};
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_terms_enum::SortedSetDocValuesTermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::ToUsizeExact;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_values::LongValues;
use std::borrow::Cow;

pub struct MultiDocValues;

pub type MultiNormNumericDocValues<CR> = Either2NumericDocValues<
    LRNormNumericDocValues<<CR as CompositeReader>::LeafReader>,
    NumericDocValuesImpl<CR>,
>;
pub type MultiNumericDocValues<CR> = Either2NumericDocValues<
    LRNumericDocValues<<CR as CompositeReader>::LeafReader>,
    NumericDocValuesImpl1<CR>,
>;
pub type MultiBinaryDocValues<CR> = Either2BinaryDocValues<
    LRBinaryDocValues<<CR as CompositeReader>::LeafReader>,
    BinaryDocValuesImpl<CR>,
>;
pub type MultiSortedNumericDocValues<CR> = Either2SortedNumericDocValues<
    LRSortedNumericDocValues<<CR as CompositeReader>::LeafReader>,
    SortedNumericDocValuesImpl<CR>,
>;

impl MultiDocValues {
    pub fn get_norm_values<CR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiNormNumericDocValues<CR>>>
    where
        CR: CompositeReader,
    {
        let reader = get_context(reader)?;
        let leaves = reader.leaves()?;
        let size = leaves.len();

        if size == 0 {
            return Ok(None);
        } else if size == 1 {
            return match leaves[0].reader().get_norm_values(field)? {
                Some(v) => Ok(Some(MultiNormNumericDocValues::A(v))),
                None => Ok(None),
            };
        }
        // Check if any of the leaf reader which has this field has norms.
        let mut norm_found = false;
        for leaf in leaves.iter() {
            if let Some(info) = leaf.reader().get_field_infos()?.field_info_by_name(field)
                && info.has_norms()
            {
                norm_found = true;
                break;
            }
        }

        if !norm_found {
            return Ok(None);
        }
        Ok(Some(MultiNormNumericDocValues::B(
            NumericDocValuesImpl::new(reader, field.to_string()),
        )))
    }

    pub fn get_numeric_values<CR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiNumericDocValues<CR>>>
    where
        CR: CompositeReader,
    {
        let reader = get_context(reader)?;
        let leaves = reader.leaves()?;
        let size = leaves.len();

        if size == 0 {
            return Ok(None);
        } else if size == 1 {
            return match leaves[0].reader().get_numeric_doc_values(field)? {
                Some(v) => Ok(Some(MultiNumericDocValues::A(v))),
                None => Ok(None),
            };
        }

        let mut any_real = false;
        for leaf in leaves.iter() {
            if let Some(info) = leaf.reader().get_field_infos()?.field_info_by_name(field)
                && *info.get_doc_values_type() == DocValuesType::Numeric
            {
                any_real = true;
                break;
            }
        }

        if !any_real {
            return Ok(None);
        }

        Ok(Some(MultiNumericDocValues::B(NumericDocValuesImpl1::new(
            reader,
            field.to_string(),
        ))))
    }

    pub fn get_binary_values<CR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiBinaryDocValues<CR>>>
    where
        CR: CompositeReader,
    {
        let reader = get_context(reader)?;
        let leaves = reader.leaves()?;
        let size = leaves.len();

        if size == 0 {
            return Ok(None);
        } else if size == 1 {
            return match leaves[0].reader().get_binary_doc_values(field)? {
                Some(v) => Ok(Some(MultiBinaryDocValues::A(v))),
                None => Ok(None),
            };
        }

        let mut any_real = false;
        for leaf in leaves.iter() {
            if let Some(info) = leaf.reader().get_field_infos()?.field_info_by_name(field)
                && *info.get_doc_values_type() == DocValuesType::Binary
            {
                any_real = true;
                break;
            }
        }

        if !any_real {
            return Ok(None);
        }

        Ok(Some(MultiBinaryDocValues::B(BinaryDocValuesImpl::new(
            reader,
            field.to_string(),
        ))))
    }
    pub fn get_sorted_numeric_values<CR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiSortedNumericDocValues<CR>>>
    where
        CR: CompositeReader,
    {
        let reader = get_context(reader)?;
        let leaves = reader.leaves()?;
        let size = leaves.len();

        if size == 0 {
            return Ok(None);
        } else if size == 1 {
            return match leaves[0].reader().get_sorted_numeric_doc_values(field)? {
                Some(v) => Ok(Some(MultiSortedNumericDocValues::A(v))),
                None => Ok(None),
            };
        }

        let mut any_real = false;
        let mut values = Vec::with_capacity(size);
        let mut total_cost = 0i64;

        for leaf in leaves.iter() {
            let v = leaf.reader().get_sorted_numeric_doc_values(field)?;
            let dv = match v {
                Some(v) => {
                    any_real = true;
                    Either2SortedNumericDocValues::B(v)
                },
                None => Either2SortedNumericDocValues::A(DocValues::empty_sorted_numeric()?),
            };

            total_cost += dv.cost()?;
            values.push(dv);
        }

        if !any_real {
            return Ok(None);
        }

        Ok(Some(MultiSortedNumericDocValues::B(
            SortedNumericDocValuesImpl::new(reader, values, field.to_string(), total_cost),
        )))
    }
}
/// Implements SortedDocValues over n subs, using an OrdinalMap
pub struct MultiSortedDocValues<S>
where
    S: SortedDocValues,
{
    /// docbase for each leaf: parallel with [`values`]
    pub doc_starts: Vec<i32>,
    /// leaf values
    pub values: Vec<S>,
    /// ordinal map mapping ords from `values` to global ord space
    pub mapping: OrdinalMap,

    total_cost: i64,
    next_leaf: usize,
    current_values: Option<usize>,
    current_doc_start: i32,
    doc_id: i32,
}

impl<S> MultiSortedDocValues<S>
where
    S: SortedDocValues,
{
    pub fn new(doc_starts: Vec<i32>, values: Vec<S>, mapping: OrdinalMap, total_cost: i64) -> Self {
        Self {
            doc_starts,
            values,
            mapping,
            total_cost,
            next_leaf: 0,
            current_values: None,
            current_doc_start: 0,
            doc_id: -1,
        }
    }
}

impl<S> DocValuesIterator for MultiSortedDocValues<S>
where
    S: SortedDocValues,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index(target_doc_id, &self.doc_starts);
        if reader_index < 0 {
            return Err(LuceneError::illegal_state("reader_index should be >= 0"));
        }
        let reader_index = reader_index as usize;
        if reader_index >= self.next_leaf {
            if reader_index == self.values.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }
            self.current_doc_start = self.doc_starts[reader_index];
            self.current_values = Some(reader_index);
            self.next_leaf = reader_index + 1;
        }

        self.doc_id = target_doc_id;

        let idx = match self.current_values {
            None => return Ok(false),
            Some(i) => i,
        };

        // delegate to leaf-level advanceExact()
        let exists = self.values[idx].advance_exact(target_doc_id - self.current_doc_start)?;

        Ok(exists)
    }
}

impl<S> DocIdSetIterator for MultiSortedDocValues<S>
where
    S: SortedDocValues,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            while self.current_values.is_none() {
                if self.next_leaf == self.values.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }
                self.current_doc_start = self.doc_starts[self.next_leaf];
                self.current_values = Some(self.next_leaf);
                self.next_leaf += 1;
            }

            let new_doc_id = self.values[*self.current_values.as_ref().unwrap()].next_doc()?;

            if new_doc_id == NO_MORE_DOCS {
                self.current_values = None;
            } else {
                self.doc_id = self.current_doc_start + new_doc_id;
                return Ok(self.doc_id);
            }
        }
    }

    fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index(target_doc_id, &self.doc_starts);
        if reader_index < 0 {
            return Err(LuceneError::illegal_state("reader_index should be >= 0"));
        }
        let reader_index = reader_index as usize;
        if reader_index >= self.next_leaf {
            if reader_index == self.values.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }
            self.current_doc_start = self.doc_starts[reader_index];
            self.current_values = Some(reader_index);
            self.next_leaf = reader_index + 1;
        }

        let idx = *self.current_values.as_ref().unwrap();
        let new_doc_id = self.values[idx].advance(target_doc_id - self.current_doc_start)?;

        if new_doc_id == NO_MORE_DOCS {
            self.current_values = None;
            self.next_doc()
        } else {
            self.doc_id = self.current_doc_start + new_doc_id;
            Ok(self.doc_id)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.total_cost)
    }
}

impl<S> SortedDocValues for MultiSortedDocValues<S>
where
    S: SortedDocValues,
{
    fn ord_value(&mut self) -> Result<i32> {
        let seg_idx = match self.current_values {
            Some(i) => i,
            None => return Err(LuceneError::illegal_state("current_values is None")),
        };

        let local_ord = self.values[seg_idx].ord_value()? as i64;

        let global_ord = self
            .mapping
            .get_global_ords((self.next_leaf - 1) as i32)
            .get(local_ord)?;

        Ok(global_ord as i32)
    }

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        let sub_index = self
            .mapping
            .get_first_segment_number(ord as i64)?
            .to_usize_exact()?;
        let segment_ord = self.mapping.get_first_segment_ord(ord as i64)?.try_into()?;
        self.values[sub_index].lookup_ord(segment_ord)
    }

    fn get_value_count(&mut self) -> Result<i32> {
        Ok(self.mapping.get_value_count().try_into()?)
    }

    type TermsEnum<'a>
        = SortedDocValuesTermsEnum<'a, Self>
    where
        S: 'a;

    fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
        self.default_terms_enum()
    }
}

/// Implements SortedSetDocValues over N subs, using an OrdinalMap.
pub struct MultiSortedSetDocValues<T>
where
    T: SortedSetDocValues,
{
    /// docbase for each leaf: parallel with `values`
    pub doc_starts: Vec<i32>,

    /// leaf values
    pub values: Vec<T>,

    /// ordinal map mapping ords from `values` to global ord space
    pub mapping: OrdinalMap,

    total_cost: i64,
    next_leaf: usize,
    current_values: Option<usize>,
    current_doc_start: i32,
    doc_id: i32,
}

impl<T> MultiSortedSetDocValues<T>
where
    T: SortedSetDocValues,
{
    pub fn new(values: Vec<T>, doc_starts: Vec<i32>, mapping: OrdinalMap, total_cost: i64) -> Self {
        debug_assert_eq!(doc_starts.len(), values.len() + 1);

        Self {
            doc_starts,
            values,
            mapping,
            total_cost,
            next_leaf: 0,
            current_values: None,
            current_doc_start: 0,
            doc_id: -1,
        }
    }
}

impl<T> DocValuesIterator for MultiSortedSetDocValues<T>
where
    T: SortedSetDocValues,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index(target_doc_id, &self.doc_starts);
        if reader_index < 0 {
            return Err(LuceneError::illegal_state("reader_index should be >= 0"));
        }
        let reader_index = reader_index as usize;

        if reader_index >= self.next_leaf {
            if reader_index == self.values.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }
            self.current_doc_start = self.doc_starts[reader_index];
            self.current_values = Some(reader_index);
            self.next_leaf = reader_index + 1;
        }

        self.doc_id = target_doc_id;

        let idx = match self.current_values {
            None => return Ok(false),
            Some(i) => i,
        };

        self.values[idx].advance_exact(target_doc_id - self.current_doc_start)
    }
}

impl<T> DocIdSetIterator for MultiSortedSetDocValues<T>
where
    T: SortedSetDocValues,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            while self.current_values.is_none() {
                if self.next_leaf == self.values.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                self.current_doc_start = self.doc_starts[self.next_leaf];
                self.current_values = Some(self.next_leaf);
                self.next_leaf += 1;
            }

            let idx = *self.current_values.as_ref().unwrap();
            let new_doc_id = self.values[idx].next_doc()?;

            if new_doc_id == NO_MORE_DOCS {
                self.current_values = None;
            } else {
                self.doc_id = self.current_doc_start + new_doc_id;
                return Ok(self.doc_id);
            }
        }
    }

    fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index(target_doc_id, &self.doc_starts);
        if reader_index < 0 {
            return Err(LuceneError::illegal_state("reader_index should be >= 0"));
        }
        let reader_index = reader_index as usize;

        if reader_index >= self.next_leaf {
            if reader_index == self.values.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            self.current_doc_start = self.doc_starts[reader_index];
            self.current_values = Some(reader_index);
            self.next_leaf = reader_index + 1;
        }

        let idx = *self.current_values.as_ref().unwrap();
        let new_doc_id = self.values[idx].advance(target_doc_id - self.current_doc_start)?;

        if new_doc_id == NO_MORE_DOCS {
            self.current_values = None;
            self.next_doc()
        } else {
            self.doc_id = self.current_doc_start + new_doc_id;
            Ok(self.doc_id)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.total_cost)
    }
}

impl<T> SortedSetDocValues for MultiSortedSetDocValues<T>
where
    T: SortedSetDocValues,
{
    fn next_ord(&mut self) -> Result<i64> {
        let idx = match self.current_values {
            Some(i) => i,
            None => return Err(LuceneError::illegal_state("current_values is None")),
        };

        let segment_ord = self.values[idx].next_ord()?;
        let global = self
            .mapping
            .get_global_ords((self.next_leaf - 1) as i32)
            .get(segment_ord)?;

        Ok(global)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        let idx = self
            .current_values
            .ok_or_else(|| LuceneError::illegal_state("current_values is None"))?;
        self.values[idx].doc_value_count()
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        let sub_index = self
            .mapping
            .get_first_segment_number(ord)?
            .to_usize_exact()?;
        let segment_ord = self.mapping.get_first_segment_ord(ord)?;
        self.values[sub_index].lookup_ord(segment_ord)
    }

    fn get_value_count(&mut self) -> Result<i64> {
        Ok(self.mapping.get_value_count())
    }

    type TermsEnum<'a>
        = SortedSetDocValuesTermsEnum<'a, Self>
    where
        T: 'a;

    fn terms_enum(&mut self) -> Result<Self::TermsEnum<'_>> {
        self.default_terms_enum()
    }

    type SortedDocValues = DummySortedDocValues;
}

pub struct NumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    next_leaf: i32,
    current_values: Option<LRNormNumericDocValues<CR::LeafReader>>,
    reader: CompositeReaderContext<CR>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
}
impl<CR> NumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    pub fn new(reader: CompositeReaderContext<CR>, field: String) -> Self {
        Self {
            next_leaf: 0,
            current_values: None,
            reader,
            doc_id: -1,
            field,
            current_doc_base: 0,
        }
    }
}

impl<CR> DocValuesIterator for NumericDocValuesImpl<CR> where CR: CompositeReader {}

impl<CR> DocIdSetIterator for NumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        loop {
            if self.current_values.is_none() {
                if self.next_leaf as usize == leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                let leaf = &leaves[self.next_leaf as usize];
                self.current_doc_base = leaf.doc_base;
                self.current_values = leaf.reader().get_norm_values(&self.field)?;

                self.next_leaf += 1;
                continue;
            }

            let new_doc_id = self.current_values.as_mut().unwrap().next_doc()?;

            if new_doc_id == NO_MORE_DOCS {
                self.current_values = None;
            } else {
                self.doc_id = self.current_doc_base + new_doc_id;
                return Ok(self.doc_id);
            }
        }
    }

    fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values = leaf.reader().get_norm_values(&self.field)?;

            if self.current_values.is_none() {
                return self.next_doc();
            }

            self.next_leaf = (reader_index + 1) as i32;
        }

        let new_doc_id = self
            .current_values
            .as_mut()
            .unwrap()
            .advance(target_doc_id - self.current_doc_base)?;

        if new_doc_id == NO_MORE_DOCS {
            self.current_values = None;
            self.next_doc()
        } else {
            self.doc_id = self.current_doc_base + new_doc_id;
            Ok(self.doc_id)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl<CR> NumericDocValues for NumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current_values {
            Some(ref mut values) => values.long_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }
}

pub struct NumericDocValuesImpl1<CR>
where
    CR: CompositeReader,
{
    next_leaf: i32,
    current_values: Option<LRNumericDocValues<CR::LeafReader>>,
    reader: CompositeReaderContext<CR>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
}

impl<CR> NumericDocValuesImpl1<CR>
where
    CR: CompositeReader,
{
    pub fn new(reader: CompositeReaderContext<CR>, field: String) -> Self {
        Self {
            next_leaf: 0,
            current_values: None,
            reader,
            doc_id: -1,
            field,
            current_doc_base: 0,
        }
    }
}

impl<CR> DocValuesIterator for NumericDocValuesImpl1<CR>
where
    CR: CompositeReader,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        let leaves = self.reader.leaves()?;
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }

            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values = leaf.reader().get_numeric_doc_values(&self.field)?;
            self.next_leaf = (reader_index + 1) as i32;
        }

        self.doc_id = target_doc_id;

        match self.current_values {
            None => Ok(false),
            Some(ref mut v) => v.advance_exact(target_doc_id - self.current_doc_base),
        }
    }
}

impl<CR> DocIdSetIterator for NumericDocValuesImpl1<CR>
where
    CR: CompositeReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        loop {
            while self.current_values.is_none() {
                if self.next_leaf as usize == leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }
                let leaf = &leaves[self.next_leaf as usize];
                self.current_doc_base = leaf.doc_base;
                self.current_values = leaf.reader().get_numeric_doc_values(&self.field)?;
                self.next_leaf += 1;
            }

            let new_doc_id = self.current_values.as_mut().unwrap().next_doc()?;

            if new_doc_id == NO_MORE_DOCS {
                self.current_values = None;
            } else {
                self.doc_id = self.current_doc_base + new_doc_id;
                return Ok(self.doc_id);
            }
        }
    }

    fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }
            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values = leaf.reader().get_numeric_doc_values(&self.field)?;
            self.next_leaf = (reader_index + 1) as i32;

            if self.current_values.is_none() {
                return self.next_doc();
            }
        }

        let new_doc_id = self
            .current_values
            .as_mut()
            .unwrap()
            .advance(target_doc_id - self.current_doc_base)?;

        if new_doc_id == NO_MORE_DOCS {
            self.current_values = None;
            self.next_doc()
        } else {
            self.doc_id = self.current_doc_base + new_doc_id;
            Ok(self.doc_id)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}

impl<CR> NumericDocValues for NumericDocValuesImpl1<CR>
where
    CR: CompositeReader,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current_values {
            Some(ref mut values) => values.long_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }
}

pub struct BinaryDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    next_leaf: i32,
    current_values: Option<LRBinaryDocValues<CR::LeafReader>>,
    reader: CompositeReaderContext<CR>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
}

impl<CR> BinaryDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    pub fn new(reader: CompositeReaderContext<CR>, field: String) -> Self {
        Self {
            next_leaf: 0,
            current_values: None,
            reader,
            doc_id: -1,
            field,
            current_doc_base: 0,
        }
    }
}
impl<CR> DocValuesIterator for BinaryDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        let leaves = self.reader.leaves()?;
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }

            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values = leaf.reader().get_binary_doc_values(&self.field)?;
            self.next_leaf = (reader_index + 1) as i32;
        }

        self.doc_id = target_doc_id;

        match self.current_values {
            None => Ok(false),
            Some(ref mut v) => v.advance_exact(target_doc_id - self.current_doc_base),
        }
    }
}
impl<CR> DocIdSetIterator for BinaryDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        loop {
            while self.current_values.is_none() {
                if self.next_leaf as usize == leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                let leaf = &leaves[self.next_leaf as usize];
                self.current_doc_base = leaf.doc_base;
                self.current_values = leaf.reader().get_binary_doc_values(&self.field)?;
                self.next_leaf += 1;
            }

            let new_doc_id = self.current_values.as_mut().unwrap().next_doc()?;

            if new_doc_id == NO_MORE_DOCS {
                self.current_values = None;
            } else {
                self.doc_id = self.current_doc_base + new_doc_id;
                return Ok(self.doc_id);
            }
        }
    }

    fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values = leaf.reader().get_binary_doc_values(&self.field)?;
            self.next_leaf = (reader_index + 1) as i32;

            if self.current_values.is_none() {
                return self.next_doc();
            }
        }

        let new_doc_id = self
            .current_values
            .as_mut()
            .unwrap()
            .advance(target_doc_id - self.current_doc_base)?;

        if new_doc_id == NO_MORE_DOCS {
            self.current_values = None;
            self.next_doc()
        } else {
            self.doc_id = self.current_doc_base + new_doc_id;
            Ok(self.doc_id)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(0)
    }
}
impl<CR> BinaryDocValues for BinaryDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        match self.current_values {
            Some(ref mut values) => values.binary_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }
}
pub struct SortedNumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    next_leaf: i32,
    current_values_index: Option<usize>,
    values: Vec<
        Either2SortedNumericDocValues<
            SingletonSortedNumericDocValues<EmptyNumeric>,
            LRSortedNumericDocValues<CR::LeafReader>,
        >,
    >,
    reader: CompositeReaderContext<CR>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
    final_total_cost: i64,
}
impl<CR> SortedNumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    pub fn new(
        reader: CompositeReaderContext<CR>,
        values: Vec<
            Either2SortedNumericDocValues<
                SingletonSortedNumericDocValues<EmptyNumeric>,
                LRSortedNumericDocValues<CR::LeafReader>,
            >,
        >,
        field: String,
        total_cost: i64,
    ) -> Self {
        Self {
            next_leaf: 0,
            current_values_index: None,
            values,
            reader,
            doc_id: -1,
            field,
            current_doc_base: 0,
            final_total_cost: total_cost,
        }
    }
}
impl<CR> DocValuesIterator for SortedNumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        let leaves = self.reader.leaves()?;
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }

            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values_index = Some(reader_index);
            self.next_leaf = (reader_index + 1) as i32;
        }

        self.doc_id = target_doc_id;
        let current_values = &mut self.values[*self.current_values_index.as_ref().unwrap()];
        current_values.advance_exact(target_doc_id - self.current_doc_base)
    }
}
impl<CR> DocIdSetIterator for SortedNumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        loop {
            if self.current_values_index.is_none() {
                if self.next_leaf as usize == leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                let leaf = &leaves[self.next_leaf as usize];
                self.current_doc_base = leaf.doc_base;
                self.current_values_index = Some(self.next_leaf as usize);
                self.next_leaf += 1;
            }

            let new_doc = self.values[*self.current_values_index.as_ref().unwrap()].next_doc()?;

            if new_doc == NO_MORE_DOCS {
                self.current_values_index = None;
            } else {
                self.doc_id = self.current_doc_base + new_doc;
                return Ok(self.doc_id);
            }
        }
    }

    fn advance(&mut self, target_doc_id: i32) -> Result<i32> {
        let leaves = self.reader.leaves()?;
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, leaves);

        if reader_index >= self.next_leaf as usize {
            if reader_index == leaves.len() {
                self.current_values_index = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            let leaf = &leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values_index = Some(reader_index);
            self.next_leaf = (reader_index + 1) as i32;
        }

        let new_doc = self.values[*self.current_values_index.as_ref().unwrap()]
            .advance(target_doc_id - self.current_doc_base)?;

        if new_doc == NO_MORE_DOCS {
            self.current_values_index = None;
            self.next_doc()
        } else {
            self.doc_id = self.current_doc_base + new_doc;
            Ok(self.doc_id)
        }
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.final_total_cost)
    }
}
impl<CR> SortedNumericDocValues for SortedNumericDocValuesImpl<CR>
where
    CR: CompositeReader,
{
    fn doc_value_count(&mut self) -> Result<i32> {
        match self.current_values_index {
            Some(ref v) => self.values[*v].doc_value_count(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }

    fn next_value(&mut self) -> Result<i64> {
        match self.current_values_index {
            Some(ref v) => self.values[*v].next_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }

    type NumericDocValues = DummyNumericDocValues;
}

#[cfg(test)]
mod tests {
    use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
    use crate::core::document::document::Document;
    use crate::core::document::field::FieldBase;
    use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
    use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
    use crate::core::index::BytesRef;
    use crate::core::index::binary_doc_values::BinaryDocValues;
    use crate::core::index::doc_values_iterator::DocValuesIterator;
    use crate::core::index::index_reader::IndexReader;
    use crate::core::index::leaf_reader::LeafReader;
    use crate::core::index::multi_doc_values::MultiDocValues;
    use crate::core::index::numeric_doc_values::NumericDocValues;
    use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, get_only_leaf_reader, is_night_mode, new_directory_shared,
        new_index_writer_config, random,
    };
    use crate::test::util::test_util::TestUtil;
    use rand::Rng;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestMultiDocValues;

    #[test]
    fn test_numerics() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mut doc = Document::new();

        let mut field = NumericDocValuesField::new("numbers", 0i64);
        doc.add(field.clone());
        // TODO 这里需要使用带分词器的构造方法
        // TODO 合并策略未实现
        let iwc = new_index_writer_config(&mut random);

        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

        let num_docs = if is_night_mode() {
            at_least(&mut random, 500)
        } else {
            at_least(&mut random, 50)
        };

        for _ in 0..num_docs {
            let value = random.random();
            field.set_long_value(value)?;
            iw.add_document(doc.clone())?;

            if random.random_range(0..17) == 0 {
                // TODO 由于没有实现force_merge 所以 我们只生成一个段
                // iw.commit()?;
            }
        }
        // TODO 由于没有实现force_merge 所以 我们只生成一个段
        iw.commit()?;

        let ir = Arc::new(iw.get_reader()?);
        // TODO force_merge未实现
        // iw.force_merge(1)?;
        let ir2 = Arc::new(iw.get_reader()?);
        let merged = get_only_leaf_reader(ir2.clone())?;
        iw.close()?;

        let mut multi =
            MultiDocValues::get_numeric_values(ir.clone(), "numbers")?.expect("multi should exist");
        let mut single = merged
            .get_numeric_doc_values("numbers")?
            .expect("single dv should exist");

        for i in 0..num_docs {
            assert_eq!(i, multi.next_doc()?);
            assert_eq!(i, single.next_doc()?);
            assert_eq!(single.long_value()?, multi.long_value()?);
        }

        test_random_advance(
            &mut random,
            &mut merged.get_numeric_doc_values("numbers")?.unwrap(),
            &mut MultiDocValues::get_numeric_values(ir.clone(), "numbers")?.unwrap(),
        )?;

        test_random_advance_exact(
            &mut random,
            &mut merged.get_numeric_doc_values("numbers")?.unwrap(),
            &mut MultiDocValues::get_numeric_values(ir.clone(), "numbers")?.unwrap(),
            merged.max_doc()?,
        )?;
        Ok(())
    }
    #[test]
    fn test_binary() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;
        let mut doc = Document::new();

        let mut field = BinaryDocValuesField::new("bytes", BytesRef::new());
        doc.add(field.clone());

        // TODO 这里需要使用带分词器的构造方法
        // TODO 合并策略未实现
        let iwc = new_index_writer_config(&mut random);

        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

        let num_docs = if is_night_mode() {
            at_least(&mut random, 500)
        } else {
            at_least(&mut random, 50)
        };

        for _ in 0..num_docs {
            let s = TestUtil::random_unicode_string(&mut random);
            let bytes = BytesRef::from_string(&s);

            field.set_bytes_value(bytes)?;
            iw.add_document(doc.clone())?;

            if random.random_range(0..17) == 0 {
                // TODO 由于没有实现 force_merge 所以仅生成一个段
                // iw.commit()?;
            }
        }

        // TODO 由于没有实现 force_merge，所以最终仍然只有一个段
        iw.commit()?;

        let ir = Arc::new(iw.get_reader()?);

        // TODO force_merge 未实现
        // iw.force_merge(1)?;
        let ir2 = Arc::new(iw.get_reader()?);
        let merged = get_only_leaf_reader(ir2.clone())?;
        iw.close()?;

        let mut multi =
            MultiDocValues::get_binary_values(ir.clone(), "bytes")?.expect("multi should exist");
        let mut single = merged
            .get_binary_doc_values("bytes")?
            .expect("single should exist");

        for i in 0..num_docs {
            assert_eq!(i, multi.next_doc()?);
            assert_eq!(i, single.next_doc()?);

            let expected = single.binary_value()?.clone();
            let actual = multi.binary_value()?.clone();

            assert_eq!(expected, actual);
        }

        test_random_advance(
            &mut random,
            &mut merged.get_binary_doc_values("bytes")?.unwrap(),
            &mut MultiDocValues::get_binary_values(ir.clone(), "bytes")?.unwrap(),
        )?;

        test_random_advance_exact(
            &mut random,
            &mut merged.get_binary_doc_values("bytes")?.unwrap(),
            &mut MultiDocValues::get_binary_values(ir.clone(), "bytes")?.unwrap(),
            merged.max_doc()?,
        )?;

        Ok(())
    }

    #[test]
    fn test_sorted_numeric() -> Result<()> {
        let mut random = random();

        let dir = new_directory_shared(&mut random)?;

        // TODO 这里需要使用带分词器的构造方法
        // TODO 合并策略未实现
        let iwc = new_index_writer_config(&mut random);

        let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

        let num_docs = if is_night_mode() {
            at_least(&mut random, 500)
        } else {
            at_least(&mut random, 50)
        };

        for _ in 0..num_docs {
            let mut doc = Document::new();
            let num_values = random.random_range(0..5);

            for _ in 0..num_values {
                let v = TestUtil::next_long(&mut random, i64::MIN, i64::MAX);
                doc.add(SortedNumericDocValuesField::new("nums", v));
            }

            iw.add_document(doc)?;

            if random.random_range(0..17) == 0 {
                // TODO 没有实现 force_merge，因此只生成单段
                // iw.commit()?;
            }
        }

        // TODO 由于没有 force_merge，仍然只有一个段
        iw.commit()?;

        let ir = Arc::new(iw.get_reader()?);

        // TODO force_merge 未实现
        // iw.force_merge(1)?;
        let ir2 = Arc::new(iw.get_reader()?);
        let merged = get_only_leaf_reader(ir2.clone())?;
        iw.close()?;

        let mut multi_opt = MultiDocValues::get_sorted_numeric_values(ir.clone(), "nums")?;
        let mut single_opt = merged.get_sorted_numeric_doc_values("nums")?;

        match (multi_opt.as_mut(), single_opt.as_mut()) {
            (None, None) => {
                // pass
            },
            (Some(multi), Some(single)) => {
                for i in 0..num_docs {
                    if i > single.doc_id() {
                        assert_eq!(single.next_doc()?, multi.next_doc()?);
                    }

                    if i == single.doc_id() {
                        let single_count = single.doc_value_count()?;
                        let multi_count = multi.doc_value_count()?;
                        assert_eq!(single_count, multi_count);

                        for _ in 0..single_count {
                            let sv = single.next_value()?;
                            let mv = multi.next_value()?;
                            assert_eq!(sv, mv);
                        }
                    }
                }
            },
            _ => {
                unreachable!(
                    "multi and single SortedNumericDocValues mismatch: one is None and the other is Some"
                );
            },
        }

        test_random_advance(
            &mut random,
            &mut merged.get_sorted_numeric_doc_values("nums")?.unwrap(),
            &mut MultiDocValues::get_sorted_numeric_values(ir.clone(), "nums")?.unwrap(),
        )?;

        test_random_advance_exact(
            &mut random,
            &mut merged.get_sorted_numeric_doc_values("nums")?.unwrap(),
            &mut MultiDocValues::get_sorted_numeric_values(ir.clone(), "nums")?.unwrap(),
            merged.max_doc()?,
        )?;

        Ok(())
    }
    fn test_random_advance<I1, I2, R: Rng + ?Sized>(
        random: &mut R,
        iter1: &mut I1,
        iter2: &mut I2,
    ) -> Result<()>
    where
        I1: DocIdSetIterator,
        I2: DocIdSetIterator,
    {
        assert_eq!(iter1.doc_id(), -1);
        assert_eq!(iter2.doc_id(), -1);

        while iter1.doc_id() != NO_MORE_DOCS {
            if random.random_bool(0.5) {
                let v1 = iter1.next_doc()?;
                let v2 = iter2.next_doc()?;
                assert_eq!(v1, v2);
            } else {
                let target = iter1.doc_id() + TestUtil::next_int(random, 1, 100);
                let v1 = iter1.advance(target)?;
                let v2 = iter2.advance(target)?;
                assert_eq!(v1, v2);
            }
        }

        Ok(())
    }
    fn test_random_advance_exact<I1, I2, R>(
        random: &mut R,
        iter1: &mut I1,
        iter2: &mut I2,
        max_doc: i32,
    ) -> Result<()>
    where
        R: Rng + ?Sized,
        I1: DocValuesIterator,
        I2: DocValuesIterator,
    {
        let mut target = TestUtil::next_int(random, 0, max_doc.min(10));

        while target < max_doc {
            let exists1 = iter1.advance_exact(target)?;
            let exists2 = iter2.advance_exact(target)?;
            assert_eq!(exists1, exists2);

            target += TestUtil::next_int(random, 0, 10);
        }

        Ok(())
    }
    // TODO 还有其他test未完成
}
