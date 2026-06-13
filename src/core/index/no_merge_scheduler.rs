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
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
/// A [`MergeScheduler`] which never executes any merges.
///
/// Use it if you want to prevent an [`IndexWriter`] from ever executing merges,
/// regardless of the [`MergePolicy`](crate::core::index::merge_policy::MergePolicy) used.
///
/// Note that you can achieve the same thing by using [`NoMergePolicy`](crate::core::index::no_merge_policy::NoMergePolicy).
/// However, with [`NoMergeScheduler`] you also ensure that no unnecessary code
/// of any [`MergeScheduler`] implementation is ever executed.
///
/// Hence, it is recommended to use both [`NoMergePolicy`](crate::core::index::no_merge_policy::NoMergePolicy) and
/// [`NoMergeScheduler`] if you want to disable merges from ever happening.
pub struct NoMergeScheduler;
impl Default for NoMergeScheduler {
  fn default() -> Self {
    Self::new()
  }
}

impl NoMergeScheduler {
  pub fn new() -> Self {
    Self
  }
}

impl CloseableRef for NoMergeScheduler {}

impl MergeScheduler for NoMergeScheduler {
  fn merge<MS, D>(
    &self,
    _merge_source: &MS,
    _trigger: MergeTrigger,
    _index_writer: &IndexWriter<D>,
  ) -> Result<()>
  where
    MS: MergeSource,
    D: Directory,
  {
    Ok(())
  }

  type Directory<D>
    = D
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    Ok(in_)
  }
}
