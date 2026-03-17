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
#[cfg(test)]
use rand::{Rng, RngExt};

/// MergeTrigger is passed to `MergePolicy::find_merges(MergeTrigger, SegmentInfos, MergeContext)`
/// to indicate the event that triggered the merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeTrigger {
    /// Merge was triggered by a segment flush.
    SegmentFlush,

    /// Merge was triggered by a full flush. Full flushes can be caused by a commit,
    /// NRT reader reopen or a close call on the index writer.
    FullFlush,

    /// Merge has been triggered explicitly by the user.
    Explicit,

    /// Merge was triggered by a successfully finished merge.
    MergeFinished,

    /// Merge was triggered by a closing IndexWriter.
    Closing,

    /// Merge was triggered on commit.
    Commit,

    /// Merge was triggered on opening NRT readers.
    GetReader,

    /// Merge was triggered by an `IndexWriter::add_indexes(CodecReader...)` operation.
    AddIndexes,
}
#[cfg(test)]
impl MergeTrigger {
    pub(crate) fn random_trigger<R: Rng + ?Sized>(random: &mut R) -> MergeTrigger {
        match random.random_range(0..8) {
            0 => MergeTrigger::SegmentFlush,
            1 => MergeTrigger::FullFlush,
            2 => MergeTrigger::Explicit,
            3 => MergeTrigger::MergeFinished,
            4 => MergeTrigger::Closing,
            5 => MergeTrigger::Commit,
            6 => MergeTrigger::GetReader,
            7 => MergeTrigger::AddIndexes,
            _ => unreachable!(),
        }
    }
}
