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
use crate::index::index_sorter::IndexSorter;
use crate::search::sort_field::{Provider, SortFiledBase};
use crate::search::sort_field_enum::SortFieldEnum;
use crate::search::sorted_numeric_sort_field::NumericProvider;
use crate::search::sorted_set_sort_field::SetProvider;
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::{LuceneError, Result};

pub trait SortFieldProvider {
    fn read_sort_field(&self, data_input: &mut impl DataInput) -> Result<SortFieldEnum>;
    /// Writes a SortField to a DataOutput
    /// This is used to record index sort information in segment headers
    fn write_sort_field(&self, sf: &SortFieldEnum, output: &mut impl DataOutput) -> Result<()>;
}
pub fn write(sf: &SortFieldEnum, output: &mut impl DataOutput) -> Result<()> {
    if let Some(index_sort) = sf.get_index_sorter()? {
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
    fn read_sort_field(&self, data_input: &mut impl DataInput) -> Result<SortFieldEnum> {
        match self {
            SortFieldProviderEnum::SortedNumericProvider(provider) => {
                provider.read_sort_field(data_input)
            },
            SortFieldProviderEnum::SortedSetProvider(provider) => {
                provider.read_sort_field(data_input)
            },
            SortFieldProviderEnum::SortProvider(provider) => provider.read_sort_field(data_input),
        }
    }

    fn write_sort_field(&self, sf: &SortFieldEnum, output: &mut impl DataOutput) -> Result<()> {
        match self {
            SortFieldProviderEnum::SortedNumericProvider(provider) => {
                provider.write_sort_field(sf, output)
            },
            SortFieldProviderEnum::SortedSetProvider(provider) => {
                provider.write_sort_field(sf, output)
            },
            SortFieldProviderEnum::SortProvider(provider) => provider.write_sort_field(sf, output),
        }
    }
}
