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
use crate::index::buffered_updates_stream::BufferedUpdatesStream;
use crate::index::documents_writer::{DocumentsWriter, FlushNotifications};
use crate::index::frozen_buffered_updates::FrozenBufferedUpdates;
use crate::index::index_file_deleter::IndexFileDeleter;
use crate::index::merge_state::DocMap;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_infos::SegmentInfos;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::long_supplier::LongSupplier;
use parking_lot::{Mutex, ReentrantMutex};
use std::rc::Rc;
use std::sync::Arc;

pub struct IndexWriter<D, L, B>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
{
    enable_test_points: bool,
    // when unrecoverable disaster strikes, we populate this with the reason that we had to close
    // IndexWriter
    tragedy: TragicException,
    // original user directory
    directory_orig: Arc<D>,
    // wrapped with additional checks
    directory: Arc<LockValidatingDirectoryWrapper<D>>,
    // increments every time a change is completed
    change_count: AtomicI64,
    // last changeCount that was committed
    last_commit_change_count: AtomicI64,
    // list of segmentInfo we will fallback to if the commit fails
    rollback_segments: Vec<SegmentCommitInfo<D>>,
    pending_commit: Option<SegmentInfos<D>>,
    pending_seq_no: AtomicI64,
    pending_commit_change_count: AtomicI64,
    files_to_commit: Vec<String>,
    global_field_number_map: Arc<FieldNumbers>,
    doc_writer: DocumentsWriter<D, L, FlushNotificationsImpl>,
    event_queue: Arc<EventQueue>,
    write_doc_values_lock: ReentrantMutex<()>,
    // used by forceMerge to note those needing merging
    segments_to_merge: HashMap<SegmentCommitInfo<D>, bool>,
    merge_max_num_segments: i32,
    write_lock: Option<D::Lock>,

    closed: AtomicBool,
    closing: AtomicBool,

    maybe_merge: AtomicBool,
    commit_user_data: Option<HashMap<String, String>>,
    merging_segments: HashSet<SegmentCommitInfo<D>>,

    merge_gen: i64,
    did_message_state: bool,
    flush_count: AtomicI32,
    flush_deletes_count: AtomicI32,
    reader_pool: ReaderPool<D>,
    buffered_updates_stream: Rc<BufferedUpdatesStream>,
    merge_finished_gen: AtomicI64,
    config: Arc<L>,
    start_commit_time: i64,
    pending_num_docs: AtomicI64,
    soft_deletes_enabled: bool,
    info_stream: InfoStreamMT,
    inner: Mutex<Inner<D, L>>,
    sub: Option<B>,
}
pub struct Inner<D, L>
where
    D: Directory,
    L: LiveIndexWriterConfig,
{
    segment_infos: SegmentInfos<D>,
    deleter: IndexFileDeleter<D, L::IndexDeletionPolicy>,
}

