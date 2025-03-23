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
use crate::index::index_sorter::IndexSorter;
use crate::search::sort_field::{Provider, SortFiledBase};
use crate::search::sort_field_enum::SortFieldEnum;
use crate::search::sorted_numeric_sort_field::NumericProvider;
use crate::search::sorted_set_sort_field::SetProvider;
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::{LuceneError, Result};

pub trait SortFieldProvider {
    fn read_sort_field<D>(&self, data_input: &mut D) -> Result<SortFieldEnum>
    where
        D: DataInput;
    /// Writes a SortField to a DataOutput
    /// This is used to record index sort information in segment headers
    fn write_sort_field<D>(&self, sf: &SortFieldEnum, output: &mut D) -> Result<()>
    where
        D: DataOutput;
}
pub fn write<D>(sf: &SortFieldEnum, output: &mut D) -> Result<()>
where
    D: DataOutput,
{
    if let Some(index_sort) = sf.get_index_sorter() {
        let provider = for_name(index_sort.get_provider_name());
        provider.write_sort_field(sf, output)?;
    } else {
        return Err(LuceneError::illegal_argument(format!(
            "Cannot serialize sort field {}",
            sf
        )));
    }
    Ok(())
}
pub fn for_name(name: &str) -> SortFieldProviderEnum {
    match name {
        NumericProvider::NAME => SortFieldProviderEnum::SortedNumericProvider(NumericProvider),
        SetProvider::NAME => SortFieldProviderEnum::SortedSetProvider(SetProvider),
        _ => SortFieldProviderEnum::SortProvider(Provider),
    }
}
pub enum SortFieldProviderEnum {
    SortedNumericProvider(NumericProvider),
    SortedSetProvider(SetProvider),
    SortProvider(Provider),
}
impl SortFieldProvider for SortFieldProviderEnum {
    fn read_sort_field<D>(&self, data_input: &mut D) -> Result<SortFieldEnum>
    where
        D: DataInput,
    {
        match self {
            SortFieldProviderEnum::SortedNumericProvider(provider) => {
                provider.read_sort_field(data_input)
            }
            SortFieldProviderEnum::SortedSetProvider(provider) => {
                provider.read_sort_field(data_input)
            }
            SortFieldProviderEnum::SortProvider(provider) => provider.read_sort_field(data_input),
        }
    }

    fn write_sort_field<D>(&self, sf: &SortFieldEnum, output: &mut D) -> Result<()>
    where
        D: DataOutput,
    {
        match self {
            SortFieldProviderEnum::SortedNumericProvider(provider) => {
                provider.write_sort_field(sf, output)
            }
            SortFieldProviderEnum::SortedSetProvider(provider) => {
                provider.write_sort_field(sf, output)
            }
            SortFieldProviderEnum::SortProvider(provider) => provider.write_sort_field(sf, output),
        }
    }
}
