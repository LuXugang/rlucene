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
use crate::codecs::codec::Codec;
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::codecs::LATEST_CODEC;
use crate::index::segment_info::SegmentInfo;

use crate::store::directory::Directory;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::StringHelper;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI64, Ordering};

pub struct SegmentCommitInfo<D>
where
    D: Directory,
{
    /// The SegmentInfo that we wrap.
    pub info: SegmentInfo<D>,

    /// Id that uniquely identifies this segment commit.
    pub id: Option<Vec<u8>>,

    /// How many deleted docs in the segment.
    pub del_count: i32,

    /// How many soft-deleted docs in the segment that are not also hard-deleted.
    pub soft_del_count: i32,

    /// Generation number of the live docs file (-1 if there are no deletes yet).
    pub del_gen: i64,

    /// Normally 1 + del_gen, unless an exception was hit on the last attempt to write.
    pub next_write_del_gen: i64,

    /// Generation number of the FieldInfos (-1 if there are no updates).
    pub field_infos_gen: i64,

    /// Normally 1 + field_infos_gen, unless an exception was hit on the last attempt to write.
    pub next_write_field_infos_gen: i64,

    /// Generation number of the DocValues (-1 if there are no updates).
    pub doc_values_gen: i64,

    /// Normally 1 + doc_values_gen, unless an exception was hit on the last attempt to write.
    pub next_write_doc_values_gen: i64,

    /// Track the per-field DocValues update files.
    pub dv_updates_files: HashMap<i32, HashSet<String>>,

    /// Track the FieldInfos update files.
    pub field_infos_files: HashSet<String>,

    /// Size of the segment in bytes (-1 of unknown).
    pub size_in_bytes: AtomicI64,

    /// Used in memory by IndexWriter to track buffered deletes. Not persisted to disk.
    pub buffered_deletes_gen: i64,
}
impl<D> SegmentCommitInfo<D>
where
    D: Directory,
{
    /// Sole constructor.
    ///
    /// # Arguments
    /// - `info`: The `SegmentInfo` that this wraps.
    /// - `del_count`: Number of deleted documents in this segment.
    /// - `Soft_del_count`: Number of soft-deleted documents not also hard-deleted.
    /// - `del_gen`: Deletion generation number (used to name deletion files).
    /// - `field_infos_gen`: FieldInfos generation number (used to name field-infos files).
    /// - `Doc_values_gen`: DocValues generation number (used to name doc-values updates files).
    /// - `ID`: ID that uniquely identifies this segment commit.
    pub fn new(
        info: SegmentInfo<D>,
        del_count: i32,
        soft_del_count: i32,
        del_gen: i64,
        field_infos_gen: i64,
        doc_values_gen: i64,
        id: Option<Vec<u8>>,
    ) -> Result<Self> {
        // Validate the ID length
        if id.is_some() && id.as_ref().unwrap().len() != StringHelper::ID_LENGTH as usize {
            return Err(LuceneError::illegal_argument(format!(
                "Invalid ID: {:?}",
                id.unwrap()
            )));
        }

        Ok(Self {
            info,
            del_count,
            soft_del_count,
            del_gen,
            next_write_del_gen: if del_gen == -1 { 1 } else { del_gen + 1 },
            field_infos_gen,
            next_write_field_infos_gen: if field_infos_gen == -1 {
                1
            } else {
                field_infos_gen + 1
            },
            doc_values_gen,
            next_write_doc_values_gen: if doc_values_gen == -1 {
                1
            } else {
                doc_values_gen + 1
            },
            id,
            dv_updates_files: HashMap::new(),
            field_infos_files: HashSet::new(),
            size_in_bytes: AtomicI64::new(-1),
            buffered_deletes_gen: -1,
        })
    }
    /// Returns a reference to the per-field DocValues updates files.
    pub fn get_doc_values_updates_files(&self) -> &HashMap<i32, HashSet<String>> {
        &self.dv_updates_files
    }

    /// Sets the DocValues updates file names, per field number. Does not deeply clone the map.
    pub fn set_doc_values_updates_files(
        &mut self,
        dv_updates_files: HashMap<i32, HashSet<String>>,
    ) {
        self.dv_updates_files.clear();
        for (key, file_set) in dv_updates_files {
            let renamed_set: HashSet<String> = file_set
                .into_iter()
                .map(|file| self.info.named_for_this_segment(file))
                .collect();
            self.dv_updates_files.insert(key, renamed_set);
        }
    }
    /// Returns a reference to the FieldInfos file names.
    pub fn get_field_infos_files(&self) -> &HashSet<String> {
        &self.field_infos_files
    }

    /// Sets the FieldInfos file names.
    pub fn set_field_infos_files(&mut self, field_infos_files: HashSet<String>) {
        self.field_infos_files.clear();
        for file in field_infos_files {
            self.field_infos_files
                .insert(self.info.named_for_this_segment(file));
        }
    }

    /// Called when we succeed in writing deletes.
    pub fn advance_del_gen(&mut self) {
        self.del_gen = self.next_write_del_gen;
        self.next_write_del_gen = self.del_gen + 1;
        self.generation_advanced();
    }

    /// Called if there was an exception while writing deletes, so that we don't try to write to the
    /// same file more than once.
    pub fn advance_next_write_del_gen(&mut self) {
        self.next_write_del_gen += 1;
    }

    /// Gets the `nextWriteDelGen`.
    pub fn get_next_write_del_gen(&self) -> i64 {
        self.next_write_del_gen
    }

    /// Sets the `nextWriteDelGen`.
    pub fn set_next_write_del_gen(&mut self, v: i64) {
        self.next_write_del_gen = v;
    }

    /// Called when we succeed in writing a new FieldInfos generation.
    pub fn advance_field_infos_gen(&mut self) {
        self.field_infos_gen = self.next_write_field_infos_gen;
        self.next_write_field_infos_gen = self.field_infos_gen + 1;
        self.generation_advanced();
    }

    /// Called if there was an exception while writing a new generation of FieldInfos,
    /// so that we don't try to write to the same file more than once.
    pub fn advance_next_write_field_infos_gen(&mut self) {
        self.next_write_field_infos_gen += 1;
    }

    /// Gets the `nextWriteFieldInfosGen`.
    pub fn get_next_write_field_infos_gen(&self) -> i64 {
        self.next_write_field_infos_gen
    }

    /// Sets the `nextWriteFieldInfosGen`.
    pub fn set_next_write_field_infos_gen(&mut self, v: i64) {
        self.next_write_field_infos_gen = v;
    }

    /// Called when we succeed in writing a new DocValues generation.
    pub fn advance_doc_values_gen(&mut self) {
        self.doc_values_gen = self.next_write_doc_values_gen;
        self.next_write_doc_values_gen = self.doc_values_gen + 1;
        self.generation_advanced();
    }

    /// Called if there was an exception while writing a new generation of DocValues,
    /// so that we don't try to write to the same file more than once.
    pub fn advance_next_write_doc_values_gen(&mut self) {
        self.next_write_doc_values_gen += 1;
    }

    /// Gets the `nextWriteDocValuesGen`.
    pub fn get_next_write_doc_values_gen(&self) -> i64 {
        self.next_write_doc_values_gen
    }

    /// Sets the `nextWriteDocValuesGen`.
    pub fn set_next_write_doc_values_gen(&mut self, v: i64) {
        self.next_write_doc_values_gen = v;
    }

    /// Returns the total size in bytes of all files for this segment.
    pub fn size_in_bytes(&self) -> Result<i64> {
        let current_size = self.size_in_bytes.load(Ordering::SeqCst);
        if current_size != -1 {
            return Ok(current_size);
        }
        let mut sum = 0;
        let directory = self
            .info
            .dir
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire  lock.".to_string()))?;
        for file_name in self.files()? {
            sum += directory.file_length(&file_name)?;
        }
        self.size_in_bytes.store(sum, Ordering::SeqCst);

        Ok(sum)
    }
    /// Returns all files in use by this segment.
    pub fn files(&self) -> Result<HashSet<String>> {
        // Start from the wrapped info's files (deep copy):
        let mut files: HashSet<String> = self.info.files()?.clone();
        // TODO we could rely on TrackingDir.getCreatedFiles() (like we do for
        // updates) and then maybe even be able to remove LiveDocsFormat.files().
        // Must separately add any live docs files:
        if self.has_deletions() {
            // debug_assert!(self.info.codec.is_some());
            LATEST_CODEC.live_docs_format().files(self, &mut files)?;
        }
        for update_files in self.dv_updates_files.values() {
            files.extend(update_files.clone());
        }
        // must separately add fieldInfos files
        files.extend(self.field_infos_files.clone());
        Ok(files)
    }

    /// Returns the buffered deletes generation number.
    pub fn get_buffered_deletes_gen(&self) -> i64 {
        self.buffered_deletes_gen
    }

    /// Sets the buffered deletes generation number.
    /// Can only be set once, otherwise it will throw an error.
    pub fn set_buffered_deletes_gen(&mut self, v: i64) -> Result<()> {
        if self.buffered_deletes_gen == -1 {
            self.buffered_deletes_gen = v;
            self.generation_advanced();
            Ok(())
        } else {
            Err(LuceneError::illegal_state(
                "buffered deletes gen should only be set once".to_string(),
            ))
        }
    }
    /// Determines if this segment has deletions.
    pub fn has_deletions(&self) -> bool {
        self.del_gen != -1
    }
    /// Returns `true` if there are any field updates for the segment in this commit.
    pub fn has_field_updates(&self) -> bool {
        self.field_infos_gen != -1
    }

    /// Returns the next available generation number of the FieldInfos files.
    pub fn get_next_field_infos_gen(&self) -> i64 {
        self.next_write_field_infos_gen
    }

    /// Returns the generation number of the field infos file, or `-1` if there are no field updates yet.
    pub fn get_field_infos_gen(&self) -> i64 {
        self.field_infos_gen
    }

    /// Returns the next available generation number of the DocValues files.
    pub fn get_next_doc_values_gen(&self) -> i64 {
        self.next_write_doc_values_gen
    }

    /// Returns the generation number of the DocValues file or `-1` if there are no doc-values updates yet.
    pub fn get_doc_values_gen(&self) -> i64 {
        self.doc_values_gen
    }

    /// Returns the next available generation number of the live docs file.
    pub fn get_next_del_gen(&self) -> i64 {
        self.next_write_del_gen
    }

    /// Returns the generation number of the live docs file or `-1` if there are no deletes yet.
    pub fn get_del_gen(&self) -> i64 {
        self.del_gen
    }

    /// Returns the number of deleted docs in the segment.
    pub fn get_del_count(&self) -> i32 {
        self.del_count
    }

    /// Returns the number of only soft-deleted docs.
    pub fn get_soft_del_count(&self) -> i32 {
        self.soft_del_count
    }

    pub fn set_del_count(&mut self, del_count: i32) -> Result<()> {
        let max_doc = self.info.max_doc()?;
        if del_count < 0 || del_count > max_doc {
            return Err(LuceneError::illegal_argument(format!(
                "invalid delCount={} (maxDoc={})",
                del_count, max_doc
            )));
        }

        debug_assert!(
            self.soft_del_count + del_count <= max_doc,
            "maxDoc={}, delCount={}, softDelCount={}",
            max_doc,
            del_count,
            self.soft_del_count
        );

        self.del_count = del_count;
        Ok(())
    }

    pub fn set_soft_del_count(&mut self, soft_del_count: i32) -> Result<()> {
        let max_doc = self.info.max_doc()?;
        if soft_del_count < 0 || soft_del_count > max_doc {
            return Err(LuceneError::illegal_argument(format!(
                "invalid softDelCount={} (maxDoc={})",
                soft_del_count, max_doc
            )));
        }

        debug_assert!(
            self.del_count + soft_del_count <= max_doc,
            "maxDoc={}, delCount={}, softDelCount={}",
            max_doc,
            self.del_count,
            soft_del_count
        );
        self.soft_del_count = soft_del_count;
        Ok(())
    }
    /// Returns a description of this segment.
    pub fn to_string_with_pending_del_count(&self, pending_del_count: i32) -> Result<String> {
        let mut s = SegmentInfo::to_string(&self.info, self.del_count + pending_del_count)?;

        if self.del_gen != -1 {
            s.push_str(&format!(":delGen={}", self.del_gen));
        }
        if self.field_infos_gen != -1 {
            s.push_str(&format!(":fieldInfosGen={}", self.field_infos_gen));
        }
        if self.doc_values_gen != -1 {
            s.push_str(&format!(":dvGen={}", self.doc_values_gen));
        }
        if self.soft_del_count > 0 {
            s.push_str(&format!(" :softDel={}", self.soft_del_count));
        }
        if self.id.is_some() {
            s.push_str(&format!(
                " :id={}",
                StringHelper::id_to_string(Option::from(self.id.as_ref().unwrap().as_slice()))
            ));
        }
        Ok(s)
    }
    /// Returns the number of deleted documents in the segment.
    /// If `include_soft_deletes` is `true`, it includes soft-deleted documents.
    pub fn get_del_count_with_soft_deletes(&self, include_soft_deletes: bool) -> i32 {
        if include_soft_deletes {
            self.get_del_count() + self.get_soft_del_count()
        } else {
            self.get_del_count()
        }
    }

    /// Advances the generation, resetting `size_in_bytes` and generating a new `id`.
    pub fn generation_advanced(&mut self) {
        self.size_in_bytes.store(-1, Ordering::SeqCst);
        self.id = Option::from(Vec::from(StringHelper::random_id()));
    }

    /// Returns an ID that uniquely identifies this segment commit.
    /// If no ID is assigned, returns `None`.
    pub fn get_id(&self) -> Option<Vec<u8>> {
        if self.id.is_none() {
            None
        } else {
            Some(self.id.clone()?)
        }
    }
}

