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
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use crate::core::index::codec_reader::CodecReader;
use crate::core::index::dummy::dummy_doc_map_sorter::DummyDocMap;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_reader::SegmentReader;
use crate::core::index::sorter::DocMap;
use crate::core::store::directory::Directory;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::InfoStreamMT;
use parking_lot::{Condvar, Mutex};

pub trait MergePolicy {}
/// OneMerge provides the information necessary to perform an individual
/// primitive merge operation, resulting in a single new segment.
///
/// The merge spec includes:
/// - the subset of segments to be merged
/// - whether the new segment should use the compound file format
pub struct OneMerge<CR, B, T>
where
    CR: CodecReader,
    B: Bits,
    T: OneMergeBase,
{
    pub(crate) register_done: bool,
    pub(crate) merge_gen: i64,
    pub(crate) is_external: bool,
    pub(crate) max_num_segments: i32,
    pub(crate) uses_pooled_readers: bool,
    /// Estimated size in bytes of the merged segment.
    pub estimated_merge_bytes: AtomicI64,
    /// Sum of sizeInBytes of all SegmentInfos; set by IW.mergeInit
    pub(crate) total_merge_bytes: AtomicI64,
    info_id: Option<String>,
    merge_readers: Vec<MergeReader<CR, B>>,
    /// Segments to be merged.
    segments: Vec<String>,
    /// Control used to pause/stop/resume the merge thread.
    merge_progress: OneMergeProgress,
    pub(crate) merge_start_ns: AtomicI64,
    /// Total number of documents in segments to be merged, not accounting for deletions.
    pub(crate) total_max_doc: i32,
    error: Mutex<Option<LuceneError>>,
    sub: T,
}
impl<CR, B> OneMerge<CR, B, DefaultOneMergeBaseImpl>
where
    CR: CodecReader,
    B: Bits,
{
    pub fn new<D>(segments: &[SegmentCommitInfo<D>]) -> Result<Self>
    where
        D: Directory,
    {
        if segments.is_empty() {
            return Err(LuceneError::illegal_state(
                "segments must include at least one segment",
            ));
        }
        let mut v = Vec::with_capacity(segments.len());
        let mut total_max_doc = 0;
        for s in segments.iter() {
            v.push(s.info.get_id_str());
            total_max_doc += s.info.max_doc()?
        }

        Ok(Self {
            register_done: false,
            merge_gen: 0,
            is_external: false,
            max_num_segments: -1,
            uses_pooled_readers: true,
            estimated_merge_bytes: AtomicI64::new(0),
            total_merge_bytes: AtomicI64::new(0),
            info_id: None,
            merge_readers: Vec::new(),
            segments: v,
            merge_progress: OneMergeProgress::new(),
            merge_start_ns: AtomicI64::new(-1),
            total_max_doc,
            error: Mutex::new(None),
            sub: DefaultOneMergeBaseImpl,
        })
    }
}
impl<CR, B, T> OneMerge<CR, B, T>
where
    CR: CodecReader,
    B: Bits,
    T: OneMergeBase,
{
    /// Constructor for wrapping.
    pub(crate) fn from_other(one_merge: OneMerge<CR, B, T>, sub: T) -> Self {
        Self {
            segments: one_merge.segments,
            merge_readers: one_merge.merge_readers,
            total_max_doc: one_merge.total_max_doc,
            merge_progress: OneMergeProgress::new(),
            uses_pooled_readers: one_merge.uses_pooled_readers,
            register_done: false,
            merge_gen: 0,
            is_external: false,
            max_num_segments: -1,
            estimated_merge_bytes: AtomicI64::new(0),
            total_merge_bytes: AtomicI64::new(0),
            info_id: None,
            merge_start_ns: AtomicI64::new(-1),
            error: Mutex::new(None),
            sub,
        }
    }
}
impl<CR> OneMerge<CR, <CR as LeafReader>::Bits, DefaultOneMergeBaseImpl>
where
    CR: CodecReader,
{
    /// Create a OneMerge directly from CodecReaders. Used to merge incoming readers in
    /// IndexWriter::add_indexes(reader...). This OneMerge works directly on readers and has an
    /// empty segments list.
    pub fn from_codec_readers(readers: Vec<CR>) -> Result<Self> {
        let mut merge_readers = Vec::with_capacity(readers.len());
        let mut total_docs = 0;

        for r in readers.into_iter() {
            let live_docs = r.get_live_docs()?;
            total_docs += r.num_docs()?;
            merge_readers.push(MergeReader::new(r, live_docs));
        }

        Ok(Self {
            register_done: false,
            merge_gen: 0,
            is_external: false,
            max_num_segments: -1,
            uses_pooled_readers: false,
            estimated_merge_bytes: AtomicI64::new(0),
            total_merge_bytes: AtomicI64::new(0),
            info_id: None,
            merge_readers,
            segments: Vec::new(),
            merge_progress: OneMergeProgress::new(),
            merge_start_ns: AtomicI64::new(-1),
            total_max_doc: total_docs,
            error: Mutex::new(None),
            sub: DefaultOneMergeBaseImpl,
        })
    }
}

