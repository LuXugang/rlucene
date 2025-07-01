/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::index::index_commit::{index_commit_util, IndexCommit};
use crate::index::index_deletion_policy::IndexDeletionPolicy;
use crate::index::index_writer::{index_writer_util, IndexWriter};
use crate::index::segment_infos::{segment_infos_util, SegmentInfos};
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::file_deleter::{FileDeleter, Messenger, MsgType};
use crate::util::info_stream::{InfoStream, InfoStreamLock};
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct IndexFileDeleter<D, P>
where
    D: Directory,
    P: IndexDeletionPolicy,
{
    /// Holds all commits (segments_N) currently in the index.
    /// This will have just 1 commit if you are using the
    /// default delete policy (KeepOnlyLastCommitDeletionPolicy).
    /// Other policies may leave commit points live for longer
    /// in which case this list would be longer than 1.
    commits: Vec<CommitPoint<D>>,

    /// Holds files we had incref'd from the previous non-commit checkpoint.
    last_files: Vec<String>,

    /// Commits that the IndexDeletionPolicy have decided to delete.
    commits_to_delete: Vec<CommitPoint<D>>,

    info_stream: InfoStreamLock,
    directory_orig: Arc<Mutex<D>>,
    directory: Arc<Mutex<D>>,
    policy: P,

    /// Whether the starting commit was deleted.
    starting_commit_deleted: bool,

    last_segment_infos: Option<SegmentInfos<D>>,
    verbose_ref_counts: bool,
    file_deleter: FileDeleter<D, MessengerImpl>,
}
impl<D, P> IndexFileDeleter<D, P>
where
    D: Directory,
    P: IndexDeletionPolicy,
{
    /// Set all gens beyond what we currently see in the directory, to avoid double-write in cases
    /// where the previous `IndexWriter` did not gracefully close/rollback (e.g. OS/machine crashed or
    /// lost power).
    fn inflate_gens(
        infos: &mut SegmentInfos<D>,
        files: impl IntoIterator<Item = String>,
        info_stream: &InfoStreamLock,
    ) -> Result<()> {
        let mut max_segment_gen = i64::MIN;
        let mut max_segment_name = i64::MIN;
        // Confusingly, this is the union of liveDocs, field infos, doc values
        // (and maybe others, in the future) gens.  This is somewhat messy,
        // since it means DV updates will suddenly write to the next gen after
        // live docs' gen, for example, but we don't have the APIs to ask the
        // codec which file is which:
        let mut max_per_segment_gen = HashMap::new();

        for file_name in files {
            if file_name == index_writer_util::WRITE_LOCK_NAME {
                continue;
            } else if file_name.starts_with(IndexFileNames::SEGMENTS) {
                let v = segment_infos_util::generation_from_segments_file_name(&file_name);
                match v {
                    Ok(gen) => {
                        max_segment_gen = max_segment_gen.max(gen);
                    },
                    Err(e) => {
                        // trash file: we have to handle this since we allow anything starting with 'segments'
                        // here
                        if !matches!(e, LuceneError::NumberFormat(_)) {
                            return Err(e);
                        }
                    },
                }
            } else if file_name.starts_with(IndexFileNames::PENDING_SEGMENTS) {
                let v = segment_infos_util::generation_from_segments_file_name(&file_name[8..]);
                match v {
                    Ok(gen) => {
                        max_segment_gen = max_segment_gen.max(gen);
                    },
                    Err(e) => {
                        // trash file: we have to handle this since we allow anything starting with
                        // 'pending_segments' here
                        if !matches!(e, LuceneError::NumberFormat(_)) {
                            return Err(e);
                        }
                    },
                }
            } else {
                let segment_name = IndexFileNames::parse_segment_name(&file_name);
                debug_assert!(segment_name.starts_with('_'), "wtf? file={file_name}");
                if file_name.to_lowercase().ends_with(".tmp") {
                    // A temp file: don't try to look at its gen
                    continue;
                }
                max_segment_name =
                    max_segment_name.max(i64::from_str_radix(&segment_name[1..], 36)?);

                let mut cur_gen = *max_per_segment_gen.get(segment_name).unwrap_or(&0i64);

                let v = IndexFileNames::parse_generation(&file_name);
                match v {
                    Ok(gen) => {
                        cur_gen = cur_gen.max(gen);
                    },
                    Err(e) => {
                        // trash file: we have to handle this since codec regex is only so good
                        if !matches!(e, LuceneError::NumberFormat(_)) {
                            return Err(e);
                        }
                    },
                }
                max_per_segment_gen.insert(segment_name.to_string(), cur_gen);
            }
        }

        // Generation is advanced before write:
        let next_gen = infos.get_generation().max(max_segment_gen);
        infos.set_next_write_generation(next_gen)?;

        let desired = 1 + max_segment_name;
        if infos.counter < desired {
            let mut info_stream = info_stream.lock();
            if info_stream.enabled("IFD") {
                info_stream.message(
                    "IFD",
                    &format!(
                        "init: inflate infos.counter to {} vs current={}",
                        desired, infos.counter
                    ),
                );
            }
            infos.counter = desired;
        }
        for info in infos.iter() {
            debug_assert!(max_per_segment_gen.contains_key(&info.info.name));
            let gen_long = *max_per_segment_gen.get(&info.info.name).unwrap();

            let next_del = info.get_next_write_del_gen();
            if next_del < gen_long + 1 {
                let mut info_stream = info_stream.lock();
                if info_stream.enabled("IFD") {
                    info_stream.message(
                        "IFD",
                        &format!(
                            "init: seg={} set nextWriteDelGen={} vs current={}",
                            info.info.name,
                            gen_long + 1,
                            next_del
                        ),
                    );
                }
                info.set_next_write_del_gen(gen_long + 1);
            }

            let next_fi = info.get_next_write_field_infos_gen();
            if next_fi < gen_long + 1 {
                let mut info_stream = info_stream.lock();
                if info_stream.enabled("IFD") {
                    info_stream.message(
                        "IFD",
                        &format!(
                            "init: seg={} set nextWriteFieldInfosGen={} vs current={}",
                            info.info.name,
                            gen_long + 1,
                            next_fi
                        ),
                    );
                }
                info.set_next_write_field_infos_gen(gen_long + 1);
            }

            let next_dv = info.get_next_write_doc_values_gen();
            if next_dv < gen_long + 1 {
                let mut info_stream = info_stream.lock();
                if info_stream.enabled("IFD") {
                    info_stream.message(
                        "IFD",
                        &format!(
                            "init: seg={} set nextWriteDocValuesGen={} vs current={}",
                            info.info.name,
                            gen_long + 1,
                            next_dv
                        ),
                    );
                }
                info.set_next_write_doc_values_gen(gen_long + 1);
            }
        }
        Ok(())
    }
    fn ensure_open(&self, index_writer: &IndexWriter<D, P>) -> Result<()> {
        index_writer.ensure_open(false)?;

        let tragic_arc = index_writer.get_tragic_exception();
        let tragic = tragic_arc.lock();
        let error = tragic.as_ref();
        if let Some(e) = error {
            return Err(LuceneError::already_closed(format!(
                "refusing to delete any files: this IndexWriter hit an unrecoverable exception: {e}",
            )));
        }

        Ok(())
    }
    pub(crate) fn is_closed(&self, index_writer: &IndexWriter<D, P>) -> Result<bool> {
        match self.ensure_open(index_writer) {
            Ok(_) => Ok(false),
            Err(e) => {
                if matches!(e, LuceneError::AlreadyClosed(_)) {
                    Ok(true)
                } else {
                    Err(e)
                }
            },
        }
    }
}
/// Holds details for each commit point. This struct is also passed to the deletion policy. Note: This struct has a natural ordering that is inconsistent with equals.
pub(crate) struct CommitPoint<D> {
    pub(crate) files: Vec<String>,
    pub(crate) segments_file_name: String,
    pub(crate) deleted: bool,
    pub(crate) directory_orig: Arc<Mutex<D>>,
    pub(crate) generation: i64,
    pub(crate) user_data: HashMap<String, String>,
    pub(crate) segment_count: usize,
}
impl<D> CommitPoint<D>
where
    D: Directory,
{
    pub(crate) fn new(
        directory_orig: Arc<Mutex<D>>,
        segment_infos: &SegmentInfos<D>,
    ) -> Result<Self> {
        // TODO：是不是只要保存segment的ID就行,避免一些拷贝
        let user_data = segment_infos.get_user_data().clone();
        let segments_file_name = segment_infos
            .get_segments_file_name()
            .ok_or_else(|| LuceneError::illegal_state("segment_N file is none"))?;
        let generation = segment_infos.get_generation();
        let files = segment_infos.files(true)?.into_iter().collect();
        let segment_count = segment_infos.size();

        Ok(CommitPoint {
            files,
            segments_file_name,
            deleted: false,
            directory_orig,
            generation,
            user_data,
            segment_count,
        })
    }
}

