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
use crate::index::field_info::FieldInfo;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use std::marker::PhantomData;
pub struct EmptyDocValuesProducer<T, I>
where
    I: IndexInput,
    T: DocValuesProducer<I>,
{
    sub: T,
    phantom: PhantomData<I>,
}
impl<T, I> DocValuesProducer<I> for EmptyDocValuesProducer<T, I>
where
    I: IndexInput,
    T: DocValuesProducer<I>,
{
    fn get_numeric(&mut self, field: &FieldInfo) -> Result<NumericDocValuesEnum<I>> {
        self.sub.get_numeric(field)
    }

    fn get_binary(&mut self, field: &FieldInfo) -> Result<BinaryDocValuesEnum<I>> {
        self.sub.get_binary(field)
    }

    fn get_sorted(&mut self, field: &FieldInfo) -> Result<SortedDocValuesEnum<I>> {
        self.sub.get_sorted(field)
    }

    fn get_sorted_numeric(&mut self, field: &FieldInfo) -> Result<SortedNumericDocValuesEnum<I>> {
        self.sub.get_sorted_numeric(field)
    }

    fn get_sorted_set(&mut self, field: &FieldInfo) -> Result<SortedSetDocValuesEnum<I>> {
        self.sub.get_sorted_set(field)
    }

    fn get_skipper(&mut self, field: &FieldInfo) -> Result<DocValuesSkipperEnum<I>> {
        self.sub.get_skipper(field)
    }

    fn check_integrity(&mut self) -> Result<()> {
        self.sub.check_integrity()
    }

    fn get_merge_instance(&mut self) -> Result<Option<DocValuesProducerEnum<I>>> {
        self.sub.get_merge_instance()
    }
}
