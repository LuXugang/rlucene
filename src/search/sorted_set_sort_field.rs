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
use crate::index::sort_field_provider::SortFieldProvider;
use crate::search::field_comparator_source::FieldComparatorSource;
use crate::search::sort_field::{SortField, SortFieldBase};
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::LuceneError;

pub struct SortedSetSortField;

pub struct SetProvider;
impl crate::search::sort_field::Provider {
    /// The name this Provider is registered under.
    pub const SET_NAME: &'static str = "SortedSetSortField";
}
impl SortFieldProvider for SetProvider {
    fn read_sort_field<D, F, S>(&self, data_input: &mut D) -> Result<SortField<F, S>, LuceneError>
    where
        D: DataInput,
        F: FieldComparatorSource,
        S: SortFieldBase,
    {
        todo!()
    }

    fn write_sort_field<D, F, S>(
        &self,
        sf: &SortField<F, S>,
        output: &mut D,
    ) -> Result<(), LuceneError>
    where
        D: DataOutput,
        F: FieldComparatorSource,
        S: SortFieldBase,
    {
        todo!()
    }
}