/// Implement `Display` for `SegmentCommitInfo`.
impl<D> std::fmt::Display for SegmentCommitInfo<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = self.to_string_with_pending_del_count(0);
        match result {
            Ok(s) => write!(f, "{}", s),
            Err(e) => write!(f, "fmt Error {}", e),
        }
    }
}
impl<D> Clone for SegmentCommitInfo<D>
where
    D: Directory,
{
    fn clone(&self) -> Self {
        let mut cloned_dv_updates_files = HashMap::new();

        for (key, value) in &self.dv_updates_files {
            cloned_dv_updates_files.insert(*key, value.clone());
        }

        let id = self.get_id();
        // Create the cloned instance
        SegmentCommitInfo {
            info: self.info.clone(),
            id,
            del_count: self.del_count,
            soft_del_count: self.soft_del_count,
            del_gen: self.del_gen,
            next_write_del_gen: self.next_write_del_gen,
            field_infos_gen: self.field_infos_gen,
            next_write_field_infos_gen: self.next_write_field_infos_gen,
            doc_values_gen: self.doc_values_gen,
            next_write_doc_values_gen: self.next_write_doc_values_gen,
            dv_updates_files: cloned_dv_updates_files,
            field_infos_files: self.field_infos_files.clone(),
            size_in_bytes: AtomicI64::new(self.size_in_bytes.load(Ordering::SeqCst)),
            buffered_deletes_gen: self.buffered_deletes_gen,
        }
    }
}
impl<D> PartialEq for SegmentCommitInfo<D>
where
    D: PartialEq + Directory,
{
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
