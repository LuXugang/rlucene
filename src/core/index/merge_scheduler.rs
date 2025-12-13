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
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::index::no_merge_scheduler::NoMergeScheduler;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
pub trait MergeScheduler: Closeable {
    fn merge<MS>(&self, merge_source: &mut MS, trigger: MergeTrigger) -> Result<()>
    where
        MS: MergeSource;
}

/// Provides access to new merges and executes the actual merge
pub trait MergeSource {
    /// The merge type produced by this source.
    type OneMerge;

    /// The `MergeScheduler` calls this method to retrieve the next merge
    /// requested by the `MergePolicy`.
    fn get_next_merge(&mut self) -> Result<Option<Self::OneMerge>>;

    /// Does finishing for a merge.
    fn on_merge_finished(&mut self, merge: &Self::OneMerge);

    /// Expert: returns true if there are merges waiting to be scheduled.
    fn has_pending_merges(&self) -> bool;

    /// Merges the indicated segments, replacing them in the stack
    /// with a single segment.
    fn merge(&mut self, merge: Self::OneMerge) -> Result<()>;
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
    fn merge<MS>(&self, merge_source: &mut MS, trigger: MergeTrigger) -> Result<()>
    where
        MS: MergeSource,
    {
        match self {
            MergeSchedulerEnum::Serial(s) => s.merge(merge_source, trigger),
            MergeSchedulerEnum::No(n) => n.merge(merge_source, trigger),
        }
    }
}
