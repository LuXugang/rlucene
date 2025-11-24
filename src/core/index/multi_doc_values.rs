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
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::{BinaryDocValues, Either2BinaryDocValues};
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::doc_values::{DocValues, EmptyNumeric};
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{
    LRBinaryDocValues, LRNormNumericDocValues, LRNumericDocValues, LRSortedNumericDocValues,
    LeafReader,
};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::numeric_doc_values::{Either2NumericDocValues, NumericDocValues};
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::core::index::sorted_numeric_doc_values::{
    Either2SortedNumericDocValues, SortedNumericDocValues,
};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::sync::Arc;

pub struct MultiDocValues;

pub type MultiNormNumericDocValues<LR> =
    Either2NumericDocValues<LRNormNumericDocValues<LR>, NumericDocValuesImpl<LR>>;
pub type MultiNumericDocValues<LR> =
    Either2NumericDocValues<LRNumericDocValues<LR>, NumericDocValuesImpl1<LR>>;
pub type MultiBinaryDocValues<LR> =
    Either2BinaryDocValues<LRBinaryDocValues<LR>, BinaryDocValuesImpl<LR>>;
pub type MultiSortedNumericDocValues<LR> =
    Either2SortedNumericDocValues<LRSortedNumericDocValues<LR>, SortedNumericDocValuesImpl<LR>>;

impl MultiDocValues {
    pub fn get_norm_values<CR, LR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiNormNumericDocValues<CR::LeafReader>>>
    where
        CR: CompositeReader + Clone,
        CR::LeafReader: LeafReader<ParentReader = CR>,
        LR: LeafReader,
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
        for leaf in leaves.as_slice() {
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
            NumericDocValuesImpl::new(leaves, field.to_string()),
        )))
    }

    pub fn get_numeric_values<CR, LR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiNumericDocValues<CR::LeafReader>>>
    where
        CR: CompositeReader + Clone,
        CR::LeafReader: LeafReader<ParentReader = CR>,
        LR: LeafReader,
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
        for leaf in leaves.as_slice() {
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
            leaves,
            field.to_string(),
        ))))
    }

    pub fn get_binary_values<CR, LR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiBinaryDocValues<CR::LeafReader>>>
    where
        CR: CompositeReader + Clone,
        CR::LeafReader: LeafReader<ParentReader = CR>,
        LR: LeafReader,
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
        for leaf in leaves.as_slice() {
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
            leaves,
            field.to_string(),
        ))))
    }
    pub fn get_sorted_numeric_values<CR, LR>(
        reader: CR,
        field: &str,
    ) -> Result<Option<MultiSortedNumericDocValues<CR::LeafReader>>>
    where
        CR: CompositeReader + Clone,
        CR::LeafReader: LeafReader<ParentReader = CR>,
        LR: LeafReader,
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

        for leaf in leaves.as_slice() {
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
            SortedNumericDocValuesImpl::new(leaves, values, field.to_string(), total_cost),
        )))
    }
}

pub struct NumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    next_leaf: i32,
    current_values: Option<LRNormNumericDocValues<LR>>,
    leaves: Vec<Arc<LeafReaderContext<LR>>>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
}
impl<LR> NumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    pub fn new(leaves: Vec<Arc<LeafReaderContext<LR>>>, field: String) -> Self {
        Self {
            next_leaf: 0,
            current_values: None,
            leaves,
            doc_id: -1,
            field,
            current_doc_base: 0,
        }
    }
}

impl<LR> DocValuesIterator for NumericDocValuesImpl<LR> where LR: LeafReader {}

impl<LR> DocIdSetIterator for NumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            if self.current_values.is_none() {
                if self.next_leaf as usize == self.leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                let leaf = &self.leaves[self.next_leaf as usize];
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
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            let leaf = &self.leaves[reader_index];
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

impl<LR> NumericDocValues for NumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current_values {
            Some(ref mut values) => values.long_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }
}

pub struct NumericDocValuesImpl1<LR>
where
    LR: LeafReader,
{
    next_leaf: i32,
    current_values: Option<LRNumericDocValues<LR>>,
    leaves: Vec<Arc<LeafReaderContext<LR>>>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
}

impl<LR> NumericDocValuesImpl1<LR>
where
    LR: LeafReader,
{
    pub fn new(leaves: Vec<Arc<LeafReaderContext<LR>>>, field: String) -> Self {
        Self {
            next_leaf: 0,
            current_values: None,
            leaves,
            doc_id: -1,
            field,
            current_doc_base: 0,
        }
    }
}

impl<LR> DocValuesIterator for NumericDocValuesImpl1<LR>
where
    LR: LeafReader,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }

            let leaf = &self.leaves[reader_index];
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

