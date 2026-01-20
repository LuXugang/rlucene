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
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::store::directory::Directory;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::close::Closeable;

pub struct DummyMergeScheduler;

impl Closeable for DummyMergeScheduler {
    fn close(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl MergeScheduler for DummyMergeScheduler {
    fn merge<MS, D, L, B>(
        &self,
        _merge_source: &MS,
        _trigger: MergeTrigger,
        _writer: &IndexWriter<D, L, B>,
    ) -> crate::core::util::error::lucene_error::Result<()>
    where
        MS: MergeSource,
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
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
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
