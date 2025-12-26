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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::OneMerge;
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_scheduler::NoMergeScheduler;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::store::directory::{Directory, DirectoryEnum2};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
pub trait MergeScheduler: Closeable {
    fn merge<MS, D, L, B>(
        &self,
        merge_source: &mut MS,
        trigger: MergeTrigger,
        writer: &IndexWriter<D, L, B>,
    ) -> Result<()>
    where
        MS: MergeSource,
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;
    type Directory: Directory;
    fn wrap_for_merge<D, CR>(
        &self,
        _merge: OneMerge<D, CR>,
        _in_: Arc<D>,
    ) -> Result<Option<Self::Directory>>
    where
        D: Directory,
        CR: CodecReader,
    {
        Ok(None)
    }
}

/// Provides access to new merges and executes the actual merge
pub trait MergeSource {
    /// The merge type produced by this source.
    type OneMerge<D>
    where
        D: Directory;

    /// The `MergeScheduler` calls this method to retrieve the next merge
    /// requested by the `MergePolicy`.
    fn get_next_merge<D, L, B>(
        &self,
        writer: &IndexWriter<D, L, B>,
    ) -> Result<Option<Self::OneMerge<D>>>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;

    /// Does finishing for a merge.
    fn on_merge_finished<D, L, B>(
        &self,
        merge: &mut Self::OneMerge<D>,
        writer: &IndexWriter<D, L, B>,
    ) where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;

    /// Expert: returns true if there are merges waiting to be scheduled.
    fn has_pending_merges<D, L, B>(&self, writer: &IndexWriter<D, L, B>) -> Result<bool>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;

    /// Merges the indicated segments, replacing them in the stack
    /// with a single segment.
    fn merge<D, L, B>(&self, merge: Self::OneMerge<D>, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;
}
pub enum MergeSchedulerEnum {
    Serial(SerialMergeScheduler),
    No(NoMergeScheduler),
}

impl Closeable for MergeSchedulerEnum {
    fn close(&mut self) -> Result<()> {
        match self {
            MergeSchedulerEnum::Serial(s) => s.close(),
            MergeSchedulerEnum::No(n) => n.close(),
        }
    }
}

impl MergeScheduler for MergeSchedulerEnum {
    fn merge<MS, D, L, B>(
        &self,
        merge_source: &mut MS,
        trigger: MergeTrigger,
        index_writer: &IndexWriter<D, L, B>,
    ) -> Result<()>
    where
        MS: MergeSource,
        D: Directory,
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        match self {
            MergeSchedulerEnum::Serial(s) => s.merge(merge_source, trigger, index_writer),
            MergeSchedulerEnum::No(n) => n.merge(merge_source, trigger, index_writer),
        }
    }

    type Directory = DirectoryEnum2<
        <SerialMergeScheduler as MergeScheduler>::Directory,
        <NoMergeScheduler as MergeScheduler>::Directory,
    >;

    fn wrap_for_merge<D, CR>(
        &self,
        merge: OneMerge<D, CR>,
        in_: Arc<D>,
    ) -> Result<Option<Self::Directory>>
    where
        D: Directory,
        CR: CodecReader,
    {
        match self {
            MergeSchedulerEnum::Serial(s) => {
                let v = s.wrap_for_merge(merge, in_)?;
                match v {
                    Some(e) => Ok(Some(DirectoryEnum2::A(e))),
                    None => Ok(None),
                }
            },
            MergeSchedulerEnum::No(n) => {
                let v = n.wrap_for_merge(merge, in_)?;
                match v {
                    Some(e) => Ok(Some(DirectoryEnum2::B(e))),
                    None => Ok(None),
                }
            },
        }
    }
}
