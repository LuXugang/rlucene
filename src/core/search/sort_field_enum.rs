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
use crate::core::index::doc_values::SortedSet;
use crate::core::index::index_sorter::{
  CPEnumType1, CPEnumType2, ComparableProviderEnum3, DocComparatorImpl, IndexSorter,
  StringComparableProvider, StringSorter,
};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::ordinal_map::OrdinalMap;
use crate::core::search::field_comparator::FieldComparatorEnum;
use crate::core::search::pruning::Pruning;
use crate::core::search::sort_field::{
  IndexSorterEnumSorter, MissingValueEnum, NPImpl1, SProviderImpl2, SortField, SortFieldType,
  SortFiledBase,
};
use crate::core::search::sorted_numeric_sort_field::{
  IndexSorterNumeric, NPImpl2, SortedNumericSortField,
};
use crate::core::search::sorted_set_selector::SortedDocValuesWrap;
use crate::core::search::sorted_set_sort_field::{SProviderImpl1, SortedSetSortField};
use crate::core::store::DataOutput;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;
use std::fmt;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Clone, PartialEq, Eq)]
pub enum SortFieldEnum {
  SortedNumeric(SortedNumericSortField),
  SortedSet(SortedSetSortField),
  Sorter(SortField),
}

macro_rules! dispatch_sort_field {
  ($self:expr, |$sort_field:ident| $body:expr) => {{
    match $self {
      SortFieldEnum::SortedNumeric($sort_field) => $body,
      SortFieldEnum::SortedSet($sort_field) => $body,
      SortFieldEnum::Sorter($sort_field) => $body,
    }
  }};
}

macro_rules! dispatch_sort_field_base {
  ($self:expr, |$sort_field:ident| $body:expr) => {{
    match $self {
      SortFieldEnum::SortedNumeric(sort_field) => {
        let $sort_field = &sort_field.base;
        $body
      },
      SortFieldEnum::SortedSet(sort_field) => {
        let $sort_field = &sort_field.base;
        $body
      },
      SortFieldEnum::Sorter($sort_field) => $body,
    }
  }};
}

macro_rules! dispatch_sort_field_base_mut {
  ($self:expr, |$sort_field:ident| $body:expr) => {{
    match $self {
      SortFieldEnum::SortedNumeric(sort_field) => {
        let $sort_field = &mut sort_field.base;
        $body
      },
      SortFieldEnum::SortedSet(sort_field) => {
        let $sort_field = &mut sort_field.base;
        $body
      },
      SortFieldEnum::Sorter($sort_field) => $body,
    }
  }};
}

impl SortFieldEnum {
  pub fn get_reverse(&self) -> bool {
    dispatch_sort_field_base!(self, |sort_field| sort_field.get_reverse())
  }
  pub fn get_field(&self) -> Option<&str> {
    dispatch_sort_field_base!(self, |sort_field| sort_field.get_field())
  }
  pub fn get_type(&self) -> SortFieldType {
    dispatch_sort_field_base!(self, |sort_field| sort_field.get_type())
  }
  pub fn get_missing_value(&self) -> Option<&MissingValueEnum> {
    dispatch_sort_field_base!(self, |sort_field| sort_field.get_missing_value())
  }
  pub fn set_optimize_sort_with_indexed_data(&mut self, optimize_sort_with_indexed_data: bool) {
    dispatch_sort_field_base_mut!(self, |sort_field| {
      sort_field.set_optimize_sort_with_indexed_data(optimize_sort_with_indexed_data)
    })
  }
}

impl SortFiledBase for SortFieldEnum {
  fn set_missing_value<T>(&mut self, missing_value: T) -> Result<()>
  where
    T: Into<MissingValueEnum>,
  {
    let missing_value = missing_value.into();
    dispatch_sort_field!(self, |sort_field| sort_field
      .set_missing_value(missing_value))
  }

  fn needs_scores(&self) -> bool {
    dispatch_sort_field!(self, |sort_field| sort_field.needs_scores())
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
    dispatch_sort_field!(self, |sort_field| sort_field.serialize(out))
  }

  type FieldComparator = FieldComparatorEnum;

