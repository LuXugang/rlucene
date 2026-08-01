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
use crate::core::codecs::Codec;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::index::index_reader::Identity;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::{DefaultLiveDocsFormat, TestUtil};
use std::collections::HashSet;

/// Just like the default live docs format but with additional asserts.
pub struct AssertingLiveDocsFormat {
  in_: DefaultLiveDocsFormat,
}

impl AssertingLiveDocsFormat {
  pub fn new() -> Self {
    Self {
      in_: TestUtil::get_default_codec().live_docs_format(),
    }
  }
}

impl AssertingLiveDocsFormat {
  fn check(bits: &impl Bits, expected_length: usize, expected_delete_count: i32) -> Result<()> {
    assert_eq!(bits.length(), expected_length);
    let mut deleted_count = 0;
    for i in 0..bits.length() {
      if !bits.get(i)? {
        deleted_count += 1;
      }
    }
    assert_eq!(
      deleted_count, expected_delete_count,
      "deleted: {deleted_count} != expected: {expected_delete_count}"
    );
    Ok(())
  }
}

impl LiveDocsFormat for AssertingLiveDocsFormat {
  type Bits = AssertingBits<<DefaultLiveDocsFormat as LiveDocsFormat>::Bits>;

  fn read_live_docs<D>(
    &self,
    dir: &impl Directory,
    info: &SegmentCommitInfo<D>,
    context: &IOContext,
  ) -> Result<Self::Bits>
  where
    D: Directory,
  {
    let raw = self.in_.read_live_docs(dir, info, context)?;
    Self::check(&raw, info.info.max_doc()? as usize, info.get_del_count())?;
    Ok(AssertingBits::new(raw))
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
    Self::check(
      bits,
      info.info.max_doc()? as usize,
      info.get_del_count() + new_del_count,
    )?;
    self
      .in_
      .write_live_docs(bits, dir, info, new_del_count, context)
  }

  fn files<D>(&self, info: &SegmentCommitInfo<D>, files: &mut HashSet<String>) -> Result<()>
  where
    D: Directory,
  {
    self.in_.files(info, files)
  }
}

pub struct AssertingBits<B>
where
  B: Bits,
{
  in_: B,
  identity: Identity,
}

impl<B> AssertingBits<B>
where
  B: Bits,
{
  fn new(in_: B) -> Self {
    // Do a simple check on initialization.
    let _ = in_.length();
    Self {
      in_,
      identity: Identity::new(),
    }
  }
}

impl<B> HasIdentity for AssertingBits<B>
where
  B: Bits,
{
  fn identity(&self) -> &Identity {
    &self.identity
  }
}

impl<B> Bits for AssertingBits<B>
where
  B: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    assert!(index < self.in_.length());
    self.in_.get(index)
  }

  fn length(&self) -> usize {
    self.in_.length()
  }
}
