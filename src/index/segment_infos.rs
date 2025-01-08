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
use crate::codecs::segment_info_format::SegmentInfoFormat;
use crate::codecs::{Codec, CodecUtil};
use crate::index::index_writer::IndexWriter;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::IndexFileNames;
use crate::store::check_sum_index_input::ChecksumIndexInput;
use crate::store::directory::Directory;
use crate::store::{DataInput, IOContext, IndexOutput};
use crate::util::error::lucene_error::LuceneError;
use crate::util::output_enum::OutputEnum;
use crate::util::{IOUtils, StringHelper, Version, LATEST, MIN_SUPPORTED_MAJOR};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::{fmt, io};

lazy_static! {
    static ref INFO_STREAM: Mutex<Option<Arc<Mutex<OutputEnum>>>> = Mutex::new(None);
}
/// The version at the time when 8.0 was released.
pub const VERSION_74: u32 = 9;
/// The version that recorded SegmentCommitInfo IDs.
pub const VERSION_86: u32 = 10;
/// Current version of SegmentInfos.
pub const VERSION_CURRENT: u32 = VERSION_86;
/// Name of the generation reference file name.
pub const OLD_SEGMENTS_GEN: &str = "segments.gen";
/// A collection of `SegmentInfo` objects with methods for operating on those segments
/// in relation to the file system.
///
/// The active segments in the index are stored in the segment info file, `segments_N`.
/// There may be one or more `segments_N` files in the index; however, the one with the
/// largest generation is the active one (when older `segments_N` files are present it's
/// because they temporarily cannot be deleted, or a custom
/// [`IndexDeletionPolicy`](crate::index::index_deletion_policy) is in use). This file lists each segment
/// by name and has details about the codec and generation of deletes.
///
/// Files:
///
/// - `segments_N`: Header, LuceneVersion, Version, NameCounter, SegCount,
///   MinSegmentLuceneVersion, `<SegName, SegID, SegCodec, DelGen, DeletionCount,
///   FieldInfosGen, DocValuesGen, UpdatesFiles><sup>SegCount</sup>, CommitUserData, Footer
///
/// Data types:
///
/// - `Header` -> [`IndexHeader`](crate::codecs::codec_util::write_index_header)
/// - `LuceneVersion` -> Which Lucene code [`Version`](crate::util::version::Version) was used for this commit,
///   written as three [`DataOutput::writeVInt`](crate::store::data_output::DataOutput::write_vint): major, minor, bugfix
/// - `MinSegmentLuceneVersion` -> Lucene code [`Version`](crate::util::version::Version) of the oldest segment,
///   written as three [`DataOutput::writeVInt`](crate::store::data_output::DataOutput::write_vint): major, minor, bugfix;
///   this is only written only if there's at least one segment
/// - `NameCounter`, `SegCount`, `DeletionCount` -> [`DataOutput::writeInt`](crate::store::data_output::DataOutput::write_int)
/// - `Generation`, `Version`, `DelGen`, `Checksum`, `FieldInfosGen`, `DocValuesGen` ->
///   [`DataOutput::writeLong`](crate::store::data_output::DataOutput::write_long)
/// - `SegID` -> [`DataOutput::writeByte`](crate::store::data_output::DataOutput::write_byte)
/// - `SegName`, `SegCodec` -> [`DataOutput::writeString`](crate::store::data_output::DataOutput::write_string)
/// - `CommitUserData` -> [`DataOutput::writeMapOfStrings`](crate::store::data_output::DataOutput::write_map_of_strings)
/// - `UpdatesFiles` -> Map<[`DataOutput::writeInt`](crate::store::data_output::DataOutput::write_int), [`DataOutput::writeSetOfStrings`](crate::store::data_output::DataOutput::write_set_of_strings)>
/// - `Footer` -> [`CodecUtil::writeFooter`](crate::codecs::codec_util::write_footer)
///
/// Field Descriptions:
///
/// - `Version` counts how often the index has been changed by adding or deleting documents.
/// - `NameCounter` is used to generate names for new segment files.
/// - `SegName` is the name of the segment, and is used as the file name prefix for all
///   of the files that compose the segment's index.
/// - `DelGen` is the generation count of the deletes file. If this is `-1`, there are no
///   deletes. Anything above zero means there are deletes stored by
///   [`LiveDocsFormat`](crate::codecs::live_docs_format).
/// - `DeletionCount` records the number of deleted documents in this segment.
/// - `SegCodec` is the [`Codec::getName`](crate::codecs::Codec::get_name) of the Codec that encoded this segment.
/// - `SegID` is the identifier of the Codec that encoded this segment.
/// - `CommitUserData` stores an optional user-supplied opaque `Map<String,String>` that was
///   passed to [`IndexWriter::setLiveCommitData`](crate::index::index_writer::IndexWriter::set_live_commit_data).
/// - `FieldInfosGen` is the generation count of the fieldInfos file. If this is `-1`,
///   there are no updates to the fieldInfos in that segment. Anything above zero means
///   there are updates to fieldInfos stored by [`FieldInfosFormat`](crate::codecs::field_infos_format::FieldInfosFormat).
/// - `DocValuesGen` is the generation count of the updatable DocValues. If this is `-1`,
///   there are no updates to DocValues in that segment. Anything above zero means there
///   are updates to DocValues stored by [`DocValuesFormat`](crate::codecs::doc_values_format::DocValuesFormat).
/// - `UpdatesFiles` stores the set of files that were updated in that segment per field.
///
/// # Notes
/// This module is experimental and subject to change.
pub struct SegmentInfos<D>
where
    D: Directory,
{
    /// Used to name new segments.
    pub counter: u64,
    /// Counts how often the index has been changed.
    pub version: u64,
    /// Generation of the "segments_N" for the next commit.
    pub generation: i64,
    /// Generation of the "segments_N" file we last successfully read or wrote.
    pub last_generation: i64,
    /// Opaque `HashMap<String, String>` that user can specify during `IndexWriter.commit`.
    pub user_data: HashMap<String, String>,
    /// List of `SegmentCommitInfo` objects.
    pub segments: Vec<SegmentCommitInfo<D>>,
    /// Id for this commit; only written starting with Lucene 5.0.
    pub id: Option<Vec<u8>>,
    /// Which Lucene version wrote this commit.
    pub lucene_version: Option<Version>,
    /// Version of the oldest segment in the index, or `None` if there are no segments.
    pub min_segment_lucene_version: Option<Version>,
    /// The Lucene version major that was used to create the index.
    pub index_created_version_major: u32,
    // Only true after prepareCommit has been called and
    // before finishCommit is called
    pending_commit: bool,
}