impl<D> PartialEq for CommitPoint<D>
where
    D: Directory,
{
    fn eq(&self, other: &Self) -> bool {
        index_commit_util::is_same_commit(self, other)
    }
}

impl<D> Eq for CommitPoint<D> where D: Directory {}

impl<D> PartialOrd for CommitPoint<D>
where
    D: Directory,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        index_commit_util::cmp_commit(self, other)
    }
}

impl<D> Ord for CommitPoint<D>
where
    D: Directory,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<D> Display for CommitPoint<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IndexFileDeleter.CommitPoint({})",
            self.segments_file_name
        )
    }
}

impl<D> IndexCommit for CommitPoint<D>
where
    D: Directory,
{
    fn get_segments_file_name(&self) -> &str {
        &self.segments_file_name
    }

    fn get_file_names(&self) -> Result<&[String]> {
        Ok(self.files.as_slice())
    }

    type Directory = D;

    fn get_directory(&self) -> Arc<Mutex<Self::Directory>> {
        self.directory_orig.clone()
    }

    fn delete(&mut self) -> Result<()> {
        todo!()
    }

    fn is_deleted(&self) -> bool {
        self.deleted
    }

    fn get_segment_count(&self) -> usize {
        self.segment_count
    }

    fn get_generation(&self) -> i64 {
        self.generation
    }

    fn user_data(&self) -> &HashMap<String, String> {
        &self.user_data
    }
}

pub(crate) struct MessengerImpl {
    info_stream: InfoStreamLock,
    verbose_ref_counts: bool,
}
impl MessengerImpl {
    pub(crate) fn new(info_stream: InfoStreamLock, verbose_ref_counts: bool) -> Self {
        MessengerImpl {
            info_stream,
            verbose_ref_counts,
        }
    }
}
impl Messenger for MessengerImpl {
    fn accept(&self, msg_type: MsgType, msg: &String) {
        if msg_type == MsgType::Ref && !self.verbose_ref_counts {
            return;
        }
        let mut info_stream = self.info_stream.lock();
        if info_stream.enabled("IFD") {
            info_stream.message("IFD", msg);
        }
    }
}
