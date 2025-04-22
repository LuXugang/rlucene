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
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::access::AccessVec;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

/// Exposes a multi-valued iterator view over a single-valued iterator.
///
/// This can be used if you want to have one multi-valued implementation that works for both
/// single-valued and multi-valued types.
pub struct SingletonSortedSetDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    inner: Rc<RefCell<SortedDocValuesEnum<I, AV>>>,
    ord: i64,
}

impl<I, AV> SingletonSortedSetDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    /// Creates a multi-valued view over the provided SortedDocValues.
    pub fn new(inner: Rc<RefCell<SortedDocValuesEnum<I, AV>>>) -> Result<Self> {
        if inner.borrow().doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                inner.borrow().doc_id()
            )));
        }
        Ok(Self { inner, ord: -1 })
    }

    pub fn get_numeric_doc_values(&self) -> Result<Rc<RefCell<SortedDocValuesEnum<I, AV>>>> {
        if self.inner.borrow().doc_id() != -1 {
            return Err(LuceneError::illegal_state(format!(
                "iterator has already been used: docID={}",
                self.inner.borrow().doc_id()
            )));
        }
        Ok(self.inner.clone())
    }
}

impl<I, AV> DocIdSetIterator for SingletonSortedSetDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn doc_id(&self) -> i32 {
        self.inner.borrow().doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.inner.borrow_mut().next_doc()?;
        if doc_id != NO_MORE_DOCS {
            self.ord = self.inner.borrow_mut().ord_value()? as i64;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        let doc_id = self.inner.borrow_mut().advance(target)?;
        if doc_id != NO_MORE_DOCS {
            self.ord = self.inner.borrow_mut().ord_value()? as i64;
        }
        Ok(doc_id)
    }

    fn cost(&self) -> Result<i64> {
        self.inner.borrow().cost()
    }
}

impl<I, AV> DocValuesIterator for SingletonSortedSetDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        if self.inner.borrow_mut().advance_exact(target)? {
            self.ord = self.inner.borrow_mut().ord_value()? as i64;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<I, AV> SortedSetDocValues<I, AV> for SingletonSortedSetDocValues<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn next_ord(&mut self) -> Result<i64> {
        Ok(self.ord)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(1)
    }

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<BytesRef<AV>>> {
        todo!()
        // self.inner.borrow_mut().lookup_ord(ord as i32)
    }

    fn get_value_count(&self) -> Result<i64> {
        Ok(self.inner.borrow_mut().get_value_count()? as i64)
    }

    fn lookup_term(&mut self, key: &BytesRef<AV>) -> Result<i64> {
        Ok(self.inner.borrow_mut().lookup_term(key)? as i64)
    }

    fn terms_enum(&mut self) -> Result<TermsEnums<I, AV>> {
        self.inner.borrow_mut().terms_enum()
    }

    fn unwrap_singleton(&self) -> Result<Option<Rc<RefCell<SortedDocValuesEnum<I, AV>>>>> {
        Ok(Some(self.get_numeric_doc_values()?))
    }
}