pub trait OneMergeBase {
    fn merge_finished(&self, success: bool, segment_dropped: bool) -> Result<()>;
    type CodecReader: CodecReader;
    fn wrap_for_merge<CR>(&self, reader: CR) -> Result<Option<Self::CodecReader>>;
    // TODO IMPORTANT 多线程参数未定义
    type DocMap: DocMap;
    fn reorder<CR, D>(&self, dir: D) -> Result<Self::DocMap>;
    fn set_merge_info<D>(info: &SegmentCommitInfo<D>)
    where
        D: Directory;
    fn on_merge_complete(&self) -> Result<()>;
    // TODO IMPORTANT 闭包未定义
    fn init_merge_readers(&self) -> Result<()>;
}
#[derive(Default)]
struct DefaultOneMergeBaseImpl;
impl OneMergeBase for DefaultOneMergeBaseImpl {
    fn merge_finished(&self, _success: bool, _segment_dropped: bool) -> Result<()> {
        todo!()
    }

    type CodecReader = SegmentReader<DummyDirectory>;

    fn wrap_for_merge<CR>(&self, _reader: CR) -> Result<Option<Self::CodecReader>> {
        todo!()
    }

    type DocMap = DummyDocMap;

    fn reorder<CR, D>(&self, _dir: D) -> Result<Self::DocMap> {
        todo!()
    }

    fn set_merge_info<D>(_info: &SegmentCommitInfo<D>)
    where
        D: Directory,
    {
        todo!()
    }

    fn on_merge_complete(&self) -> Result<()> {
        todo!()
    }

    fn init_merge_readers(&self) -> Result<()> {
        todo!()
    }
}

/// Reason for pausing the merge thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PauseReason {
    /// Stopped (because of throughput rate set to 0, typically).
    Stopped,
    /// Temporarily paused because of exceeded throughput rate.
    Paused,
    /// Other reason.
    Other,
}
/// Progress and state for an executing merge. This struct encapsulates the
/// logic to pause and resume the merge thread or to abort the merge entirely.
pub struct OneMergeProgress {
    pause_lock: Mutex<()>,
    pausing: Condvar,
    /// Pause times (in nanoseconds) for each [`PauseReason`](PauseReason).
    pause_times: PauseTimes,
    aborted: AtomicBool,
    /// This field is for sanity-check purpos only. Only the same thread that
    //     /// invoked `OneMerge#mergeInit()` is permiestted to be calling `pauseNanos`.
    /// This is always verified at runtime.
    owner: Mutex<Option<ThreadId>>,
}

#[derive(Default)]

struct PauseTimes {
    stopped: AtomicU64,
    paused: AtomicU64,
    other: AtomicU64,
}

