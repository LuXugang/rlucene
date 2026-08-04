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
use crate::core::codecs::compound_format::CompoundFormat;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::io::Error;
use std::sync::Arc;

pub struct CrankyCompoundFormat<CF> {
  delegate: CF,
  random: Arc<Mutex<StdRng>>,
}

impl<CF> CrankyCompoundFormat<CF> {
  pub fn new(delegate: CF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<CF> CompoundFormat for CrankyCompoundFormat<CF>
where
  CF: CompoundFormat,
{
  type Directory<D>
    = CF::Directory<D>
  where
    D: Directory;

  fn get_compound_reader<D>(&self, dir: &D, si: &SegmentInfo<D>) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    self.delegate.get_compound_reader(dir, si)
  }

  fn write<D>(&self, dir: &impl Directory, si: &SegmentInfo<D>, context: &IOContext) -> Result<()>
  where
    D: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from CompoundFormat.write()",
      )));
    }
    self.delegate.write(dir, si, context)
  }
}
