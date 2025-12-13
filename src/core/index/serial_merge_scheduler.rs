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
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

/// A [`MergeScheduler`] that simply does each merge sequentially, using the current thread.
pub struct SerialMergeScheduler;
impl Default for SerialMergeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialMergeScheduler {
    pub fn new() -> Self {
        SerialMergeScheduler
    }
}

impl Closeable for SerialMergeScheduler {
    fn close(&mut self) -> Result<()> {
        todo!()
    }
}
/// Just do the merges in sequence.
/// We do this "synchronized" so that even if the application is using multiple threads,
/// only one merge may run at a time.
impl MergeScheduler for SerialMergeScheduler {
    fn merge<MS, D, L, B>(
        &self,
        merge_source: &mut MS,
        _trigger: MergeTrigger,
        index_writer: &IndexWriter<D, L, B>,
    ) -> Result<()>
    where
        MS: MergeSource,
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        match merge_source.get_next_merge(index_writer)? {
            Some(merge) => merge_source.merge(merge, index_writer),
            None => Ok(()),
        }
    }
}
