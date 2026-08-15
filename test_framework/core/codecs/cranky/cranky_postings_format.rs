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
use crate::core::codecs::fields_consumer::FieldsConsumer;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::index::fields::Fields;
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
use std::io::Error;
use std::sync::Arc;

pub struct CrankyPostingsFormat<PF> {
  delegate: PF,
  random: Arc<Mutex<StdRng>>,
}

impl<PF> CrankyPostingsFormat<PF> {
  pub fn new(delegate: PF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<PF> HasIdentity for CrankyPostingsFormat<PF>
where
  PF: PostingsFormat,
{
  fn identity(&self) -> &Identity {
    self.delegate.identity()
  }
}

impl<PF> PostingsFormat for CrankyPostingsFormat<PF>
where
  PF: PostingsFormat,
{
  fn get_name(&self) -> &str {
    self.delegate.get_name()
  }

  type FieldsConsumer<O: IndexOutput> = CrankyFieldsConsumer<PF::FieldsConsumer<O>>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from PostingsFormat.fieldsConsumer()",
      )));
    }
    Ok(CrankyFieldsConsumer::new(
      self.delegate.fields_consumer(state, segment_info)?,
      Arc::clone(&self.random),
    ))
  }

  type FieldsProducer<I: IndexInput> = PF::FieldsProducer<I>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsProducer<D1::IndexInput>>
  where
    D1: Directory,
  {
    self.delegate.fields_producer(state, segment_info)
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Err(LuceneError::illegal_argument(format!(
      "Could not load postings format named \"{name}\""
    )))
  }
}

pub struct CrankyFieldsConsumer<FC> {
  delegate: FC,
  random: Arc<Mutex<StdRng>>,
}

impl<FC> CrankyFieldsConsumer<FC> {
  fn new(delegate: FC, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<FC> FieldsConsumer for CrankyFieldsConsumer<FC>
where
  FC: FieldsConsumer,
{
  fn write<D1, D2, F, N>(
    &mut self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    fields: &mut F,
    norms: Option<&N>,
  ) -> Result<()>
  where
    D1: Directory,
    F: Fields,
    N: NormsProducer,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from FieldsConsumer.write()",
      )));
    }
    self.delegate.write(state, segment_info, fields, norms)
  }
}

impl<FC> Closeable for CrankyFieldsConsumer<FC>
where
  FC: FieldsConsumer,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()?;
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from FieldsConsumer.close()",
      )));
    }
    Ok(())
  }
}
