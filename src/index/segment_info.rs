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
use crate::index::sort::Sort;
use crate::store::codec::Codec;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::LuceneError;
use crate::util::version::Version;
use crate::util::StringHelper;
use std::collections::{HashMap, HashSet};
use std::task::Poll;

/// Used by some member fields to mean not present (e.g., norms, deletions).
pub const NO: i32 = -1; // e.g. no norms; no deletes;

/// Used by some member fields to mean present (e.g., norms, deletions).
pub const YES: i32 = 1; // e.g. have norms; have deletes;
/// Information about a segment such as its name, directory, and files related to the segment.
///
/// # Experimental
/// This API is experimental and may change in future releases.
pub struct SegmentInfo<'a, D>
where
    D: Directory,
{
    /// Unique segment name in the directory.
    pub name: String,
    max_doc: Option<u32>, // number of docs in seg
    /// Where this segment resides.
    pub dir: &'a mut D,
    is_compound_file: bool,
    /// Id that uniquely identifies this segment.
    id: Vec<u8>,
    codec: Option<Codec>,
    diagnostics: HashMap<String, String>,
    attributes: HashMap<String, String>,
    index_sort: Option<Sort>,
    /// Tracks the Lucene version this segment was created with, since 3.1.
    /// Null indicates an older than 3.0 index, and it's used to detect a too-old index.
    /// The format expected is "x.y" - "2.x" for pre-3.0 indexes (or null), and
    /// specific versions afterwards ("3.0.0", "3.1.0" etc.).
    /// See `Version` for details.
    version: Version,
    /// Tracks the minimum version that contributed documents to a segment.
    /// For flush segments, that is the version that wrote it.
    /// For merged segments, this is the minimum `min_version` of all the segments that have been merged
    /// into this segment.
    min_version: Option<Version>,
    has_blocks: bool,
    set_files: Option<HashSet<String>>,
}

impl<'a, D> SegmentInfo<'a, D>
where
    D: Directory,
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
        dir: &'a mut D,
        version: Version,
        min_version: Option<Version>,
        name: String,
        max_doc: Option<u32>,
        is_compound_file: bool,
        has_blocks: bool,
        codec: Option<Codec>,
        diagnostics: HashMap<String, String>,
        id: Vec<u8>,
        attributes: HashMap<String, String>,
        index_sort: Option<Sort>,
    ) -> Result<SegmentInfo<D>, LuceneError> {
        // debug_assert!(
        //     !dir.is::<TrackingDirectoryWrapper>(),
        //     "dir should not be a TrackingDirectoryWrapper"
        // );
        if id.len() != StringHelper::ID_LENGTH as usize {
            panic!("invalid id: {:?}", id);
        }
        Ok(SegmentInfo {
            dir,
            version,
            min_version,
            name,
            max_doc,
            is_compound_file,
            has_blocks,
            codec,
            diagnostics,
            id,
            attributes,
            index_sort,
            set_files: None,
        })
    }
}
impl<'a, D> SegmentInfo<'a, D>
where
    D: Directory,
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
    pub fn get_diagnostics(&self) -> HashMap<String, String> {
        self.diagnostics.clone()
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
    /// Can only be called once to set the codec
    pub fn set_codec(&mut self, codec: Codec) -> Result<(), LuceneError> {
        if self.codec.is_some() {
            return Err(LuceneError::illegal_argument(
                "Codec was already set".to_string(),
            ));
        }
        self.codec = Some(codec);
        Ok(())
    }

    /// Returns the Codec that wrote this segment
    pub fn get_codec(&self) -> &Option<Codec> {
        &self.codec
    }

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
}
