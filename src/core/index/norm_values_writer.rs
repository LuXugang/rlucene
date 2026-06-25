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
use crate::core::codecs::norms_consumer::NormsConsumer;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::docs_with_field_set::{DocsWithFieldSet, DocsWithFieldSetDISI};
use crate::core::index::field_info::FieldInfo;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::numeric_doc_values::NumericDocValuesEnum2;
use crate::core::index::numeric_doc_values_writer::{
  NumericDVs, SortingNumericDocValues, sort_doc_values,
};
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::sorter::DocMap;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::packed::PackedInts;
use crate::core::util::packed::packed_long_values::{
  Builder, PackedLongValues, PackedLongValuesIterator,
};
use crate::core::util::{Counter, SharedCounter};
use std::sync::Arc;

/// Buffers up pending long per doc, then flushes when segment flushes.
pub(crate) struct NormValuesWriter {
  docs_with_field: DocsWithFieldSet,
  pending: Builder,
  iw_bytes_used: SharedCounter,
  bytes_used: i64,
  field_info: Arc<FieldInfo>,
  last_doc_id: i32,
}
impl NormValuesWriter {
  pub(crate) fn new(field_info: Arc<FieldInfo>, iw_bytes_used: SharedCounter) -> Result<Self> {
    Ok(Self {
      docs_with_field: DocsWithFieldSet::new(),
      pending: PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?,
      iw_bytes_used,
      bytes_used: 0,
      field_info,
      last_doc_id: -1,
    })
  }
  pub(crate) fn add_value(&mut self, doc_id: i32, value: i64) -> Result<()> {
    if doc_id <= self.last_doc_id {
      return Err(LuceneError::illegal_argument(format!(
        "Norm for \"{}\" appears more than once in this document (only one value is allowed per field)",
        self.field_info.name
      )));
    }

    self.pending.add(value)?;
    self.docs_with_field.add(doc_id)?;
    self.update_bytes_used()?;
    self.last_doc_id = doc_id;
    Ok(())
  }

  fn update_bytes_used(&mut self) -> Result<()> {
    let new_bytes_used = self.pending.ram_bytes_used()? + self.docs_with_field.ram_bytes_used()?;
    self
      .iw_bytes_used
      .add_and_get(new_bytes_used - self.bytes_used);
    self.bytes_used = new_bytes_used;
    Ok(())
  }
  pub(crate) fn finish(&mut self, _max_doc: i32) {
    self.docs_with_field.finish()
  }

  pub(crate) fn flush<D, DM, N>(
    &mut self,
    sort_map: Option<&DM>,
    norms_consumer: &mut N,
    segment_info: &SegmentInfo<D>,
  ) -> Result<()>
  where
    D: Directory,
    DM: DocMap,
    N: NormsConsumer,
  {
    self.finish(segment_info.max_doc()?);
    let values = std::mem::take(&mut self.pending).build()?;
    let sorted = match sort_map {
      Some(sort_map) => {
        let dense = sort_map.size() == self.docs_with_field.cardinality();
        let iter = self.docs_with_field.iterator()?;
        let mut buffer_norms = BufferedNorms::new(&values, iter);
        let sorted = sort_doc_values(segment_info.max_doc()?, sort_map, &mut buffer_norms, dense)?;
        Some(sorted)
      },
      None => None,
    };

    let mut norms_producer =
      NormsProducerImpl::new(sorted, std::mem::take(&mut self.docs_with_field), values)?;
    norms_consumer.add_norms_field(&self.field_info, &mut norms_producer)?;

    Ok(())
  }
}

struct NormsProducerImpl {
  sorted: Option<NumericDVs<FixedBitSet>>,
  docs_with_field: DocsWithFieldSet,
  values: PackedLongValues,
}
impl NormsProducerImpl {
  pub(crate) fn new(
    sorted: Option<NumericDVs<FixedBitSet>>,
    docs_with_field: DocsWithFieldSet,
    values: PackedLongValues,
  ) -> Result<Self> {
    Ok(Self {
      sorted,
      docs_with_field,
      values,
    })
  }
}

impl NormsProducer for NormsProducerImpl {
  type NumericDocValues =
    NumericDocValuesEnum2<BufferedNorms, SortingNumericDocValues<FixedBitSet>>;

  fn get_norms(&self, _field_info2: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
    match &self.sorted {
      Some(sorted) => Ok(NumericDocValuesEnum2::B(SortingNumericDocValues::new(
        sorted.clone(),
      ))),
      None => Ok(NumericDocValuesEnum2::A(BufferedNorms::new(
        &self.values,
        self.docs_with_field.iterator()?,
      ))),
    }
  }

  fn check_integrity(&self) -> Result<()> {
    Ok(())
  }
}

/// iterates over the values we have in ram
struct BufferedNorms {
  iter: PackedLongValuesIterator,
  doc_with_field: DocsWithFieldSetDISI,
  value: i64,
}
impl BufferedNorms {
  pub(crate) fn new(values: &PackedLongValues, doc_with_field: DocsWithFieldSetDISI) -> Self {
    Self {
      iter: values.iterator(),
      doc_with_field,
      value: 0,
    }
  }
}

impl DocValuesIterator for BufferedNorms {
  fn advance_exact(&mut self, _target: i32) -> Result<bool> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl DocIdSetIterator for BufferedNorms {
  fn doc_id(&self) -> i32 {
    self.doc_with_field.doc_id()
  }

  fn next_doc(&mut self) -> Result<i32> {
    let doc_id = self.doc_with_field.next_doc()?;
    if doc_id != NO_MORE_DOCS {
      self.value = self.iter.next_value();
    }
    Ok(doc_id)
  }

  fn advance(&mut self, _target: i32) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn cost(&self) -> Result<i64> {
    self.doc_with_field.cost()
  }
}

impl NumericDocValues for BufferedNorms {
  fn long_value(&mut self) -> Result<i64> {
    Ok(self.value)
  }
}