impl<D> SegmentInfos<D>
where
    D: Directory,
{
    /// Sole constructor.
    ///
    /// # Arguments
    /// - `index_created_version_major`: The Lucene version major at index creation time,
    ///   or 6 if the index was created before 7.0.
    pub fn new(index_created_version_major: u32) -> Result<SegmentInfos<D>, LuceneError> {
        if index_created_version_major > LATEST.major {
            return Err(LuceneError::illegal_argument(format!(
                "indexCreatedVersionMajor is in the future: {}",
                index_created_version_major
            )));
        }
        if index_created_version_major < 6 {
            return Err(LuceneError::illegal_argument(format!(
                "indexCreatedVersionMajor must be >= 6, got: {}",
                index_created_version_major
            )));
        }

        Ok(SegmentInfos {
            counter: 0,
            version: 0,
            generation: 0,
            last_generation: 0,
            user_data: HashMap::new(),
            segments: Vec::new(),
            id: None,
            lucene_version: None,
            min_segment_lucene_version: None,
            index_created_version_major,
            pending_commit: false,
        })
    }

    /// Get the generation of the most recent commit to the list of index files (N in the segments_N file).
    ///
    /// # Arguments
    /// - `files`: A slice of file names to check.
    pub fn get_last_commit_generation(files: &[String]) -> Result<i64, LuceneError> {
        let mut max = -1;
        for file in files {
            if file.starts_with(IndexFileNames::SEGMENTS)
                // skipping this file here helps deliver the right exception when opening an old index
                && file.starts_with(OLD_SEGMENTS_GEN) == false
            {
                let gen = Self::generation_from_segments_file_name(file)?;
                if gen > max {
                    max = gen;
                }
            }
        }
        Ok(max)
    }

    /// Get the generation of the most recent commit to the index in this directory.
    pub fn get_last_commit_generation_from_directory(directory: &D) -> Result<i64, LuceneError> {
        let files = directory.list_all()?;
        Self::get_last_commit_generation(&files)
    }

    /// Get the filename of the segments_N file for the most recent commit in the list of index files.
    pub fn get_last_commit_segments_file_name(
        files: &[String],
    ) -> Result<Option<String>, LuceneError> {
        let last_gen = Self::get_last_commit_generation(files)?;
        Ok(IndexFileNames::file_name_from_generation(
            IndexFileNames::SEGMENTS,
            "",
            last_gen,
        ))
    }

    /// Get the filename of the segments_N file for the most recent commit to the index in this Directory.
    pub fn get_last_commit_segments_file_name_from_directory(
        directory: &D,
    ) -> Result<Option<String>, LuceneError> {
        let last_gen = Self::get_last_commit_generation_from_directory(directory)?;
        Ok(IndexFileNames::file_name_from_generation(
            IndexFileNames::SEGMENTS,
            "",
            last_gen,
        ))
    }

    /// Get the segments_N filename in use by this segment infos.
    pub fn get_segments_file_name(&self) -> Option<String> {
        IndexFileNames::file_name_from_generation(
            IndexFileNames::SEGMENTS,
            "",
            self.last_generation,
        )
    }

    /// Parse the generation off the segments file name and return it.
    pub fn generation_from_segments_file_name(file_name: &str) -> Result<i64, LuceneError> {
        if file_name == OLD_SEGMENTS_GEN {
            Err(LuceneError::illegal_argument(format!(
                "\"{}\" is not a valid segment file name since 4.0",
                OLD_SEGMENTS_GEN
            )))
        } else if file_name == IndexFileNames::SEGMENTS {
            Ok(0)
        } else if file_name.starts_with(IndexFileNames::SEGMENTS) {
            let generation_str = &file_name[IndexFileNames::SEGMENTS.len() + 1..];
            match i64::from_str_radix(generation_str, 36) {
                Ok(generation) => Ok(generation),
                Err(_) => Err(LuceneError::illegal_argument(format!(
                    "Failed to parse generation from file name: \"{}\"",
                    file_name
                ))),
            }
        } else {
            Err(LuceneError::illegal_argument(format!(
                "fileName \"{}\" is not a segments file",
                file_name
            )))
        }
    }
    /// Returns the generation of the next pending `segments_N` that will be written.
    pub fn get_next_pending_generation(&self) -> i64 {
        if self.generation == -1 {
            1
        } else {
            self.generation + 1
        }
    }

    /// Since Lucene 5.0, every commit (`segments_N`) writes a unique id. This will return that id.
    pub fn get_id(&self) -> Option<Vec<u8>> {
        self.id.clone()
    }
    /// Read a particular `segmentFileName`. This may throw an error if a commit is in process.
    ///
    /// # Arguments
    ///
    /// - `directory`: Directory containing the segments file.
    /// - `segment_file_name`: The segment file to load.
    ///
    /// # Errors
    ///
    /// - Returns `LuceneError::CorruptIndex` if the index is corrupt.
    /// - Returns `LuceneError` for any low-level IO error.
    ///
    pub fn read_commit_with_min_version(
        directory: Arc<Mutex<D>>,
        segment_file_name: &str,
    ) -> Result<Self, LuceneError> {
        Self::read_commit_with_file_min_version(directory, segment_file_name, *MIN_SUPPORTED_MAJOR)
    }

    /// Reads a particular `segmentFileName`, as long as the commit's
    /// [`SegmentInfos::get_index_created_version_major`](SegmentInfos::get_index_created_version_major)
    /// is strictly greater than the provided minimum supported major version.
    ///
    /// If the commit's version is older, an [`IndexFormatTooOldException`](LuceneError::index_format_too_old)
    /// will be thrown. Note that this may return an `Err` if a commit is in process.
    pub fn read_commit_with_file_min_version(
        directory: Arc<Mutex<D>>,
        segment_file_name: &str,
        min_supported_major_version: u32,
    ) -> Result<SegmentInfos<D>, LuceneError> {
        let generation = SegmentInfos::<D>::generation_from_segments_file_name(segment_file_name)?;
        let mut input = match directory
            .lock()
            .map_err(|_| {
                LuceneError::illegal_argument("Failed to acquire directory lock.".to_string())
            })?
            .open_checksum_input(segment_file_name)
        {
            Ok(input) => input,
            Err(e) => {
                return Err(LuceneError::corrupt_index(format!(
                    "Unexpected file read error while opening index: {}",
                    e
                )));
            }
        };

        match SegmentInfos::read_commit_impl(
            directory.clone(),
            &mut input,
            generation,
            min_supported_major_version,
        ) {
            Ok(commit) => Ok(commit),
            Err(e) => Err(LuceneError::corrupt_index(format!(
                "Unexpected file read error while reading index: {:?}",
                e
            ))),
        }
    }

    /// Read the commit from the provided [`ChecksumIndexInput`](ChecksumIndexInput).
    pub fn read_commit_with_input<I: ChecksumIndexInput>(
        directory: Arc<Mutex<D>>,
        input: &mut I,
        generation: i64,
    ) -> Result<Self, LuceneError> {
        Self::read_commit_impl(directory, input, generation, *MIN_SUPPORTED_MAJOR)
    }
    /// Read the commit from the provided [`ChecksumIndexInput`](ChecksumIndexInput).
    pub fn read_commit_impl<I: ChecksumIndexInput>(
        directory: Arc<Mutex<D>>,
        input: &mut I,
        generation: i64,
        min_supported_major_version: u32,
    ) -> Result<Self, LuceneError> {
        let mut prior_error: Option<LuceneError> = None;

        // Read the magic number
        let magic = CodecUtil::read_be_int(input)?;
        if magic != CodecUtil::CODEC_MAGIC {
            return Err(LuceneError::index_format_too_old(format!("Format version is not supported (resource {}): {} (needs to be between {} and {}). This version of Lucene only supports indexes created with release {}.0 and later", input, magic, CodecUtil::CODEC_MAGIC, CodecUtil::CODEC_MAGIC, *MIN_SUPPORTED_MAJOR)));
        }
        let format =
            CodecUtil::check_header_no_magic(input, "segments", VERSION_74, VERSION_CURRENT)?;

        // Read the ID
        let mut id = vec![0u8; StringHelper::ID_LENGTH as usize];
        let id_len = id.len();
        debug_assert!(id_len <= u32::MAX as usize);
        input.read_bytes(&mut id, 0, id_len as u32)?;
        CodecUtil::check_index_header_suffix(input, &format!("{:x}", generation))?;

        let lucene_version = Version::from_bits(
            input.read_vint()? as u32,
            input.read_vint()? as u32,
            input.read_vint()? as u32,
        )?;

        let index_created_version = input.read_vint()?;
        debug_assert!(index_created_version >= 0);
        if lucene_version.major < index_created_version as u32 {
            return Err(LuceneError::corrupt_index(format!(
                "Creation version [{}] can't be greater than the version that wrote the segment infos: [{}]",
                index_created_version, lucene_version
            )));
        }

        if (index_created_version as u32) < min_supported_major_version {
            let reason = format!(
                "This index was initially created with Lucene {}.x while the current version is {} and Lucene only supports reading {}",
                index_created_version,
                LATEST,
                if min_supported_major_version == *MIN_SUPPORTED_MAJOR{
                    "the current and previous major versions".to_string()
                } else {
                    format!("from version {} upwards", min_supported_major_version)
                });
            return Err(LuceneError::index_format_too_old(format!("Format version is not supported (resource {}): {}. This version of Lucene only supports indexes created with release {}.0 and later by default.", input, reason, *MIN_SUPPORTED_MAJOR)));
        }

        let mut infos = Self::new(index_created_version as u32)?;
        infos.id = Some(id);
        infos.generation = generation;
        infos.last_generation = generation;
        infos.lucene_version = Some(lucene_version);
        if let Err(e) = Self::parse_segment_infos(directory, input, &mut infos, format) {
            prior_error = Some(e);
        }

        if format >= VERSION_74 {
            if prior_error.is_none() {
                CodecUtil::check_footer(input)?;
            } else {
                CodecUtil::check_footer_with_error(input, &mut prior_error.unwrap())?;
            }
        } else if let Some(e) = prior_error {
            return Err(e);
        }

        Ok(infos)
    }
    pub fn parse_segment_infos<I: DataInput>(
        directory: Arc<Mutex<D>>,
        input: &mut I,
        infos: &mut SegmentInfos<D>,
        format: u32,
    ) -> Result<(), LuceneError> {
        infos.version = CodecUtil::read_be_long(input)?;
        let counter_value = input.read_vlong()?;
        debug_assert!(counter_value >= 0);
        infos.counter = counter_value as u64;

        let num_segments = CodecUtil::read_be_int(input)? as i32;
        if num_segments < 0 {
            return Err(LuceneError::corrupt_index(format!(
                "Invalid segment count: {} (resource={})",
                num_segments, input
            )));
        }

        if num_segments > 0 {
            // Read minSegmentLuceneVersion
            infos.min_segment_lucene_version = Some(Version::from_bits(
                input.read_vint()? as u32,
                input.read_vint()? as u32,
                input.read_vint()? as u32,
            )?);
        }

        let mut total_docs = 0;

        for _ in 0..num_segments {
            let seg_name = input.read_string()?;
            let mut segment_id = vec![0u8; StringHelper::ID_LENGTH as usize];
            let segment_id_len = segment_id.len();
            debug_assert!(segment_id_len <= u32::MAX as usize);
            input.read_bytes(&mut segment_id, 0, segment_id_len as u32)?;
            let codec = Self::read_codec();
            let mut info = codec.segment_info_format().read(
                directory.clone(),
                &seg_name,
                segment_id,
                &IOContext::default_io_context()?,
            )?;
            info.set_codec(codec)?;

            let max_doc = info.max_doc()?;
            total_docs += max_doc;

            let del_gen = CodecUtil::read_be_long(input)?;
            let del_count = CodecUtil::read_be_int(input)?;
            if del_count > max_doc {
                return Err(LuceneError::corrupt_index(format!(
                    "Invalid deletion count: {} vs maxDoc={}, (resource={})",
                    del_count, max_doc, input
                )));
            }
            let field_infos_gen = CodecUtil::read_be_long(input)?;
            let dv_gen = CodecUtil::read_be_long(input)?;
            let soft_del_count = CodecUtil::read_be_int(input)?;
            if soft_del_count > max_doc {
                return Err(LuceneError::corrupt_index(format!(
                    "Invalid soft deletion count: {} vs maxDoc={}, (resource={})",
                    soft_del_count, max_doc, input
                )));
            }

            if soft_del_count + del_count > max_doc {
                return Err(LuceneError::corrupt_index(format!(
                    "Invalid combined deletion count: {} vs maxDoc={}, (resource={})",
                    soft_del_count + del_count,
                    max_doc,
                    input
                )));
            }

            let sci_id = if format > VERSION_74 {
                match input.read_byte()? {
                    1 => {
                        let mut id = vec![0u8; StringHelper::ID_LENGTH as usize];
                        let id_len = id.len();
                        debug_assert!(id_len <= u32::MAX as usize);
                        input.read_bytes(&mut id, 0, id_len as u32)?;
                        Some(id)
                    }
                    0 => None,
                    marker => {
                        return Err(LuceneError::corrupt_index(format!(
                            "Invalid SegmentCommitInfo ID marker: {}",
                            marker
                        )));
                    }
                }
            } else {
                None
            };

            if let Some(min_version) = &infos.min_segment_lucene_version {
                debug_assert!(info.get_version().is_some());
                if !info
                    .get_version()
                    .as_ref()
                    .unwrap()
                    .on_or_after(min_version)
                {
                    return Err(LuceneError::corrupt_index(format!(
                        "segments file recorded minSegmentLuceneVersion={} but segment={} has older version={} (resource={})",
                        min_version,
                        seg_name,
                        info.get_version().as_ref().unwrap(),
                        input
                    )));
                }
            }
            if infos.index_created_version_major >= 7 {
                if info.get_version().as_ref().unwrap().major < infos.index_created_version_major {
                    return Err(LuceneError::corrupt_index(format!(
                        "segments file recorded indexCreatedVersionMajor={} but segment={} has older version={} (resource={})",
                        infos.index_created_version_major,
                        seg_name,
                        info.get_version().as_ref().unwrap(),
                        input
                    )));
                }

                if info.get_min_version().is_none() {
                    return Err(LuceneError::corrupt_index(format!(
                        "segments infos must record minVersion with indexCreatedVersionMajor={} (resource={})",
                        infos.index_created_version_major,
                        input
                    )));
                }
            }

            let mut si_per_commit = SegmentCommitInfo::new(
                info,
                del_count as i32,
                soft_del_count as i32,
                del_gen as i64,
                field_infos_gen as i64,
                dv_gen as i64,
                sci_id,
            )?;
            si_per_commit.set_field_infos_files(input.read_set_of_strings()?);
            let num_dv_fields = CodecUtil::read_be_int(input)?;
            let dv_update_files = if num_dv_fields == 0 {
                HashMap::new()
            } else {
                let mut map = HashMap::new();
                for _ in 0..num_dv_fields {
                    map.insert(
                        CodecUtil::read_be_int(input)? as i32,
                        input.read_set_of_strings()?,
                    );
                }
                map
            };
            si_per_commit.set_doc_values_updates_files(dv_update_files);
            infos.add(si_per_commit)?;
        }
        infos.user_data = input.read_map_of_strings()?;
        // LUCENE-6299: check we are in bounds
        if total_docs > IndexWriter::get_actual_max_docs() {
            return Err(LuceneError::corrupt_index(format!(
                "Too many documents: an index cannot exceed {} but readers have total maxDoc={}",
                IndexWriter::get_actual_max_docs(),
                total_docs
            )));
        }
        Ok(())
    }

    pub fn read_codec() -> Lucene101Codec {
        Lucene101Codec
    }

    fn write_with_directory(&mut self, directory: &mut D) -> Result<(), LuceneError> {
        let next_generation = self.get_next_pending_generation();
        let segment_file_name_wrap = IndexFileNames::file_name_from_generation(
            IndexFileNames::PENDING_SEGMENTS,
            "",
            next_generation,
        );
        debug_assert!(segment_file_name_wrap.is_some());
        let segment_file_name = segment_file_name_wrap.unwrap();

        // Always advance the generation on write
        self.generation = next_generation;

        let mut success = false;
        {
            let result = (|| {
                {
                    let mut segn_output = Some(
                        directory
                            .create_output(&segment_file_name, IOContext::default_io_context()?)?,
                    );
                    if let Some(ref mut output) = segn_output {
                        self.write(output)?;
                    }
                }
                directory.sync(&[&segment_file_name])?;
                success = true;
                Ok(())
            })();
            if let Err(e) = result {
                // Try not to leave a truncated segments_N file in the index
                IOUtils::delete_files_ignoring_exceptions(directory, &[segment_file_name]);
                return Err(e);
            }
        }
        if success {
            self.pending_commit = true;
        }

        Ok(())
    }

    /// Write the current `SegmentInfos` to the provided `IndexOutput`.
    ///
    /// # Errors
    ///
    /// Returns a `LuceneError` if there is an issue writing the segment information.
    pub fn write<T: IndexOutput>(&self, out: &mut T) -> Result<(), LuceneError> {
        CodecUtil::write_index_header(
            out,
            "segments",
            VERSION_CURRENT,
            &StringHelper::random_id(),
            &format!("{:x}", self.generation),
        )?;
        out.write_vint(LATEST.major as i32)?;
        out.write_vint(LATEST.minor as i32)?;
        out.write_vint(LATEST.bug_fix as i32)?;

        out.write_vint(self.index_created_version_major as i32)?;
        debug_assert!(self.version <= i64::MAX as u64);
        CodecUtil::write_be_long(out, self.version as i64)?;
        out.write_vlong(self.counter as i64)?;
        CodecUtil::write_be_int(out, self.segments.len() as i32)?;

        if self.size() > 0 {
            let mut min_segment_version: Option<Version> = None;
            // We do a separate loop up front so we can write the minSegmentVersion before
            // any SegmentInfo; this makes it cleaner to throw IndexFormatTooOldExc at read time:
            for si_per_commit in &self.segments {
                let segment_version = si_per_commit.info.version.clone();
                debug_assert!(segment_version.is_some());
                if min_segment_version.is_none()
                    || !segment_version
                        .as_ref()
                        .unwrap()
                        .on_or_after(&min_segment_version.as_ref().unwrap())
                {
                    min_segment_version = segment_version;
                }
            }

            let min_version = min_segment_version.as_ref().unwrap();
            out.write_vint(min_version.major as i32)?;
            out.write_vint(min_version.minor as i32)?;
            out.write_vint(min_version.bug_fix as i32)?;
        }
        for si_per_commit in &self.segments {
            let si = &si_per_commit.info;
            if self.index_created_version_major >= 7 && si.min_version.is_none() {
                return Err(LuceneError::illegal_state(format!(
                    "Segments must record minVersion if they have been created on or after Lucene 7: {}",
                    si.name
                )));
            }
            out.write_string(&si.name)?;
            let segment_id = &si.get_id();
            let segment_id_len = segment_id.len();
            if segment_id_len != StringHelper::ID_LENGTH as usize {
                return Err(LuceneError::illegal_state(format!(
                    "Cannot write segment: invalid id segment={} id={:?}",
                    si.name, segment_id
                )));
            }
            debug_assert!(segment_id_len <= u32::MAX as usize);
            out.write_bytes_with_len(segment_id, segment_id_len as u32)?;
            out.write_string(&si.codec.as_ref().unwrap().get_name())?;

            CodecUtil::write_be_long(out, si_per_commit.del_gen)?;
            let del_count = si_per_commit.del_count;
            let max_doc = si.max_doc()?;
            if del_count < 0 || del_count > max_doc as i32 {
                return Err(LuceneError::illegal_state(format!(
                    "Cannot write segment: invalid maxDoc segment={} maxDoc={} delCount={}",
                    si.name, max_doc, del_count
                )));
            }
            CodecUtil::write_be_int(out, del_count)?;
            CodecUtil::write_be_long(out, si_per_commit.field_infos_gen)?;
            CodecUtil::write_be_long(out, si_per_commit.doc_values_gen)?;

            let soft_del_count = si_per_commit.soft_del_count;
            if soft_del_count < 0 || soft_del_count > max_doc as i32 {
                return Err(LuceneError::illegal_state(format!(
                    "Cannot write segment: invalid maxDoc segment={} maxDoc={} softDelCount={}",
                    si.name, max_doc, soft_del_count
                )));
            }
            CodecUtil::write_be_int(out, soft_del_count)?;

            if let Some(sci_id) = &si_per_commit.id {
                out.write_byte(1)?;
                let sci_id_len = sci_id.len();
                debug_assert_eq!(
                    sci_id_len,
                    StringHelper::ID_LENGTH as usize,
                    "Invalid SegmentCommitInfo#id: {:?}",
                    sci_id
                );
                debug_assert!(sci_id_len <= u32::MAX as usize);
                out.write_bytes_range(sci_id, 0, sci_id_len as u32)?;
            } else {
                out.write_byte(0)?;
            }

            out.write_set_of_strings(&si_per_commit.field_infos_files)?;

            let dv_updates_files = &si_per_commit.dv_updates_files;
            let dv_updates_files_len = dv_updates_files.len();
            debug_assert!(dv_updates_files_len <= i32::MAX as usize);
            CodecUtil::write_be_int(out, dv_updates_files_len as i32)?;
            for (key, value) in dv_updates_files {
                CodecUtil::write_be_int(out, *key)?;
                out.write_set_of_strings(value)?;
            }
        }
        out.write_map_of_strings(&self.user_data)?;
        CodecUtil::write_footer(out)?;

        Ok(())
    }

    pub fn try_clone(&self) -> Result<Self, LuceneError> {
        let mut cloned = Self {
            counter: self.counter,
            version: self.version,
            generation: self.generation,
            last_generation: self.last_generation,
            user_data: self.user_data.clone(),
            segments: Vec::with_capacity(self.segments.len()),
            id: self.id.clone(),
            lucene_version: self.lucene_version.clone(),
            min_segment_lucene_version: self.min_segment_lucene_version.clone(),
            index_created_version_major: self.index_created_version_major,
            pending_commit: false,
        };

        for segment_commit_info in &self.segments {
            debug_assert!(segment_commit_info.info.codec.is_some());
            cloned.add(segment_commit_info.clone())?;
        }
        Ok(cloned)
    }
    /// Returns the version number when this `SegmentInfos` was generated.
    pub fn get_version(&self) -> u64 {
        self.version
    }

    /// Returns the current generation.
    pub fn get_generation(&self) -> i64 {
        self.generation
    }

    /// Returns the last successfully read or written generation.
    pub fn get_last_generation(&self) -> i64 {
        self.last_generation
    }
    /// Carry over generation numbers from another `SegmentInfos`.
    pub fn update_generation(&mut self, other: &SegmentInfos<D>) {
        self.last_generation = other.last_generation;
        self.generation = other.generation;
    }

    /// Carry over generation numbers, and version/counter, from another `SegmentInfos`.
    pub fn update_generation_version_and_counter(&mut self, other: &SegmentInfos<D>) {
        self.update_generation(other);
        self.version = other.version;
        self.counter = other.counter;
    }

    /// Set the generation to be used for the next commit.
    pub fn set_next_write_generation(&mut self, generation: i64) -> Result<(), LuceneError> {
        if generation < self.generation {
            return Err(LuceneError::illegal_state(format!(
                "Cannot decrease generation to {} from current generation {}",
                generation, self.generation
            )));
        }
        self.generation = generation;
        Ok(())
    }

    /// Rollback a pending commit.
    pub fn rollback_commit(&mut self, directory: &mut D) {
        if self.pending_commit {
            self.pending_commit = false;

            // We try to clean up our pending_segments_N

            // Must carefully compute fileName from "generation"
            // since lastGeneration isn't incremented:
            if let Some(pending) = IndexFileNames::file_name_from_generation(
                IndexFileNames::PENDING_SEGMENTS,
                "",
                self.generation,
            ) {
                // Suppress so we keep throwing the original exception in our caller
                IOUtils::delete_files_ignoring_exceptions(directory, &[pending]);
            }
        }
    }
    /// Call this to start a commit. This writes the new segments file, but writes an invalid checksum
    /// at the end, so that it is not visible to readers. Once this is called you must call [`finish_commit`](SegmentInfos::finish_commit)
    /// to complete the commit or [`rollback_commit`](SegmentInfos::rollback_commit) to abort it.
    ///
    /// Note: [`changed()`](SegmentInfos::changed) should be called prior to this method if changes have been made to this [`SegmentInfos`](SegmentInfos) instance.
    pub fn prepare_commit(&mut self, dir: &mut D) -> Result<(), LuceneError> {
        if self.pending_commit {
            return Err(LuceneError::illegal_state(
                "prepare_commit was already called".to_string(),
            ));
        }
        dir.sync_metadata()?;
        self.write_with_directory(dir)?;
        Ok(())
    }

    /// Returns all file names referenced by `SegmentInfo`. The returned collection is recomputed on each invocation.
    pub fn files(&self, include_segments_file: bool) -> Result<HashSet<String>, LuceneError> {
        let mut files = HashSet::new();
        if include_segments_file {
            if let Some(segment_file_name) = self.get_segments_file_name() {
                files.insert(segment_file_name);
            }
        }
        for segment_commit_info in &self.segments {
            files.extend(segment_commit_info.files()?);
        }
        Ok(files)
    }
    /// Returns the committed `segments_N` filename.
    pub fn finish_commit(&mut self, dir: &mut D) -> Result<String, LuceneError> {
        if !self.pending_commit {
            return Err(LuceneError::illegal_state(
                "prepare_commit was not called".to_string(),
            ));
        }

        let mut success_rename_and_sync = false;

        let result = (|| {
            let dest;
            let src = IndexFileNames::file_name_from_generation(
                IndexFileNames::PENDING_SEGMENTS,
                "",
                self.generation,
            )
            .ok_or_else(|| {
                LuceneError::illegal_state("Failed to generate source file name.".to_string())
            })?;
            dest = IndexFileNames::file_name_from_generation(
                IndexFileNames::SEGMENTS,
                "",
                self.generation,
            )
            .ok_or_else(|| {
                LuceneError::illegal_state("Failed to generate destination file name.".to_string())
            })?;
            dir.rename(&src, &dest)?;
            dir.sync_metadata()?;
            success_rename_and_sync = true;
            Ok(dest)
        })();

        match result {
            Ok(dest_file) => {
                self.pending_commit = false;
                self.last_generation = self.generation;
                Ok(dest_file)
            }
            Err(e) => {
                if !success_rename_and_sync {
                    // Attempt to roll back the commit if renaming or syncing failed
                    self.rollback_commit(dir);
                }
                Err(e)
            }
        }
    }
    /// Writes and syncs to the Directory, taking care to remove the segments file on exception.
    ///
    /// Note: [`changed()`](SegmentInfos::changed) should be called prior to this method if changes have been made to this [`SegmentInfos`](SegmentInfos) instance.
    pub fn commit(&mut self, dir: &mut D) -> Result<(), LuceneError> {
        self.prepare_commit(dir)?;
        self.finish_commit(dir)?;
        Ok(())
    }
    /// Returns `user_data` saved with this commit.
    pub fn get_user_data(&self) -> &HashMap<String, String> {
        &self.user_data
    }

    /// Sets the commit data.
    pub fn set_user_data(
        &mut self,
        data: Option<HashMap<String, String>>,
        do_increment_version: bool,
    ) {
        if let Some(new_data) = data {
            self.user_data = new_data;
        } else {
            self.user_data = HashMap::new();
        }

        if do_increment_version {
            self.changed();
        }
    }

    /// Replaces all segments in this instance, but keeps generation, version, counter so that future commits remain write-once.
    pub fn replace(&mut self, other: Self) {
        self.rollback_segment_infos(other.segments);
        self.last_generation = other.last_generation;
        self.user_data = other.user_data.clone();
    }

    /// Returns the sum of all segment's `max_docs`. Note that this does not include deletions.
    pub fn total_max_doc(&self) -> Result<i64, LuceneError> {
        let mut count: i64 = 0;
        for segment_commit_info in &self.segments {
            count += segment_commit_info.info.max_doc()? as i64;
        }

        // Ensure we don't exceed the actual max document limit.
        debug_assert!(count <= IndexWriter::get_actual_max_docs() as i64);
        Ok(count)
    }
    /// Call this before committing if changes have been made to the segments.
    pub fn changed(&mut self) {
        self.version += 1;
    }

    /// Set the version to a new value. The new version must be greater than or equal to the current version.
    pub fn set_version(&mut self, new_version: u64) -> Result<(), LuceneError> {
        if new_version < self.version {
            return Err(LuceneError::illegal_argument(format!(
                "newVersion (={}) cannot be less than current version (={})",
                new_version, self.version
            )));
        }
        self.version = new_version;
        Ok(())
    }
    // /// Applies all changes caused by committing a merge to this `SegmentInfos`.
    // pub fn apply_merge_changes(
    //     &mut self,
    //     merge: &MergePolicy<D, W>,
    //     drop_segment: bool,
    // ) -> Result<(), LuceneError> {
    //     if self.index_created_version_major >= 7 && merge.info.info.min_version.is_none() {
    //         return Err(LuceneError::illegal_argument(
    //             "All segments must record the minVersion for indices created on or after Lucene 7"
    //                 .to_string(),
    //         ));
    //     }
    //
    //     let merged_away: HashSet<_> = merge.segments.iter().cloned().collect();
    //     let mut inserted = false;
    //     let mut new_seg_idx = 0;
    //
    //     for seg_idx in 0..self.segments.len() {
    //         debug_assert!(seg_idx >= new_seg_idx);
    //         let info = &self.segments[seg_idx];
    //         if merged_away.contains(info) {
    //             if !inserted && !drop_segment {
    //                 self.segments[new_seg_idx] = merge.info.clone();
    //                 inserted = true;
    //                 new_seg_idx += 1;
    //             }
    //         } else {
    //             self.segments[new_seg_idx] = info.clone();
    //             new_seg_idx += 1;
    //         }
    //     }
    //
    //     // Remove duplicate segments from the list
    //     self.segments.truncate(new_seg_idx);
    //
    //     // If we didn't insert the new segment, check if we should add it to the beginning
    //     if !inserted && !drop_segment {
    //         self.segments.insert(0, merge.info.clone());
    //     }
    //
    //     Ok(())
    // }
    pub fn create_backup_segment_infos(&self) -> Result<Vec<SegmentCommitInfo<D>>, LuceneError> {
        let mut backup_list = Vec::with_capacity(self.segments.len());
        for segment_commit_info in &self.segments {
            debug_assert!(
                segment_commit_info.info.codec.is_some(),
                "Codec is None for segment {}",
                segment_commit_info.info.name
            );
            backup_list.push(segment_commit_info.clone());
        }
        Ok(backup_list)
    }

    pub fn rollback_segment_infos(&mut self, infos: Vec<SegmentCommitInfo<D>>) {
        self.segments.clear();
        self.segments.extend(infos);
    }
    /// Returns an iterator over the contained segments in order.
    pub fn iter(&self) -> impl Iterator<Item = &SegmentCommitInfo<D>> {
        self.segments.iter()
    }

    /// Returns all contained segments as a non-mutable reference to the internal vector.
    pub fn as_list(&self) -> &[SegmentCommitInfo<D>] {
        &self.segments
    }

    /// Returns the number of `SegmentCommitInfo`s.
    pub fn size(&self) -> u32 {
        let len = self.segments.len();
        debug_assert!(len <= u32::MAX as usize);
        len as u32
    }

    /// Appends the provided `SegmentCommitInfo` to the `segments` list.
    pub fn add(&mut self, si: SegmentCommitInfo<D>) -> Result<(), LuceneError> {
        if self.index_created_version_major >= 7 && si.info.min_version.is_none() {
            return Err(LuceneError::illegal_argument(format!(
                "All segments must record the minVersion for indices created on or after Lucene 7, but minVersion is missing for segment: {}",
                si
            )));
        }
        self.segments.push(si);
        Ok(())
    }

    /// Appends the provided [`SegmentCommitInfo`](SegmentCommitInfo)s.
    pub fn add_all(
        &mut self,
        sis: impl IntoIterator<Item = SegmentCommitInfo<D>>,
    ) -> Result<(), LuceneError> {
        for si in sis {
            self.add(si)?;
        }
        Ok(())
    }

    /// Clears all `SegmentCommitInfo`s.
    pub fn clear(&mut self) {
        self.segments.clear();
    }

    /// Removes the provided `SegmentCommitInfo`.
    ///
    /// **Warning**: O(N) cost
    pub fn remove(&mut self, si: &SegmentCommitInfo<D>) -> bool
    where
        D: PartialEq,
    {
        if let Some(pos) = self.segments.iter().position(|x| x == si) {
            self.segments.remove(pos);
            true
        } else {
            false
        }
    }

    /// Removes the `SegmentCommitInfo` at the provided index.
    ///
    /// **Warning**: O(N) cost
    pub fn remove_at(&mut self, index: usize) {
        if index < self.segments.len() {
            self.segments.remove(index);
        }
    }

    /// Returns true if the provided `SegmentCommitInfo` is contained.
    ///
    /// **Warning**: O(N) cost
    pub fn contains(&self, si: &SegmentCommitInfo<D>) -> bool
    where
        D: PartialEq,
    {
        self.segments.contains(si)
    }

    /// Returns the index of the provided `SegmentCommitInfo`.
    ///
    /// **Warning**: O(N) cost
    pub fn index_of(&self, si: &SegmentCommitInfo<D>) -> Option<usize>
    where
        D: PartialEq,
    {
        self.segments.iter().position(|x| x == si)
    }

    /// Returns the `Version` of the Lucene commit.
    pub fn get_commit_lucene_version(&self) -> Option<&Version> {
        self.lucene_version.as_ref()
    }

    /// Returns the `Version` of the oldest segment, or `None` if there are no segments.
    pub fn get_min_segment_lucene_version(&self) -> Option<&Version> {
        self.min_segment_lucene_version.as_ref()
    }

    /// Returns the version major that was used to initially create the index.
    /// This version is set when the index is first created and then never changes.
    /// Older indices report 6 as the creation version.
    pub fn get_index_created_version_major(&self) -> u32 {
        self.index_created_version_major
    }
}