impl<LR> DocIdSetIterator for NumericDocValuesImpl1<LR>
where
    LR: LeafReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            while self.current_values.is_none() {
                if self.next_leaf as usize == self.leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }
                let leaf = &self.leaves[self.next_leaf as usize];
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
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }
            let leaf = &self.leaves[reader_index];
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

impl<LR> NumericDocValues for NumericDocValuesImpl1<LR>
where
    LR: LeafReader,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.current_values {
            Some(ref mut values) => values.long_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }
}

pub struct BinaryDocValuesImpl<LR>
where
    LR: LeafReader,
{
    next_leaf: i32,
    current_values: Option<LRBinaryDocValues<LR>>,
    leaves: Vec<Arc<LeafReaderContext<LR>>>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
}

impl<LR> BinaryDocValuesImpl<LR>
where
    LR: LeafReader,
{
    pub fn new(leaves: Vec<Arc<LeafReaderContext<LR>>>, field: String) -> Self {
        Self {
            next_leaf: 0,
            current_values: None,
            leaves,
            doc_id: -1,
            field,
            current_doc_base: 0,
        }
    }
}
impl<LR> DocValuesIterator for BinaryDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }

            let leaf = &self.leaves[reader_index];
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
impl<LR> DocIdSetIterator for BinaryDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            while self.current_values.is_none() {
                if self.next_leaf as usize == self.leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                let leaf = &self.leaves[self.next_leaf as usize];
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
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                self.current_values = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            let leaf = &self.leaves[reader_index];
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
impl<LR> BinaryDocValues for BinaryDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn binary_value(&mut self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
        match self.current_values {
            Some(ref mut values) => values.binary_value(),
            None => Err(LuceneError::illegal_state("current_values is none")),
        }
    }
}
pub struct SortedNumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    next_leaf: i32,
    current_values_index: Option<usize>,
    values: Vec<
        Either2SortedNumericDocValues<
            SingletonSortedNumericDocValues<EmptyNumeric>,
            LRSortedNumericDocValues<LR>,
        >,
    >,
    leaves: Vec<Arc<LeafReaderContext<LR>>>,
    doc_id: i32,
    field: String,
    current_doc_base: i32,
    final_total_cost: i64,
}
impl<LR> SortedNumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    pub fn new(
        leaves: Vec<Arc<LeafReaderContext<LR>>>,
        values: Vec<
            Either2SortedNumericDocValues<
                SingletonSortedNumericDocValues<EmptyNumeric>,
                LRSortedNumericDocValues<LR>,
            >,
        >,
        field: String,
        total_cost: i64,
    ) -> Self {
        Self {
            next_leaf: 0,
            current_values_index: None,
            values,
            leaves,
            doc_id: -1,
            field,
            current_doc_base: 0,
            final_total_cost: total_cost,
        }
    }
}
impl<LR> DocValuesIterator for SortedNumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn advance_exact(&mut self, target_doc_id: i32) -> Result<bool> {
        if target_doc_id < self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "Out of range: {}",
                    target_doc_id
                )));
            }

            let leaf = &self.leaves[reader_index];
            self.current_doc_base = leaf.doc_base;
            self.current_values_index = Some(reader_index);
            self.next_leaf = (reader_index + 1) as i32;
        }

        self.doc_id = target_doc_id;
        let current_values = &mut self.values[*self.current_values_index.as_ref().unwrap()];
        current_values.advance_exact(target_doc_id - self.current_doc_base)
    }
}
impl<LR> DocIdSetIterator for SortedNumericDocValuesImpl<LR>
where
    LR: LeafReader,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        loop {
            if self.current_values_index.is_none() {
                if self.next_leaf as usize == self.leaves.len() {
                    self.doc_id = NO_MORE_DOCS;
                    return Ok(self.doc_id);
                }

                let leaf = &self.leaves[self.next_leaf as usize];
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
        if target_doc_id <= self.doc_id {
            return Err(LuceneError::illegal_argument(format!(
                "can only advance beyond current document: on docID={} but targetDocID={}",
                self.doc_id, target_doc_id
            )));
        }

        let reader_index = ReaderUtil::sub_index_with_leaves(target_doc_id, self.leaves.as_slice());

        if reader_index >= self.next_leaf as usize {
            if reader_index == self.leaves.len() {
                self.current_values_index = None;
                self.doc_id = NO_MORE_DOCS;
                return Ok(self.doc_id);
            }

            let leaf = &self.leaves[reader_index];
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
impl<LR> SortedNumericDocValues for SortedNumericDocValuesImpl<LR>
where
    LR: LeafReader,
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