impl<D, L, B> IndexWriter<D, L, B>
where
    D: Directory,
    L: LiveIndexWriterConfig,
    B: IndexWriterBase,
{
    /// Drops a segment that has 100% deleted documents.
    pub(crate) fn drop_deleted_segment(&self, _info: &SegmentCommitInfo<D>) -> Result<()> {
        todo!()
    }

    fn checkpoint(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        self.changed();
        let (deleter, segment_infos) = {
            let v = &mut *inner;
            (&mut v.deleter, &v.segment_infos)
        };
        deleter.checkpoint(segment_infos, true)?;
        Ok(())
    }
    /// Checkpoints with IndexFileDeleter, so it's aware of new files, and increments changeCount,
    /// so on close/commit we will write a new segments file, but does NOT bump segmentInfos.version.
    fn check_point_no_sis(&self) -> Result<()> {
        let mut inner = self.inner.lock();
        self.change_count.fetch_add(1, Ordering::SeqCst);
        let (deleter, segment_infos) = {
            let v = &mut *inner;
            (&mut v.deleter, &v.segment_infos)
        };
        deleter.checkpoint(segment_infos, false)?;
        Ok(())
    }

    /// Called internally if any index state has changed.
    fn changed(&self) {
        let mut inner = self.inner.lock();
        self.change_count.fetch_add(1, Ordering::SeqCst);
        inner.segment_infos.changed();
    }
    fn publish_frozen_updates(&self, packet: FrozenBufferedUpdates) -> Result<i64> {
        let _guard = self.inner.lock();
        debug_assert!(packet.any());
        let (next_gen, packet) = self.buffered_updates_stream.push(packet);
        // Do this as an event so it applies higher in the stack when we are not holding
        // DocumentsWriterFlushQueue.purgeLock:
        let event: EventEnum<D> = EventEnum::E(EventImpl5::new(packet));
        self.event_queue.add(event)?;
        drop(_guard);
        Ok(next_gen)
    }
    /// Atomically adds the segment private delete packet and publishes the flushed segments SegmentInfo to the index writer.
    fn publish_flushed_segment(
        &self,
        mut new_segment: SegmentCommitInfo<D>,
        field_infos: Rc<FieldInfos>,
        packet: Option<FrozenBufferedUpdates>,
        global_packet: Option<FrozenBufferedUpdates>,
        sort_map: Option<Rc<DocMapImpl>>,
    ) -> Result<()> {
        let mut inner = self.inner.lock();
        let mut published = false;
        let max_doc = new_segment.info.max_doc()?;
        let res: Result<()> = (|| {
            // Lock order IW -> BDS
            self.ensure_open(false)?;

            if self.info_stream.enabled("IW") {
                self.info_stream
                    .message("IW", &format!("publishFlushedSegment {}", new_segment));
            }

            if let Some(gp) = global_packet
                && gp.any()
            {
                let _ = self.publish_frozen_updates(gp)?;
            }
            // Publishing the segment must be sync'd on IW -> BDS to make the sure
            // that no merge prunes away the seg. private delete packet
            let packet_any = match packet {
                Some(ref p) => p.any(),
                None => false,
            };
            let next_gen = if packet_any {
                self.publish_frozen_updates(packet.unwrap())?
            } else {
                // Since we don't have a delete packet to apply we can get a new
                // generation right away
                let v = self.buffered_updates_stream.get_next_gen();
                // No deletes/updates here, so marked finished immediately:
                self.buffered_updates_stream.finished_segment(v);
                v
            };

            if self.info_stream.enabled("IW") {
                // let segs = self.seg_string(&new_segment);
                // self.info_stream.message(
                //     "IW",
                //     &format!(
                //         "publish sets newSegment delGen={} seg={}",
                //         next_gen, segs
                //     ),
                // );
            }
            new_segment.set_buffered_deletes_gen(next_gen)?;
            let new_segment_id = new_segment.info.get_id_str();
            inner.segment_infos.add(new_segment)?;
            let index_created_version_major = inner.segment_infos.get_index_created_version_major();
            let new_segment = inner.segment_infos.info_mut(&new_segment_id).unwrap();
            published = true;
            self.checkpoint()?;
            if packet_any {
                let _ = self.get_pooled_instance(
                    new_segment,
                    true,
                    index_created_version_major,
                    sort_map,
                )?;
            }
            // this is a corner case where documents delete them-self with soft deletes. This is used to
            // build delete tombstones etc. in this case we haven't seen any updates to the DV in this
            // fresh flushed segment.
            // if we have seen updates the update code checks if the segment is fully deleted.
            let has_initial_soft_deleted = {
                if let Some(name) = self.config.get_soft_deletes_field() {
                    if let Some(fi) = field_infos.field_info_by_name(name) {
                        fi.get_doc_values_gen() == -1
                            && *fi.get_doc_values_type() != DocValuesType::None
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            let is_fully_hard_deleted =
                new_segment.get_del_count() == new_segment.info.max_doc()?;
            // we either have a fully hard-deleted segment or one or more docs are soft-deleted. In both
            // cases we need
            // to go and check if they are fully deleted. This has the nice side-effect that we now have
            // accurate numbers
            // for the soft delete right after we flushed to disk.
            if has_initial_soft_deleted || is_fully_hard_deleted {
                let rld =
                    self.get_pooled_instance(new_segment, true, index_created_version_major, None)?;
                let result: Result<()> = (|| {
                    match rld {
                        None => {
                            return Err(LuceneError::illegal_state(
                                "failed to open newly flushed segment",
                            ));
                        },
                        Some(ref rld) => {
                            if self.is_fully_deleted(rld, new_segment)? {
                                self.drop_deleted_segment(new_segment)?;
                                self.checkpoint()?;
                            }
                        },
                    }
                    Ok(())
                })();
                self.release(&rld.unwrap(), new_segment)?;
                result?;
            }
            Ok(())
        })();

        if !published {
            self.adjust_pending_num_docs(-(max_doc as i64));
        }
        self.flush_count.fetch_add(1, Ordering::AcqRel);
        if let Some(ref s) = self.sub {
            s.do_after_flush()?
        }

        res
    }

    pub fn set_live_commit_data(&self) {}

    pub fn ensure_open(&self, fail_if_closing: bool) -> Result<()> {
        if self.closed.load(Ordering::SeqCst)
            || (fail_if_closing && self.closing.load(Ordering::SeqCst))
        {
            let tragedy = self.tragedy.lock();
            let error_opt = tragedy.as_ref();
            match error_opt {
                Some(err) => Err(LuceneError::already_closed(format!("{err}"))),
                None => Err(LuceneError::illegal_state("no tragic error set")),
            }
        } else {
            Ok(())
        }
    }
    fn on_tragic_event(&self, _tragedy: &LuceneError, _location: &str) -> Result<()> {
        todo!()
    }

    pub fn get_tragic_exception(&self) -> TragicException {
        self.tragedy.clone()
    }
    pub(crate) fn is_deleter_closed(&self) -> Result<bool> {
        let inner = self.inner.lock();
        inner.deleter.is_closed(self)
    }

    fn delete_new_files<'a, I>(&self, files: I) -> Result<()>
    where
        I: IntoIterator<Item = &'a String>,
    {
        let inner = self.inner.lock();
        inner.deleter.delete_new_files(files)
    }

    fn flush_failed(&self, info: &SegmentInfo<D>) -> Result<()> {
        let inner = self.inner.lock();
        match info.files() {
            Ok(files) => inner.deleter.delete_new_files(files.iter())?,
            Err(_) => { // no-op},
            },
        }
        Ok(())
    }

    fn publish_flushed_segments(&self, forced: bool) -> Result<()> {
        let c = |mut ticket: FlushTicket<D>, writer: &IndexWriter<D, L, B>| {
            let buffered_updates = ticket.take_frozen_updates();
            ticket.mark_published();
            let new_segment = ticket.get_flushed_segment();
            match new_segment {
                // this is a flushed global deletes package - not a segments
                None => {
                    if let Some(buffered_updates) = buffered_updates
                        && buffered_updates.any()
                    {
                        if writer.info_stream.enabled("IW") {
                            self.info_stream.message(
                                "IW",
                                &format!("flush: push buffered updates: {buffered_updates:?}"),
                            );
                        }
                        writer.publish_frozen_updates(buffered_updates)?;
                    }
                },
                Some(seg) => {
                    if self.info_stream.enabled("IW") {
                        self.info_stream.message(
                            "IW",
                            &format!(
                                "publishFlushedSegment seg-private updates={:?}",
                                seg.segment_updates
                            ),
                        );
                    }
                    if seg.segment_updates.is_some() && self.info_stream.enabled("DW") {
                        self.info_stream.message(
                            "IW",
                            &format!(
                                "flush: push buffered seg private updates: {:?}",
                                seg.segment_updates
                            ),
                        );
                    }
                    self.publish_flushed_segment(
                        seg.segment_info.take().unwrap(),
                        seg.field_infos.clone(),
                        seg.segment_updates.take(),
                        buffered_updates,
                        seg.sort_map.take(),
                    )?;
                },
            }
            Ok(())
        };
        self.doc_writer.purge_flush_tickets(forced, self, c)?;
        Ok(())
    }
    fn adjust_pending_num_docs(&self, num_docs: i64) -> i64 {
        let count = self.pending_num_docs.fetch_add(num_docs, Ordering::AcqRel) + num_docs;
        debug_assert!(count >= 0, "pendingNumDocs is negative: {}", count);
        count
    }

    fn is_fully_deleted(
        &self,
        readers_and_updates: &ReadersAndUpdates<D>,
        info: &SegmentCommitInfo<D>,
    ) -> Result<bool> {
        if readers_and_updates.is_fully_deleted(info)? {
            debug_assert!(self.inner.is_locked());
            return Ok(!(readers_and_updates
                .keep_fully_deleted_segment(self.config.get_merge_policy())?));
        }
        Ok(false)
    }

    pub(crate) fn release(
        &self,
        readers_and_updates: &ReadersAndUpdates<D>,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<()> {
        self.do_release(readers_and_updates, true, info)
    }

    fn do_release(
        &self,
        readers_and_updates: &ReadersAndUpdates<D>,
        assert_live_info: bool,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<()> {
        debug_assert!(self.inner.is_locked());
        if self
            .reader_pool
            .release(readers_and_updates, assert_live_info, info)?
        {
            // if we write anything here we have to hold the lock otherwise IDF will delete files
            // underneath us
            self.check_point_no_sis()?;
        }
        Ok(())
    }

    pub(crate) fn get_pooled_instance(
        &self,
        info: &SegmentCommitInfo<D>,
        create: bool,
        index_created_version_major: i32,
        sort_map: Option<Rc<DocMapImpl>>,
    ) -> Result<Option<Rc<ReadersAndUpdates<D>>>> {
        self.ensure_open(false)?;
        self.reader_pool
            .get(info, create, index_created_version_major, sort_map)
    }
    /// Translates a frozen packet of delete term/query, or doc values updates, into their actual
    /// doc IDs in the index, and applies the change. This is a heavy operation and is done concurrently
    /// by incoming indexing threads. This method will return immediately without blocking if another
    /// thread is currently applying the package. To ensure the packet has been applied,
    /// [`IndexWriter::force_apply(FrozenBufferedUpdates)`](Self::force_apply) must be called.
    pub(crate) fn try_apply<U>(&self, updates: U) -> Result<bool>
    where
        U: AsRef<FrozenBufferedUpdates>,
    {
        if updates.as_ref().try_lock() {
            self.force_apply(updates)?;
        }
        Ok(false)
    }
    /// Translates a frozen packet of delete term/query, or doc values updates, into their actual
    /// doc IDs in the index, and applies the change.
    /// This is a heavy operation and is done concurrently by incoming indexing threads.
    pub(crate) fn force_apply<U>(&self, _updates: U) -> Result<bool>
    where
        U: AsRef<FrozenBufferedUpdates>,
    {
        todo!()
    }
}
pub trait IndexWriterBase {
    /// A hook for extending classes to execute operations after pending added and deleted documents have been flushed to the Directory
    /// but before the change is committed (new segments_N file written).
    fn do_after_flush(&self) -> Result<()>;
    /// A hook for extending classes to execute operations before pending added and deleted documents are flushed to the Directory.
    fn do_before_flush(&self) -> Result<()>;
}
pub(crate) type TragicException = Arc<Mutex<Option<LuceneError>>>;

#[derive(Default)]
pub struct DocMapIndexWriter;
impl DocMap for DocMapIndexWriter {
    fn get(&self, _doc_id: i32) -> i32 {
        todo!()
    }
}

pub(crate) struct FlushNotificationsImpl;
impl FlushNotifications for FlushNotificationsImpl {
    fn delete_unused_files<'a, I>(&self, _files: I)
    where
        I: IntoIterator<Item = &'a String>,
    {
        todo!()
    }

    fn flush_failed<D>(&self, _info: SegmentInfo<D>)
    where
        D: Directory,
    {
        todo!()
    }

    fn after_segments_flushed(&self) -> Result<()> {
        todo!()
    }

    fn on_tragic_event(&self, _event: LuceneError, _message: &str) {
        todo!()
    }

    fn on_deletes_applied(&self) {
        todo!()
    }

    fn on_ticket_backlog(&self) {
        todo!()
    }
}

pub(crate) struct LongSupplierImpl {
    stream: Rc<BufferedUpdatesStream>,
}
impl LongSupplier for LongSupplierImpl {
    fn get_as_long(&self) -> i64 {
        self.stream.get_completed_del_gen()
    }
}

use crate::codecs::{Codec, CompoundFormat, LATEST_CODEC};
use crate::index::doc_values_type::DocValuesType;
use crate::index::documents_writer_flush_queue::FlushTicket;
use crate::index::field_infos::{FieldInfos, FieldNumbers};
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::reader_pool::ReaderPool;
use crate::index::readers_and_updates::ReadersAndUpdates;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::sorter::DocMapImpl;
use crate::store::IOContext;
use crate::store::lock_validating_directory_wrapper::LockValidatingDirectoryWrapper;
use crate::store::tracking_directory_wrapper::TrackingDirectoryWrapper;
use crate::util::array_util::ArrayUtil;
use crate::util::constants::Constants;
use crate::util::info_stream::{InfoStream, InfoStreamMT};
use crate::util::io_consumer::IOConsumer;
use crate::util::unicode_util::UnicodeUtil;
use crate::util::{BYTE_BLOCK_SIZE, LATEST};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};

/// Maximum number of documents. In Java Lucene, We subtract 128 to ensure
/// it's well below the typical JVM's `ArrayUtil.MAX_ARRAY_LENGTH` and
/// avoid potential overflow issues across JVM implementations.
/// In Rust Lucene, we keep the same value for consistency.
pub const MAX_DOCS: i32 = i32::MAX - 128;
/// Maximum value for the token position in an indexed field.
pub const MAX_POSITION: i32 = i32::MAX - 128;
/// A variable that holds the actual maximum number of documents, which can
/// be adjusted for testing purposes.
pub const ACTUAL_MAX_DOCS: i32 = MAX_DOCS;

pub const MAX_TERM_LENGTH: i32 = BYTE_BLOCK_SIZE - 1;
pub const WRITE_LOCK_NAME: &str = "write.lock";
/// Key for the source of a segment in the [`SegmentInfo#get_diagnostics()`](crate::index::segment_info::SegmentInfo::get_diagnostics)
pub const SOURCE: &str = "source";
/// Source of a segment which results from a merge of other segments.
pub const SOURCE_MERGE: &str = "merge";
/// Source of a segment which results from a flush.
pub const SOURCE_FLUSH: &str = "flush";
pub const MAX_STORED_STRING_LENGTH: i32 =
    ArrayUtil::MAX_ARRAY_LENGTH as i32 / UnicodeUtil::MAX_UTF8_BYTES_PER_CHAR;
pub(crate) fn get_actual_max_docs() -> i32 {
    ACTUAL_MAX_DOCS
}
/// Convenience overload: no extra details.
pub(crate) fn set_diagnostics<D>(info: &mut SegmentInfo<D>, source: &str)
where
    D: Directory,
{
    set_diagnostics_impl(info, source, None)
}
fn set_diagnostics_impl<D>(
    info: &mut SegmentInfo<D>,
    source: &str,
    details: Option<HashMap<String, String>>,
) where
    D: Directory,
{
    let mut diagnostics = HashMap::new();
    diagnostics.insert("source".to_string(), source.to_string());
    diagnostics.insert("lucene.version".to_string(), LATEST.to_string());
    diagnostics.insert("os".to_string(), Constants::os_name());
    diagnostics.insert("os.arch".to_string(), Constants::os_arch());
    diagnostics.insert("os.version".to_string(), Constants::os_version());
    diagnostics.insert(
        "timestamp".to_string(),
        chrono::Utc::now().timestamp_millis().to_string(),
    );
    if let Some(details) = details {
        for (k, v) in details {
            diagnostics.insert(k, v);
        }
    }
    info.set_diagnostics(diagnostics);
}
/// NOTE: this method creates a compound file for all files returned by `info.files()`. While,
/// generally, this may include separate norms and deletion files, this `SegmentInfo` must not
/// reference such files when this method is called, because they are not allowed within a compound
/// file.
pub(crate) fn create_compound_file<D, T, D2>(
    info_stream: &InfoStreamMT,
    directory: &TrackingDirectoryWrapper<D>,
    info: &mut SegmentInfo<D2>,
    context: &IOContext,
    mut delete_files: T,
) -> Result<()>
where
    D: Directory,
    D2: Directory,
    T: for<'a> IOConsumer<&'a HashSet<String>>,
{
    // maybe this check is not needed, but why take the risk?
    if !directory
        .get_created_files()
        .lock()
        .created_filenames
        .is_empty()
    {
        return Err(LuceneError::illegal_state(
            "pass a clean trackingdir for CFS creation",
        ));
    }

    {
        if info_stream.enabled("IW") {
            info_stream.message("IW", "create compound file");
        }
    }
    // Now merge all added files
    let write_result = (|| {
        LATEST_CODEC
            .compound_format()
            .write(directory, info, context)?;
        Ok(())
    })();
    let filename = directory
        .get_created_files()
        .lock()
        .created_filenames
        .clone();
    if write_result.is_err() {
        delete_files.accept(&filename)?;
    }
    // Replace all previous files with the CFS/CFE files:
    info.set_files(filename)?;

    write_result
}

struct EventQueue {}
impl EventQueue {
    fn acquire(&self) -> Result<()> {
        todo!()
    }
    fn add<D>(&self, event: EventEnum<D>) -> Result<()>
    where
        D: Directory,
    {
        todo!()
    }
    fn process_events(&self) -> Result<()> {
        todo!()
    }
    fn process_events_internal(&self) -> Result<()> {
        todo!()
    }
    fn close(&self) -> Result<()> {
        todo!()
    }
}

/// Interface for internal atomic events. See [`DocumentsWriter`] for details.
/// Events are executed concurrently and no order is guaranteed. Each event should only rely on
/// the serializability within its `process` method. All actions that must happen before or after
/// a certain action must be encoded inside the [`process(IndexWriter)`](Self::process) method.
trait Event<D>
where
    D: Directory,
{
    /// Processes the event. This method is called by the [`IndexWriter`] passed as the first argument.
    ///
    /// # Arguments
    ///
    /// * `writer` — the [`IndexWriter`] that executes the event.
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase;
}
struct EventImpl1 {
    files: HashSet<String>,
}
impl EventImpl1 {
    pub fn new(files: HashSet<String>) -> Self {
        Self { files }
    }
}
impl<D> Event<D> for EventImpl1
where
    D: Directory,
{
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        writer.delete_new_files(&self.files)
    }
}

struct EventImpl2<D>
where
    D: Directory,
{
    info: SegmentInfo<D>,
}
impl<D> EventImpl2<D>
where
    D: Directory,
{
    pub fn new(info: SegmentInfo<D>) -> Self {
        Self { info }
    }
}
impl<D> Event<D> for EventImpl2<D>
where
    D: Directory,
{
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        writer.flush_failed(&self.info)
    }
}

struct EventImpl3;
impl<D> Event<D> for EventImpl3
where
    D: Directory,
{
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        let result = writer.publish_flushed_segments(true);
        writer.flush_count.fetch_add(1, Ordering::SeqCst);
        result
    }
}
struct EventImpl4;
impl<D> Event<D> for EventImpl4
where
    D: Directory,
{
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        writer.publish_flushed_segments(true)
    }
}
struct EventImpl5 {
    packet: Rc<FrozenBufferedUpdates>,
}
impl EventImpl5 {
    pub fn new(packet: Rc<FrozenBufferedUpdates>) -> Self {
        Self { packet }
    }
}
impl<D> Event<D> for EventImpl5
where
    D: Directory,
{
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
        B: IndexWriterBase,
    {
        // we call tryApply here since we don't want to block if a refresh or a flush is already
        // applying the
        // packet. The flush will retry this packet anyway to ensure all of them are applied
        match writer.try_apply(&self.packet) {
            Ok(_) => {
                writer.flush_deletes_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
            Err(e) => {
                match writer.on_tragic_event(&e, "applyUpdatesPacket") {
                    Ok(_) => Err(e),
                    Err(err) => {
                        // TODO 这里没有将e跟err 合并成一个合理的Error
                        Err(LuceneError::illegal_state(format!(
                            "{err} + supper error:{{e}}"
                        )))
                    },
                }
            },
        }
    }
}

enum EventEnum<D>
where
    D: Directory,
{
    A(EventImpl1),
    B(EventImpl2<D>),
    C(EventImpl3),
    D(EventImpl4),
    E(EventImpl5),
}
impl<D> Event<D> for EventEnum<D>
where
    D: Directory,
{
    fn process<L, B>(&mut self, writer: &IndexWriter<D, L, B>) -> Result<()>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        match self {
            EventEnum::A(e) => e.process(writer),
            EventEnum::B(e) => e.process(writer),
            EventEnum::C(e) => e.process(writer),
            EventEnum::D(e) => e.process(writer),
            EventEnum::E(e) => e.process(writer),
        }
    }
}
