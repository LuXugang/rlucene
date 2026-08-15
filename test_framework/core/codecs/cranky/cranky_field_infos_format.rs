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
use crate::core::codecs::field_infos_format::FieldInfosFormat;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::io::Error;
use std::sync::Arc;

pub struct CrankyFieldInfosFormat<FIF> {
  delegate: FIF,
  random: Arc<Mutex<StdRng>>,
}

impl<FIF> CrankyFieldInfosFormat<FIF> {
  pub fn new(delegate: FIF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<FIF> FieldInfosFormat for CrankyFieldInfosFormat<FIF>
where
  FIF: FieldInfosFormat,
{
  fn read<D>(
    &self,
    directory: &impl Directory,
    segment_info: &SegmentInfo<D>,
    segment_suffix: &str,
    io_context: &IOContext,
  ) -> Result<FieldInfos> {
    self
      .delegate
      .read(directory, segment_info, segment_suffix, io_context)
  }

  fn write<D>(
    &self,
    directory: &impl Directory,
    segment_info: &SegmentInfo<D>,
    segment_suffix: &str,
    infos: &FieldInfos,
    io_context: &IOContext,
  ) -> Result<()> {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from FieldInfosFormat.getFieldInfosWriter()",
      )));
    }
    self
      .delegate
      .write(directory, segment_info, segment_suffix, infos, io_context)
  }
}
