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
use crate::core::codecs::doc_values_consumer::DocValuesConsumer;
use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::codecs::doc_values_producer::DocValuesProducer;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_reader::Identity;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::sync::Arc;

pub struct CrankyDocValuesFormat<DVF> {
  delegate: DVF,
  random: Arc<Mutex<StdRng>>,
}

impl<DVF> CrankyDocValuesFormat<DVF> {
  pub fn new(delegate: DVF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<DVF> Display for CrankyDocValuesFormat<DVF>
where
  DVF: DocValuesFormat,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.delegate, f)
  }
}

impl<DVF> HasIdentity for CrankyDocValuesFormat<DVF>
where
  DVF: DocValuesFormat,
{
  fn identity(&self) -> &Identity {
    self.delegate.identity()
  }
}

impl<DVF> DocValuesFormat for CrankyDocValuesFormat<DVF>
where
  DVF: DocValuesFormat,
{
  fn get_name(&self) -> &str {
    self.delegate.get_name()
  }

  type DocValuesConsumer<O: IndexOutput> = CrankyDocValuesConsumer<DVF::DocValuesConsumer<O>>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesConsumer<D1::IndexOutput>>
  where
    D1: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesFormat.fieldsConsumer()",
      )));
    }
    Ok(CrankyDocValuesConsumer::new(
      self.delegate.fields_consumer(state, segment_info)?,
      Arc::clone(&self.random),
    ))
  }

  type DocValuesProducer<I: IndexInput> = DVF::DocValuesProducer<I>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesProducer<D1::IndexInput>>
  where
    D1: Directory,
  {
    self.delegate.fields_producer(state, segment_info)
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Err(LuceneError::illegal_argument(format!(
      "Could not load doc values format named \"{name}\""
    )))
  }
}

pub struct CrankyDocValuesConsumer<DVC> {
  delegate: DVC,
  random: Arc<Mutex<StdRng>>,
}

impl<DVC> CrankyDocValuesConsumer<DVC> {
  fn new(delegate: DVC, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<DVC> Closeable for CrankyDocValuesConsumer<DVC>
where
  DVC: DocValuesConsumer,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()?;
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesConsumer.close()",
      )));
    }
    Ok(())
  }
}

impl<DVC> DocValuesConsumer for CrankyDocValuesConsumer<DVC>
where
  DVC: DocValuesConsumer,
{
  type IndexOutput = DVC::IndexOutput;

  fn add_numeric_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesConsumer.addNumericField()",
      )));
    }
    self
      .delegate
      .add_numeric_field(write_state, segment_info, field, values_producer)
  }

  fn add_binary_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesConsumer.addBinaryField()",
      )));
    }
    self
      .delegate
      .add_binary_field(write_state, segment_info, field, values_producer)
  }

  fn add_sorted_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesConsumer.addSortedField()",
      )));
    }
    self
      .delegate
      .add_sorted_field(write_state, segment_info, field, values_producer)
  }

  fn add_sorted_numeric_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesConsumer.addSortedNumericField()",
      )));
    }
    self
      .delegate
      .add_sorted_numeric_field(write_state, segment_info, field, values_producer)
  }

  fn add_sorted_set_field<D1, D2, D>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    field: &Arc<FieldInfo>,
    values_producer: &D,
  ) -> Result<()>
  where
    D1: Directory<IndexOutput = Self::IndexOutput>,
    D: DocValuesProducer,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from DocValuesConsumer.addSortedSetField()",
      )));
    }
    self
      .delegate
      .add_sorted_set_field(write_state, segment_info, field, values_producer)
  }
}
