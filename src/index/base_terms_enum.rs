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
use crate::index::postings_enum::PostingsEnum;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::io_boolean_supplier::{IOBooleanSupplier, IOBooleanSupplierEnum};
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
/// A base `TermsEnum` that provides default implementations for:
///
/// - [`attributes()`](BaseTermsEnum::attributes)
/// - [`term_state()`](BaseTermsEnum::term_state)
/// - [`seek_exact(&BytesRef)`](BaseTermsEnum::seek_exact)
/// - [`seek_exact_with_state(&BytesRef, &TermState)`](BaseTermsEnum::seek_exact_with_state)
///
/// In some cases, the default implementation may be slow and consume large amounts of memory,
/// so subclasses SHOULD provide their own implementation if possible.
pub struct BaseTermsEnum<T>
where
    T: TermsEnum,
{
    atts: AttributeSource,
    sub_terms_enum: Rc<RefCell<T>>,
}
impl<T> BaseTermsEnum<T>
where
    T: TermsEnum,
{
    pub fn new(sub_terms_enum: Rc<RefCell<T>>) -> Self {
        Self {
            atts: AttributeSource::new(),
            sub_terms_enum,
        }
    }
}

impl<T> BytesRefIterator for BaseTermsEnum<T>
where
    T: TermsEnum,
{
    fn next(&mut self) -> Result<Option<BytesRef>> {
        self.sub_terms_enum.borrow_mut().next()
    }
}

impl<T> TermsEnum for BaseTermsEnum<T>
where
    T: TermsEnum<IOBooleanSupplierType = IOBooleanSupplierEnum<T>, TermStateType = TermStateEnum>,
{
    fn attributes(&self) -> &AttributeSource {
        // TODO: 参考BaseTermsEnum中prepare_seek_exact方法 来选择使用父或子的实现
        &self.atts
    }

    fn prepare_seek_exact(
        &mut self,
        term: Rc<BytesRef>,
    ) -> Result<Option<Self::IOBooleanSupplierType>> {
        let sub = self
            .sub_terms_enum
            .borrow_mut()
            .prepare_seek_exact(term.clone());
        match sub {
            Ok(s) => Ok(s),
            Err(e) => match e {
                // sub_terms_enum's invalid error,
                // it means sub_terms_enum uses the return value of BaseTermsEnum's prepare_seek_exact
                LuceneError::NotImplemented(_) => {
                    let supplier = IOBooleanSupplierImpl {
                        text: term,
                        sub_terms_enum: self.sub_terms_enum.clone(),
                    };
                    if self.seek_exact(&supplier.text)? {
                        Ok(Some(IOBooleanSupplierEnum::Impl1(supplier)))
                    } else {
                        Ok(None)
                    }
                }
                // sub_terms_enum's valid error
                _ => Err(e),
            },
        }
    }

    type IOBooleanSupplierType = IOBooleanSupplierEnum<T>;

    fn seek_ceil(&mut self, term: &BytesRef) -> Result<SeekStatus> {
        self.sub_terms_enum.borrow_mut().seek_ceil(term)
    }

    fn seek_exact_by_ord(&mut self, ord: i64) -> Result<()> {
        self.sub_terms_enum.borrow_mut().seek_exact_by_ord(ord)
    }

    fn seek_exact_with_state(&mut self, term: &BytesRef, _state: &impl TermState) -> Result<()> {
        if self.seek_exact(term)? {
            return Err(LuceneError::illegal_argument(format!(
                "term= {} does not exist",
                term
            )));
        };
        Ok(())
    }

    fn term(&self) -> Result<BytesRef> {
        self.sub_terms_enum.borrow_mut().term()
    }

    fn ord(&self) -> Result<i64> {
        self.sub_terms_enum.borrow_mut().ord()
    }

    fn doc_freq(&self) -> Result<i32> {
        self.sub_terms_enum.borrow_mut().doc_freq()
    }

    fn total_term_freq(&self) -> Result<i64> {
        self.sub_terms_enum.borrow_mut().total_term_freq()
    }

    fn postings_with_flags(
        &mut self,
        reuse: &Option<impl PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnumType> {
        self.sub_terms_enum
            .borrow_mut()
            .postings_with_flags(reuse, flags)
    }

    type PostingsEnumType = T::PostingsEnumType;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnumType> {
        self.sub_terms_enum.borrow_mut().impacts(flags)
    }

    type ImpactsEnumType = T::ImpactsEnumType;

    fn term_state(&self) -> Result<Self::TermStateType> {
        let sub = self.sub_terms_enum.borrow_mut().term_state();
        match sub {
            Ok(s) => Ok(s),
            Err(e) => match e {
                // sub_terms_enum's invalid error,
                // it means sub_terms_enum uses the return value of BaseTermsEnum's term_state
                LuceneError::NotImplemented(_) => Ok(TermStateEnum::Impl1(TermStateImpl1)),
                // sub_terms_enum's valid error
                _ => Err(e),
            },
        }
    }

    type TermStateType = TermStateEnum;
}
#[derive(Debug, Clone)]
pub struct TermStateImpl1;
impl Display for TermStateImpl1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", TermState::to_string(self))
    }
}
impl TermState for TermStateImpl1 {
    fn copy_from(&mut self, _other: &impl TermState) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }
}
pub struct IOBooleanSupplierImpl<T>
where
    T: TermsEnum,
{
    pub(crate) text: Rc<BytesRef>,
    sub_terms_enum: Rc<RefCell<T>>,
}
impl<T> IOBooleanSupplier for IOBooleanSupplierImpl<T>
where
    T: TermsEnum,
{
    fn get(&mut self) -> Result<bool> {
        self.sub_terms_enum.borrow_mut().seek_exact(&self.text)
    }
}
