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
use crate::index::index_sorter::{DocComparatorEnum, IndexSorter, StringSorter};
use crate::index::leaf_reader::LeafReader;
use crate::search::sort_field::{
    IndexSorterEnumSorter, MissingValueEnum, SortField, SortFiledBase,
};
use crate::search::sorted_numeric_sort_field::{IndexSorterNumeric, SortedNumericSortField};
use crate::search::sorted_set_sort_field::{SortedDocValuesProviderImpl, SortedSetSortField};
use crate::store::DataOutput;
use crate::util::error::lucene_error::Result;
use std::fmt;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Clone, PartialEq, Eq)]
pub enum SortFieldEnum {
    SortedNumeric(SortedNumericSortField),
    SortedSet(SortedSetSortField),
    Sorter(SortField),
}

impl SortFiledBase for SortFieldEnum {
    fn set_missing_value(&mut self, missing_value: Option<MissingValueEnum>) -> Result<()> {
        match self {
            SortFieldEnum::SortedNumeric(sort_field) => sort_field.set_missing_value(missing_value),
            SortFieldEnum::SortedSet(sort_field) => sort_field.set_missing_value(missing_value),
            SortFieldEnum::Sorter(sort_field) => sort_field.set_missing_value(missing_value),
        }
    }

    fn needs_scores(&self) -> bool {
        match self {
            SortFieldEnum::SortedNumeric(sort_field) => sort_field.needs_scores(),
            SortFieldEnum::SortedSet(sort_field) => sort_field.needs_scores(),
            SortFieldEnum::Sorter(sort_field) => sort_field.needs_scores(),
        }
    }

    type IndexSort = IndexSortEnum;

    fn get_index_sorter(&self) -> Result<Option<Self::IndexSort>> {
        match self {
            SortFieldEnum::SortedNumeric(sort_field) => {
                let sorter = sort_field.get_index_sorter()?;
                Ok(sorter.map(IndexSortEnum::SortedNumeric))
            },
            SortFieldEnum::SortedSet(sort_field) => {
                let sorter = sort_field.get_index_sorter()?;
                Ok(sorter.map(IndexSortEnum::SortedSet))
            },
            SortFieldEnum::Sorter(sort_field) => {
                let sorter = sort_field.get_index_sorter()?;
                Ok(sorter.map(IndexSortEnum::Sorter))
            },
        }
    }

    fn serialize(&self, out: &mut impl DataOutput) -> Result<()> {
        match self {
            SortFieldEnum::SortedNumeric(sort_field) => sort_field.serialize(out),
            SortFieldEnum::SortedSet(sort_field) => sort_field.serialize(out),
            SortFieldEnum::Sorter(sort_field) => sort_field.serialize(out),
        }
    }
}

impl Display for SortFieldEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortFieldEnum::SortedNumeric(sort_field) => {
                write!(f, "{}", sort_field)
            },
            SortFieldEnum::SortedSet(sort_field) => write!(f, "{}", sort_field),
            SortFieldEnum::Sorter(sort_field) => write!(f, "{}", sort_field),
        }
    }
}

impl Hash for SortFieldEnum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            SortFieldEnum::SortedNumeric(sort_field) => sort_field.hash(state),
            SortFieldEnum::SortedSet(sort_field) => sort_field.hash(state),
            SortFieldEnum::Sorter(sort_field) => sort_field.hash(state),
        }
    }
}

impl From<SortField> for SortFieldEnum {
    fn from(sort_field: SortField) -> Self {
        SortFieldEnum::Sorter(sort_field)
    }
}
impl From<SortedNumericSortField> for SortFieldEnum {
    fn from(sort_field: SortedNumericSortField) -> Self {
        SortFieldEnum::SortedNumeric(sort_field)
    }
}
impl From<SortedSetSortField> for SortFieldEnum {
    fn from(sort_field: SortedSetSortField) -> Self {
        SortFieldEnum::SortedSet(sort_field)
    }
}

pub trait SortFieldVecExt {
    fn push_sort_fields(&mut self, item: impl Into<SortFieldEnum>);
}

impl SortFieldVecExt for Vec<SortFieldEnum> {
    fn push_sort_fields(&mut self, item: impl Into<SortFieldEnum>) {
        self.push(item.into());
    }
}

pub enum IndexSortEnum {
    SortedNumeric(IndexSorterNumeric),
    SortedSet(StringSorter<SortedDocValuesProviderImpl>),
    Sorter(IndexSorterEnumSorter),
}
impl IndexSorter for IndexSortEnum {
    fn get_provider_name(&self) -> &str {
        match self {
            IndexSortEnum::SortedNumeric(sorter) => sorter.get_provider_name(),
            IndexSortEnum::SortedSet(sorter) => sorter.get_provider_name(),
            IndexSortEnum::Sorter(sorter) => sorter.get_provider_name(),
        }
    }

    type DocComparator = DocComparatorEnum;

    fn get_doc_comparator<LR>(
        &mut self,
        leaf_reader: &mut LR,
        max_doc: i32,
    ) -> Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        match self {
            IndexSortEnum::SortedNumeric(sorter) => sorter.get_doc_comparator(leaf_reader, max_doc),
            IndexSortEnum::SortedSet(sorter) => Ok(DocComparatorEnum::String(
                sorter.get_doc_comparator(leaf_reader, max_doc)?,
            )),
            IndexSortEnum::Sorter(sorter) => sorter.get_doc_comparator(leaf_reader, max_doc),
        }
    }
}