  fn get_comparator(&self, num_hits: usize, pruning: Pruning) -> Result<Self::FieldComparator> {
    dispatch_sort_field!(self, |sort_field| sort_field
      .get_comparator(num_hits, pruning))
  }
}

impl Display for SortFieldEnum {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    dispatch_sort_field!(self, |sort_field| write!(f, "{sort_field}"))
  }
}

impl Hash for SortFieldEnum {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    dispatch_sort_field!(self, |sort_field| sort_field.hash(state))
  }
}

impl_from_for_enum!(
    SortFieldEnum,
    SortField => Sorter,
    SortedNumericSortField => SortedNumeric,
    SortedSetSortField => SortedSet,
);
pub type CPType<LR> = ComparableProviderEnum3<
  CPEnumType2<NPImpl2, LR>,
  StringComparableProvider<SortedDocValuesWrap<SortedSet<LR>>>,
  CPEnumType1<NPImpl1, LR, SProviderImpl2>,
>;
pub enum IndexSortEnum {
  SortedNumeric(IndexSorterNumeric),
  SortedSet(StringSorter<SProviderImpl1>),
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

  type ComparableProvider<LR>
    = CPType<LR>
  where
    LR: LeafReader;

  fn get_comparable_providers<LR>(
    &self,
    readers: &[LR],
  ) -> Result<Vec<Self::ComparableProvider<LR>>>
  where
    LR: LeafReader,
  {
    let missing_value = self.get_missing_value();
    let ordinal_map = self.get_ordinal_map(readers)?;
    let mut provider = Vec::with_capacity(readers.len());
    match self {
      IndexSortEnum::SortedNumeric(sorter) => {
        for (idx, reader) in readers.iter().enumerate() {
          let v = sorter.get_comparable_providers_per_reader(
            reader,
            idx,
            &missing_value,
            ordinal_map.as_ref(),
          )?;
          provider.push(ComparableProviderEnum3::SortedNumeric(v))
        }
        Ok(provider)
      },
      IndexSortEnum::SortedSet(sorter) => {
        for (idx, reader) in readers.iter().enumerate() {
          let v = sorter.get_comparable_providers_per_reader(
            reader,
            idx,
            &missing_value,
            ordinal_map.as_ref(),
          )?;
          provider.push(ComparableProviderEnum3::SortedSet(v))
        }
        Ok(provider)
      },
      IndexSortEnum::Sorter(sorter) => {
        for (idx, reader) in readers.iter().enumerate() {
          let v = sorter.get_comparable_providers_per_reader(
            reader,
            idx,
            &missing_value,
            ordinal_map.as_ref(),
          )?;
          provider.push(ComparableProviderEnum3::Sorter(v))
        }
        Ok(provider)
      },
    }
  }

  fn get_ordinal_map<LR>(&self, readers: &[LR]) -> Result<Option<OrdinalMap>>
  where
    LR: LeafReader,
  {
    match self {
      IndexSortEnum::SortedNumeric(sorter) => sorter.get_ordinal_map(readers),
      IndexSortEnum::SortedSet(sorter) => sorter.get_ordinal_map(readers),
      IndexSortEnum::Sorter(sorter) => sorter.get_ordinal_map(readers),
    }
  }

  fn get_missing_value(&self) -> MissingValueEnum {
    match self {
      IndexSortEnum::SortedNumeric(sorter) => sorter.get_missing_value(),
      IndexSortEnum::SortedSet(sorter) => sorter.get_missing_value(),
      IndexSortEnum::Sorter(sorter) => sorter.get_missing_value(),
    }
  }

  type DocComparator = DocComparatorImpl;

  fn get_doc_comparator<LR>(&self, leaf_reader: &LR, max_doc: i32) -> Result<Self::DocComparator>
  where
    LR: LeafReader,
  {
    match self {
      IndexSortEnum::SortedNumeric(sorter) => sorter.get_doc_comparator(leaf_reader, max_doc),
      IndexSortEnum::SortedSet(sorter) => Ok(DocComparatorImpl::String(
        sorter.get_doc_comparator(leaf_reader, max_doc)?,
      )),
      IndexSortEnum::Sorter(sorter) => sorter.get_doc_comparator(leaf_reader, max_doc),
    }
  }
}
