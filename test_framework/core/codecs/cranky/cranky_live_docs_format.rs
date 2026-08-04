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
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::collections::HashSet;
use std::io::Error;
use std::sync::Arc;

pub struct CrankyLiveDocsFormat<LDF> {
  delegate: LDF,
  random: Arc<Mutex<StdRng>>,
}

impl<LDF> CrankyLiveDocsFormat<LDF> {
  pub fn new(delegate: LDF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<LDF> LiveDocsFormat for CrankyLiveDocsFormat<LDF>
where
  LDF: LiveDocsFormat,
{
  type Bits = LDF::Bits;

  fn read_live_docs<D>(
    &self,
    dir: &impl Directory,
    info: &SegmentCommitInfo<D>,
    context: &IOContext,
  ) -> Result<Self::Bits>
  where
    D: Directory,
  {
    self.delegate.read_live_docs(dir, info, context)
  }

  fn write_live_docs<D>(
    &self,
    bits: &impl Bits,
    dir: &impl Directory,
    info: &SegmentCommitInfo<D>,
    new_del_count: i32,
    context: &IOContext,
  ) -> Result<()>
  where
    D: Directory,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from LiveDocsFormat.writeLiveDocs()",
      )));
    }
    self
      .delegate
      .write_live_docs(bits, dir, info, new_del_count, context)
  }

  fn files<D>(&self, info: &SegmentCommitInfo<D>, files: &mut HashSet<String>) -> Result<()>
  where
    D: Directory,
  {
    // TODO: is this called only from write? if so we should throw exception!
    self.delegate.files(info, files)
  }
}
