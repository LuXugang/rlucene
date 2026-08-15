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
use crate::core::codecs::norms_format::NormsFormat;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::io::Error;
use std::sync::Arc;

pub struct CrankyNormsFormat<NF> {
  delegate: NF,
  random: Arc<Mutex<StdRng>>,
}

impl<NF> CrankyNormsFormat<NF> {
  pub fn new(delegate: NF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<NF> NormsFormat for CrankyNormsFormat<NF>
where
  NF: NormsFormat,
{
  type NormsConsumer<O: IndexOutput> = CrankyNormsConsumer<NF::NormsConsumer<O>>;

  fn norms_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::NormsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from NormsFormat.normsConsumer()",
      )));
    }
    Ok(CrankyNormsConsumer::new(
      self.delegate.norms_consumer(state, segment_info)?,
      Arc::clone(&self.random),
    ))
  }

  type NormsProducer<I: IndexInput> = NF::NormsProducer<I>;

  fn norms_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::NormsProducer<D1::IndexInput>>
  where
    D1: Directory,
  {
    self.delegate.norms_producer(state, segment_info)
  }
}

pub struct CrankyNormsConsumer<NC> {
  delegate: NC,
  random: Arc<Mutex<StdRng>>,
}

impl<NC> CrankyNormsConsumer<NC> {
  fn new(delegate: NC, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<NC> Closeable for CrankyNormsConsumer<NC>
where
  NC: NormsConsumer,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()?;
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from NormsConsumer.close()",
      )));
    }
    Ok(())
  }
}

impl<NC> NormsConsumer for CrankyNormsConsumer<NC>
where
  NC: NormsConsumer,
{
  fn add_norms_field(
    &mut self,
    field: &Arc<FieldInfo>,
    values_producer: &mut impl NormsProducer,
  ) -> Result<()> {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from NormsConsumer.addNormsField()",
      )));
    }
    self.delegate.add_norms_field(field, values_producer)
  }
}
