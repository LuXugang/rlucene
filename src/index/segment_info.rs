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
use crate::codecs::lucene101_codec::Lucene101Codec;
use crate::index::sort::Sort;
use crate::index::{IndexFileNames, CODEC_FILE_PATTERN};
use crate::search::field_comparator_source::{DummyFieldComparatorSource, FieldComparatorSource};
use crate::search::sort_field::{DummySortFieldBase, SortFieldBase};
use crate::store::directory::Directory;
use crate::store::dummy::dummy_directory::DummyDirectory;
use crate::util::error::lucene_error::LuceneError;
use crate::util::version::Version;
use crate::util::StringHelper;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Information about a segment such as its name, directory, and files related to the segment.
///
/// # Experimental
/// This API is experimental and may change in future releases.
pub struct SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    /// Unique segment name in the directory.
    pub name: String,
    max_doc: Option<u32>, // number of docs in seg
    /// Where this segment resides.
    pub dir: Arc<Mutex<D>>,
    is_compound_file: bool,
    /// Id that uniquely identifies this segment.
    id: Vec<u8>,
    // Diff to Java Lucene: We need to ensure that there is only one Codec in the index
    // Therefore, we do not need to explicitly define the Codec in the SegmentInfo.
    // pub(crate) codec: Option<Lucene101Codec>,
    diagnostics: HashMap<String, String>,
    attributes: Arc<Mutex<HashMap<String, String>>>,
    index_sort: Option<Sort<F, S>>,
    /// Tracks the Lucene version this segment was created with, since 3.1.
    /// Null indicates an older than 3.0 index, and it's used to detect a too-old index.
    /// The format expected is "x.y" - "2.x" for pre-3.0 indexes (or null), and
    /// specific versions afterwards ("3.0.0", "3.1.0" etc.).
    /// See `Version` for details.
    pub(crate) version: Option<Version>,
    /// Tracks the minimum version that contributed documents to a segment.
    /// For flush segments, that is the version that wrote it.
    /// For merged segments, this is the minimum `min_version` of all the segments that have been merged
    /// into this segment.
    pub(crate) min_version: Option<Version>,
    has_blocks: bool,
    set_files: Option<HashSet<String>>,
}

impl SegmentInfo<DummyDirectory, DummySortFieldBase, DummyFieldComparatorSource> {
    /// Used by some member fields to mean not present (e.g., norms, deletions).
    pub const NO: i32 = -1; // e.g. no norms; no deletes;
    /// Used by some member fields to mean present (e.g., norms, deletions).
    pub const YES: i32 = 1; // e.g. have norms; have deletes;
}

