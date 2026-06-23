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
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::close::CloseableRef;

pub struct DummyMergeScheduler;

impl CloseableRef for DummyMergeScheduler {
  fn close(&self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }
}

impl MergeScheduler for DummyMergeScheduler {
  fn merge<MS, D>(
    &self,
    _merge_source: &MS,
    _trigger: MergeTrigger,
    _writer: &IndexWriter<D>,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    MS: MergeSource,
    D: Directory + 'static,
  {
    dummy_unreachable!()
  }

  type Directory<D>
    = DummyDirectory
  where
    D: Directory;

  fn wrap_for_merge<D>(
    &self,
    _in_: D,
  ) -> crate::core::util::error::lucene_error::Result<Self::Directory<D>>
  where
    D: Directory,
  {
    dummy_unreachable!()
  }
}