impl<D> fmt::Display for SegmentInfos<D>
where
    D: Directory,
{
    /// Returns a readable description of this segment.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.get_segments_file_name().unwrap_or_default())?;
        let count = self.size();
        for (i, segment_commit_info) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(
                f,
                "{}",
                segment_commit_info.to_string_with_pending_del_count(0)
            )?
        }
        Ok(())
    }
}

/// Sets the global INFO_STREAM to the given `OutputEnum`.
pub fn set_info_stream(output: OutputEnum) {
    let mut info_stream = INFO_STREAM.lock().unwrap();
    *info_stream = Some(Arc::new(Mutex::new(output)));
}

/// Returns the current global INFO_STREAM as an `Option<Arc<Mutex<OutputEnum>>>`.
pub fn get_info_stream() -> Option<Arc<Mutex<OutputEnum>>> {
    let info_stream = INFO_STREAM.lock().unwrap();
    info_stream.clone()
}

/// Prints a message to the INFO_STREAM, if it is set.
/// This function assumes the caller has checked whether INFO_STREAM is `Some`.
pub fn message(msg: &str) -> io::Result<()> {
    let info_stream = INFO_STREAM.lock().unwrap();
    if let Some(ref stream) = *info_stream {
        let mut stream = stream.lock().unwrap();
        writeln!(stream, "SIS: {}", msg)?;
    }
    Ok(())
}
