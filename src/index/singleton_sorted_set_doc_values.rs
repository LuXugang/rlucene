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
use std::borrow::Cow;

use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

/// Exposes a multi-valued iterator view over a single-valued iterator.
///
/// This can be used if you want to have one multi-valued implementation that
/// works for both single-valued and multi-valued types.
pub struct SingletonSortedSetDocValues<S>
where
    S: SortedDocValues,
{
    pub(crate) inner: Option<S>,
    ord: i64,
}

impl<S> SingletonSortedSetDocValues<S>
where
    S: SortedDocValues,
{
    /// Creates a multi-valued view over the provided SortedDocValues.
    pub fn new(inner: S) -> Result<Self> {
        if inner.doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                inner.doc_id()
            )));
        }
        Ok(Self {
            inner: Some(inner),
            ord: -1,
        })
    }

    pub fn get_sorted_doc_values(&mut self) -> Result<S> {
        if self.inner.as_ref().unwrap().doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                self.inner.as_ref().unwrap().doc_id()
            )));
        }
        Ok(self.inner.take().unwrap())
    }
}

impl<S> DocIdSetIterator for SingletonSortedSetDocValues<S>
where
    S: SortedDocValues,
{
    fn doc_id(&self) -> i32 {
        self.inner.as_ref().unwrap().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.inner.as_mut().unwrap().next_doc()?;
        if doc_id != NO_MORE_DOCS {
            self.ord = self.inner.as_mut().unwrap().ord_value()? as i64;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc_id = self.inner.as_mut().unwrap().advance(target)?;
        if doc_id != NO_MORE_DOCS {
            self.ord = self.inner.as_mut().unwrap().ord_value()? as i64;
        }
        Ok(doc_id)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.as_ref().unwrap().cost()
    }
}

impl<S> DocValuesIterator for SingletonSortedSetDocValues<S>
where
    S: SortedDocValues,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.as_mut().unwrap().advance_exact(target)? {
            self.ord = self.inner.as_mut().unwrap().ord_value()? as i64;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<S> SortedSetDocValues for SingletonSortedSetDocValues<S>
where
    S: SortedDocValues,
{
    fn next_ord(&mut self) -> Result<i64> {
        Ok(self.ord)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(1)
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.inner.as_mut().unwrap().lookup_ord(ord as i32)
    }

    fn get_value_count(&mut self) -> Result<i64> {
        Ok(self.inner.as_mut().unwrap().get_value_count()? as i64)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i64> {
        Ok(self.inner.as_mut().unwrap().lookup_term(key)? as i64)
    }

    type TermsEnum = DummyTermsEnum;

    fn is_single_valued(&self) -> bool {
        true
    }

    type SortedDocValues = S;

    fn get_sorted_doc_values(&mut self) -> Result<Option<Self::SortedDocValues>> {
        Ok(Some(self.get_sorted_doc_values()?))
    }
}
