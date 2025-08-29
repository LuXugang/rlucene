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

pub struct IndexWriter<D, L>
where
    D: Directory,
    L: LiveIndexWriterConfig,
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
    segment_infos: SegmentInfos<D>,
    global_field_number_map: Arc<FieldNumbers>,
    doc_writer: DocumentsWriter<D, L>,
    write_doc_values_lock: ReentrantMutex<()>,
    deleter: IndexFileDeleter<D, L::IndexDeletionPolicy>,
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
}

impl<D, L> IndexWriter<D, L>
where
    D: Directory,
    L: LiveIndexWriterConfig,
{
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

    pub fn get_tragic_exception(&self) -> TragicException {
        self.tragedy.clone()
    }
    pub(crate) fn is_deleter_closed(&self) -> Result<bool> {
        self.deleter.is_closed(self)
    }
    pub(crate) fn try_apply(&mut self, _updates: &mut FrozenBufferedUpdates) -> Result<bool> {
        todo!()
    }
    pub(crate) fn force_apply(&mut self, _updates: &mut FrozenBufferedUpdates) -> Result<bool> {
        todo!()
    }
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

    fn flush_failed<D>(&self, _info: &SegmentInfo<D>)
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
use crate::index::field_infos::FieldNumbers;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::index::reader_pool::ReaderPool;
use crate::index::segment_commit_info::SegmentCommitInfo;
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
