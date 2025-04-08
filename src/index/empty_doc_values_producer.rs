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
use crate::codecs::doc_values_enum::doc_values::{
    BinaryDocValuesEnum, DocValuesSkipperEnum, NumericDocValuesEnum, SortedDocValuesEnum,
    SortedNumericDocValuesEnum, SortedSetDocValuesEnum,
};
use crate::codecs::doc_values_producer::{DocValuesProducer, DocValuesProducerEnum};
use crate::codecs::lucene90_doc_values_consumer::{
    EmptyDocValuesProducerSub3, EmptyDocValuesProducerSub4,
};
use crate::index::field_info::FieldInfo;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use std::cell::RefCell;
use std::rc::Rc;

pub struct EmptyDocValuesProducer<I>
where
    I: IndexInput,
{
    sub: EmptyDocValuesProducerSubEnum<I>,
    phantom: std::marker::PhantomData<I>,
}
impl<I> EmptyDocValuesProducer<I>
where
    I: IndexInput,
{
    pub fn new(sub: EmptyDocValuesProducerSubEnum<I>) -> Self {
        Self {
            sub,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<I> DocValuesProducer<I> for EmptyDocValuesProducer<I>
where
    I: IndexInput,
{
    fn get_numeric(&mut self, _field: &Rc<FieldInfo>) -> Result<NumericDocValuesEnum<I>> {
        self.sub.get_numeric(_field)
    }

    fn get_binary(&mut self, _field: &Rc<FieldInfo>) -> Result<BinaryDocValuesEnum<I>> {
        self.sub.get_binary(_field)
    }

    fn get_sorted(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> Result<Rc<RefCell<SortedDocValuesEnum<I>>>> {
        self.sub.get_sorted(_field)
    }

    fn get_sorted_numeric(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> Result<SortedNumericDocValuesEnum<I>> {
        self.sub.get_sorted_numeric(_field)
    }

    fn get_sorted_set(&mut self, _field: &Rc<FieldInfo>) -> Result<SortedSetDocValuesEnum<I>> {
        self.sub.get_sorted_set(_field)
    }

    fn get_skipper(&mut self, _field: &Rc<FieldInfo>) -> Result<DocValuesSkipperEnum<I>> {
        self.sub.get_skipper(_field)
    }

    fn check_integrity(&mut self) -> Result<()> {
        self.sub.check_integrity()
    }

    fn get_merge_instance(&mut self) -> Result<Option<DocValuesProducerEnum<I>>> {
        self.sub.get_merge_instance()
    }
}

pub enum EmptyDocValuesProducerSubEnum<I>
where
    I: IndexInput,
{
    Impl3(EmptyDocValuesProducerSub3<I>),
    Impl4(EmptyDocValuesProducerSub4<I>),
}
impl<I> DocValuesProducer<I> for EmptyDocValuesProducerSubEnum<I>
where
    I: IndexInput,
{
    fn get_numeric(&mut self, _field: &Rc<FieldInfo>) -> Result<NumericDocValuesEnum<I>> {
        todo!()
    }

    fn get_binary(&mut self, _field: &Rc<FieldInfo>) -> Result<BinaryDocValuesEnum<I>> {
        todo!()
    }

    fn get_sorted(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> Result<Rc<RefCell<SortedDocValuesEnum<I>>>> {
        todo!()
    }

    fn get_sorted_numeric(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> Result<SortedNumericDocValuesEnum<I>> {
        todo!()
    }

    fn get_sorted_set(&mut self, _field: &Rc<FieldInfo>) -> Result<SortedSetDocValuesEnum<I>> {
        todo!()
    }

    fn get_skipper(&mut self, _field: &Rc<FieldInfo>) -> Result<DocValuesSkipperEnum<I>> {
        todo!()
    }

    fn check_integrity(&mut self) -> Result<()> {
        todo!()
    }

    fn get_merge_instance(&mut self) -> Result<Option<DocValuesProducerEnum<I>>> {
        todo!()
    }
}