impl<D, S, F> SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    /// Constructs a new complete `SegmentInfo` instance from input.
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory where this segment resides.
    /// * `version` - The Lucene version this segment was created with.
    /// * `min_version` - The minimum version that contributed documents to this segment.
    /// * `name` - Unique segment name.
    /// * `max_doc` - Number of documents in the segment.
    /// * `is_compound_file` - Indicates if this segment uses a compound file format.
    /// * `has_blocks` - Indicates if the segment has blocks.
    /// * `codec` - The codec used to encode/decode this segment.
    /// * `diagnostics` - Diagnostic information related to the segment.
    /// * `id` - Unique identifier for this segment.
    /// * `attributes` - Additional attributes for the segment.
    /// * `index_sort` - The sort order of the index, if any.
    ///
    /// # Panics
    ///
    /// This method panics if:
    /// * `id` length does not match the expected `ID_LENGTH`.
    /// * `dir` is a `TrackingDirectoryWrapper`.
    pub fn new(
        dir: Arc<Mutex<D>>,
        version: Option<Version>,
        min_version: Option<Version>,
        name: String,
        max_doc: Option<u32>,
        is_compound_file: bool,
        has_blocks: bool,
        diagnostics: HashMap<String, String>,
        //TODO: type should be [u8,16],avoid heap allocation?
        id: Vec<u8>,
        attributes: HashMap<String, String>,
        index_sort: Option<Sort<F, S>>,
    ) -> Result<SegmentInfo<D, S, F>, LuceneError> {
        // debug_assert!(
        //     !dir.is::<TrackingDirectoryWrapper>(),
        //     "dir should not be a TrackingDirectoryWrapper"
        // );
        if id.len() != StringHelper::ID_LENGTH as usize {
            return Err(LuceneError::illegal_argument(format!(
                "Invalid id: {:?}",
                id
            )));
        }
        Ok(SegmentInfo {
            dir,
            version,
            min_version,
            name,
            max_doc,
            is_compound_file,
            has_blocks,
            diagnostics,
            id,
            attributes: Arc::new(Mutex::new(attributes)),
            index_sort,
            set_files: None,
        })
    }
}
impl<D, S, F> SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    /// Sets the diagnostics map. The given map is cloned to ensure immutability.
    pub fn set_diagnostics(&mut self, diagnostics: HashMap<String, String>) {
        self.diagnostics = diagnostics;
    }

    /// Adds or modifies this segment's diagnostics.
    ///
    /// Entries in the given map whose keys are not present in the current diagnostics are added.
    /// Otherwise, existing entries are modified with the given map's value.
    ///
    /// # Arguments
    /// * `diagnostics` - The additional diagnostics to be added or modified.
    pub fn add_diagnostics(&mut self, diagnostics: HashMap<String, String>) {
        let mut copy = self.diagnostics.clone();
        copy.extend(diagnostics);
        self.set_diagnostics(copy);
    }

    /// Returns diagnostics saved into the segment when it was written.
    /// The map is immutable.
    pub fn get_diagnostics(&self) -> &HashMap<String, String> {
        &self.diagnostics
    }
    /// Marks whether this segment is stored as a compound file.
    ///
    /// # Arguments
    ///
    /// * `is_compound_file` - `true` if this is a compound file; otherwise, `false`.
    pub fn set_use_compound_file(&mut self, is_compound_file: bool) {
        self.is_compound_file = is_compound_file;
    }
    /// Returns `true` if this segment is stored as a compound file; otherwise, `false`.
    pub fn get_use_compound_file(&self) -> bool {
        self.is_compound_file
    }

    /// Returns `true` if this segment contains documents written as blocks.
    ///
    /// # See Also
    /// [`LeafMetaData::has_blocks`](crate::index::leaf_metadata::LeafMetaData::get_has_blocks)
    pub fn get_has_blocks(&self) -> bool {
        self.has_blocks
    }

    /// Sets the `has_blocks` property to `true`. This setting is viral and can't be unset.
    pub fn set_has_blocks(&mut self) {
        self.has_blocks = true;
    }
    // /// Can only be called once to set the codec
    // pub fn set_codec(&mut self, codec: Lucene101Codec) -> Result<(), LuceneError> {
    //     if self.codec.is_some() {
    //         return Err(LuceneError::illegal_argument(
    //             "Codec was already set".to_string(),
    //         ));
    //     }
    //     self.codec = Some(codec);
    //     Ok(())
    // }
    //
    // /// Returns the Codec that wrote this segment
    // pub fn get_codec(&self) -> &Option<Lucene101Codec> {
    //     &self.codec
    // }

    /// Returns the number of documents in this segment (deletions are not taken into account)
    pub fn max_doc(&self) -> Result<u32, LuceneError> {
        if self.max_doc.is_none() {
            return Err(LuceneError::illegal_argument(
                "maxDoc isn't set yet".to_string(),
            ));
        }
        Ok(self.max_doc.unwrap())
    }

    /// Sets the max_doc value, can only be called once
    pub fn set_max_doc(&mut self, max_doc: u32) -> Result<(), LuceneError> {
        if self.max_doc.is_some() {
            return Err(LuceneError::illegal_argument(format!(
                "maxDoc was already set: this.maxDoc={} vs maxDoc {}",
                self.max_doc.unwrap(),
                max_doc
            )));
        }
        self.max_doc = Some(max_doc);
        Ok(())
    }

    /// Returns all files referenced by this SegmentInfo
    pub fn files(&self) -> Result<&HashSet<String>, LuceneError> {
        if self.set_files.is_none() {
            debug_assert!(self.max_doc.is_some());
            return Err(LuceneError::illegal_argument(format!(
                "files were not computed yet; segment={} maxDoc={}",
                self.name,
                self.max_doc.unwrap()
            )));
        }
        Ok(self.set_files.as_ref().unwrap())
    }

    /// Sets the files for this segment
    pub fn set_files(&mut self, files: HashSet<String>) {
        self.set_files = Some(files);
    }
    /// Converts this segment information into a formatted string with deletions count.
    ///
    /// # Arguments
    ///
    /// * `del_count` - Number of deletions in the segment.
    ///
    /// # Format
    ///
    /// `_a(3.1):c45/4:[sorter=<long: "timestamp">!]`
    ///
    /// - `_a`: The segment's name.
    /// - `(3.1)`: Lucene version used to create the segment (`?` if unknown).
    /// - `c`: Indicates the compound file format (`C` if not compound).
    /// - `45`: Number of documents in the segment.
    /// - `/4`: Number of deletions (only present if deletions exist).
    /// - `[sorter=<long: "timestamp">!]`: Indicates the segment is sorted by the `timestamp` field in descending order (optional, omitted for unsorted segments).
    pub fn to_string(&self, del_count: i32) -> Result<String, LuceneError> {
        let mut s = String::new();
        s.push_str(&self.name);

        // 处理 version 字段
        s.push('(');
        s.push_str(
            &self
                .version
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        s.push(')');

        let cfs = if self.is_compound_file { 'c' } else { 'C' };
        s.push(':');
        s.push(cfs);

        s.push_str(&self.max_doc.as_ref().unwrap().to_string());

        if del_count != 0 {
            s.push('/');
            s.push_str(&del_count.to_string());
        }

        if let Some(index_sort) = &self.index_sort {
            s.push_str(":[indexSort=");
            s.push_str(&index_sort.to_string());
            s.push(']');
        }

        if !self.diagnostics.is_empty() {
            s.push_str(":[diagnostics=");
            s.push_str(&format!("{:?}", self.diagnostics));
            s.push(']');
        }

        let attributes = self.attributes.lock().map_err(|_| {
            LuceneError::illegal_state("Failed to acquire lock on attributes.".to_string())
        })?;
        if !attributes.is_empty() {
            s.push_str(":[attributes=");
            s.push_str(&format!("{:?}", *attributes));
            s.push(']');
        }
        Ok(s)
    }
    /// Returns the version of the code which wrote the segment.
    pub fn get_version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    /// Returns the minimum Lucene version that contributed documents to this segment, or `None`
    /// if it is unknown.
    pub fn get_min_version(&self) -> Option<&Version> {
        self.min_version.as_ref()
    }

    /// Returns the id that uniquely identifies this segment.
    pub fn get_id(&self) -> Vec<u8> {
        self.id.clone()
    }

    /// Add these files to the set of files written for this segment.
    pub fn add_files(&mut self, files: HashSet<String>) -> Result<(), LuceneError> {
        self.check_file_names(&files)?;
        debug_assert!(self.set_files.is_some());
        let transformed_files: HashSet<String> = files
            .into_iter()
            .map(|file| self.named_for_this_segment(file))
            .collect();
        if let Some(set_files) = &mut self.set_files {
            for file in transformed_files {
                set_files.insert(file);
            }
        }

        Ok(())
    }

    /// Add this file to the set of files written for this segment.
    pub fn add_file(&mut self, file: String) -> Result<(), LuceneError> {
        self.add_files(HashSet::from([file]))
    }

    fn check_file_names(&self, files: &HashSet<String>) -> Result<(), LuceneError> {
        for file in files {
            // Check if the file name matches the codec file pattern
            if !CODEC_FILE_PATTERN.is_match(file) {
                return Err(LuceneError::illegal_argument(format!(
                    "Invalid codec filename '{}', must match: {}",
                    file,
                    CODEC_FILE_PATTERN.as_str()
                )));
            }

            if file.to_lowercase().ends_with(".tmp") {
                return Err(LuceneError::illegal_argument(format!(
                    "Invalid codec filename '{}', cannot end with .tmp extension",
                    file
                )));
            }
        }

        Ok(())
    }
    /// Strips any segment name from the file and renames it with this segment.
    /// This is because "segment names" can change, e.g., by addIndexes(Dir).
    pub fn named_for_this_segment(&self, file: String) -> String {
        format!("{}{}", self.name, IndexFileNames::strip_segment_name(&file))
    }
    /// Get a codec attribute value, or None if it does not exist.
    pub fn get_attribute(&self, key: &str) -> Result<Option<String>, LuceneError> {
        let attributes = self.attributes.lock().map_err(|_| {
            LuceneError::illegal_state("Failed to acquire lock on attributes.".to_string())
        })?;
        Ok(attributes.get(key).cloned())
    }
    /// Puts a codec attribute value.
    ///
    /// This is a key-value mapping for the field that the codec can use to store additional
    /// metadata, and will be available to the codec when reading the segment via `get_attribute`.
    ///
    /// If a value already exists for the field, it will be replaced with the new value. This method
    /// ensures thread safety by making a copy-on-write for every attribute change.
    pub fn put_attribute(&self, key: String, value: String) -> Result<Option<String>, LuceneError> {
        // This needs to be thread-safe because multiple threads may be updating (different) attributes
        // at the same time due to concurrent merging, plus some threads may be calling toString() on
        // segment info while other threads are updating attributes.
        let mut attributes = self.attributes.lock().map_err(|_| {
            LuceneError::illegal_state("Failed to acquire lock on attributes.".to_string())
        })?;
        Ok(attributes.insert(key, value))
    }
    /// Returns the internal codec attributes map.
    pub fn get_attributes(&self) -> Result<Arc<Mutex<HashMap<String, String>>>, LuceneError> {
        Ok(self.attributes.clone())
    }

    /// Returns the sort order of this segment, or None if the index has no sort.
    pub fn get_index_sort(&self) -> Option<&Sort<F, S>> {
        self.index_sort.as_ref()
    }
}
impl<D, S, F> Display for SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = self.to_string(0);
        match result {
            Ok(s) => write!(f, "{}", s),
            Err(e) => write!(f, "fmt Error: {}", e),
        }
    }
}
impl<D, S, F> PartialEq for SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.dir, &other.dir) && self.name == other.name
    }
}

impl<D, S, F> Eq for SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
}
impl<D, S, F> Hash for SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        let dir_address = Arc::as_ptr(&self.dir) as usize;
        state.write_usize(dir_address);
        self.name.hash(state);
    }
}
impl<D, S, F> Clone for SegmentInfo<D, S, F>
where
    D: Directory,
    S: SortFieldBase,
    F: FieldComparatorSource,
{
    fn clone(&self) -> Self {
        SegmentInfo {
            name: self.name.clone(),
            max_doc: self.max_doc,
            dir: self.dir.clone(),
            is_compound_file: self.is_compound_file,
            id: self.id.clone(),
            diagnostics: self.diagnostics.clone(),
            attributes: self.attributes.clone(),
            index_sort: self.index_sort.clone(),
            version: self.version.clone(),
            min_version: self.min_version.clone(),
            has_blocks: self.has_blocks,
            set_files: self.set_files.clone(),
        }
    }
}
