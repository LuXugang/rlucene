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
use crate::codecs::doc_values_enum::doc_values::SortedDocValuesEnum;
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::terms_enums::TermsEnums;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::doc_id_set_iterator_static::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};
/// Exposes a multi-valued iterator view over a single-valued iterator.
///
/// This can be used if you want to have one multi-valued implementation that works for both
/// single-valued and multi-valued types.
pub struct SingletonSortedSetDocValues<I>
where
    I: IndexInput,
{
    inner: SortedDocValuesEnum<I>,
    ord: i64,
}

impl<I> SingletonSortedSetDocValues<I>
where
    I: IndexInput,
{
    /// Creates a multi-valued view over the provided SortedDocValues.
    pub fn new(inner: SortedDocValuesEnum<I>) -> Result<Self> {
        if inner.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                inner.doc_id()
            )));
        }
        Ok(Self { inner, ord: -1 })
    }

    pub fn get_numeric_doc_values(&self) -> Result<&SortedDocValuesEnum<I>> {
        if self.inner.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                self.inner.doc_id()
            )));
        }
        Ok(&self.inner)
    }
}

impl<I> DocIdSetIterator for SingletonSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.inner.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.inner.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            self.ord = self.inner.ord_value()? as i64;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc_id = self.inner.advance(target)?;
        if doc_id != NO_MORE_DOCS {
            self.ord = self.inner.ord_value()? as i64;
        }
        Ok(doc_id)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.cost()
    }
}

impl<I> DocValuesIterator for SingletonSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.advance_exact(target)? {
            self.ord = self.inner.ord_value()? as i64;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<I> SortedSetDocValues<I> for SingletonSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn next_ord(&mut self) -> Result<i64> {
        Ok(self.ord)
    }

    fn doc_value_count(&mut self) -> Result<i64> {
        Ok(1)
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<BytesRef> {
        self.inner.lookup_ord(ord as i32)
    }

    fn get_value_count(&self) -> Result<i64> {
        Ok(self.inner.get_value_count()? as i64)
    }

    fn lookup_term(&mut self, key: &BytesRef) -> Result<i64> {
        Ok(self.inner.lookup_term(key)? as i64)
    }

    fn terms_enum(&mut self) -> Result<TermsEnums<I>> {
        self.inner.terms_enum()
    }
}
