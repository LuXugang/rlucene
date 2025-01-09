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
use crate::search::sort_field::SortField;
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::LuceneError;

pub struct SortedNumericSortField;

pub struct NumericProvider;
impl crate::search::sort_field::Provider {
    /// The name this Provider is registered under.
    pub const NUMERIC_NAME: &'static str = "SortedNumericSortField";
}
impl SortFieldProvider for NumericProvider {
    fn read_sort_field<D: DataInput, F: FieldComparatorSource>(
        &self,
        data_input: &mut D,
    ) -> Result<SortField<F>, LuceneError> {
        todo!()
    }

    fn write_sort_field<D: DataOutput, F: FieldComparatorSource>(
        &self,
        sf: &SortField<F>,
        output: &mut D,
    ) -> Result<(), LuceneError> {
        todo!()
    }
}