impl Default for OneMergeProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl OneMergeProgress {
    /// Creates a new merge progress info.
    pub fn new() -> Self {
        Self {
            pause_lock: Mutex::new(()),
            pausing: Condvar::new(),
            // Place all the pause reasons in there immediately so that we can
            // simply update values.
            pause_times: PauseTimes::default(),
            aborted: AtomicBool::new(false),
            owner: Mutex::new(None),
        }
    }
    /// Abort the merge this progress tracks at the next possible moment.
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Relaxed);
        self.wakeup(); // wakeup any paused merge thread.
    }
    /// Return the aborted state of this merge.
    pub fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::Relaxed)
    }

    /// Pauses the calling thread for at least `pause_nanos` nanoseconds unless
    /// the merge is aborted or the external condition returns `false`, in
    /// which case control returns immediately.
    ///
    /// The external condition is required so that other threads can terminate
    /// the pausing immediately before `pause_nanos` expires. We can't rely
    /// on just `Condvar::wait_timeout_while()` alone because it can return
    /// due to spurious wakeups too.
    ///
    /// # Arguments
    /// - `condition`: The pause condition that should return `false` if
    ///   immediate return from this method is needed. Other threads can wake up
    ///   any sleeping thread by calling [`wakeup()`](OneMergeProgress::wakeup),
    ///   but the thread may sleep for the remainder of the requested time if
    ///   this condition remains `true`.
    pub fn pause_nanos<F>(&self, pause_nanos: u64, reason: PauseReason, condition: F)
    where
        F: Fn() -> bool,
    {
        {
            let owner = self.owner.lock();
            let current_id = thread::current().id();
            debug_assert_eq!(
                *owner,
                Some(current_id),
                "Only owner thread can pause merge"
            );
        }

        let start = Instant::now();
        let deadline = start + Duration::from_nanos(pause_nanos);

        let mut lock = self.pause_lock.lock();
        while !self.aborted.load(Ordering::Relaxed) && condition() {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let timeout = deadline - now;
            self.pausing.wait_for(&mut lock, timeout);
        }

        let elapsed = start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        self.add_pause_time(reason, elapsed);
    }

    fn add_pause_time(&self, reason: PauseReason, nanos: u64) {
        match reason {
            PauseReason::Stopped => self.pause_times.stopped.fetch_add(nanos, Ordering::Relaxed),
            PauseReason::Paused => self.pause_times.paused.fetch_add(nanos, Ordering::Relaxed),
            PauseReason::Other => self.pause_times.other.fetch_add(nanos, Ordering::Relaxed),
        };
    }
    /// Request a wakeup for any threads stalled in
    /// [`pauseNanos`](OneMergeProgress::pause_nanos).
    pub fn wakeup(&self) {
        let _lock = self.pause_lock.lock();
        self.pausing.notify_all();
    }
    /// Returns pause reasons and associated times in nanoseconds.
    pub fn get_pause_times(&self) -> HashMap<PauseReason, u64> {
        let mut map = HashMap::new();
        map.insert(
            PauseReason::Stopped,
            self.pause_times.stopped.load(Ordering::Relaxed),
        );
        map.insert(
            PauseReason::Paused,
            self.pause_times.paused.load(Ordering::Relaxed),
        );
        map.insert(
            PauseReason::Other,
            self.pause_times.other.load(Ordering::Relaxed),
        );
        map
    }
    pub fn set_merge_thread(&self) {
        let mut owner = self.owner.lock();
        debug_assert!(owner.is_none());
        *owner = Some(thread::current().id());
    }
}
/// This trait represents the current context of the merge selection process.
/// It allows access to real-time information such as:
/// - the segments currently being merged
/// - how many deletes a segment would reclaim if merged
///
/// This context may be stateful and can change during the execution of a
/// merge policy's selection processes.
pub trait MergeContext {
    /// Returns the number of deletes a merge would claim back
    /// if the given segment is merged.
    ///
    /// See [`MergePolicy::num_deletes_to_merge`].
    ///
    /// * `info` — the segment to get the number of deletes for
    fn num_deletes_to_merge<D>(&mut self, info: &SegmentCommitInfo<D>) -> Result<i32>
    where
        D: Directory;

    /// Returns the number of deleted documents in the given segment.
    fn num_deleted_docs<D>(&self, info: &SegmentCommitInfo<D>) -> i32
    where
        D: Directory;

    /// Returns the info stream that can be used to log messages.
    fn get_info_stream(&self) -> InfoStreamMT;

    /// Returns an unmodifiable set of segments that are currently merging.
    fn get_merging_segments(&self) -> HashSet<String>;
}

pub(crate) struct MergeReader<CR, B>
where
    CR: CodecReader,
    B: Bits,
{
    codec_reader: CR,
    hard_live_docs: Option<B>,
}
impl<CR, B> MergeReader<CR, B>
where
    CR: CodecReader,
    B: Bits,
{
    pub(crate) fn new(codec_reader: CR, hard_live_docs: Option<B>) -> Self {
        Self {
            codec_reader,
            hard_live_docs,
        }
    }
}
