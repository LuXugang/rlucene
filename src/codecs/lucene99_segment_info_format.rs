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
use crate::codecs::segment_info_format::SegmentInfoFormat;
use crate::codecs::{
    check_footer, check_footer_with_error, check_index_header, write_footer, write_index_header,
};
use crate::index::index_sorter::IndexSorter;
use crate::index::segment_info::{SegmentInfo, NO, YES};
use crate::index::sort::Sort;
use crate::index::sort_field_provider::{for_name, write, SortFieldProvider};
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::store::{DataInput, DataOutput, IOContext};
use crate::util::error::lucene_error::LuceneError;
use crate::util::Version;

/// Lucene 9.9 Segment info format.
///
/// # Files
///
/// - `.si`: Header, SegVersion, SegSize, IsCompoundFile, Diagnostics, Files, Attributes, IndexSort, Footer
///
/// # Data Types
///
/// - **Header** --> [`CodecUtil::write_index_header`](crate::codecs::codec_util::write_index_header)
/// - **SegSize** --> [`DataOutput::write_int`](crate::store::data_output::DataOutput::write_int) (Int32)
/// - **SegVersion** --> [`DataOutput::write_string`](crate::store::data_output::DataOutput::write_string) (String)
/// - **SegMinVersion** --> [`DataOutput::write_string`](crate::store::data_output::DataOutput::write_string) (String)
/// - **Files** --> [`DataOutput::write_set_of_strings`](crate::store::data_output::DataOutput::write_set_of_strings) (Set<String>)
/// - **Diagnostics**, **Attributes** --> [`DataOutput::write_map_of_strings`](crate::store::data_output::DataOutput::write_map_of_strings) (Map<String, String>)
/// - **IsCompoundFile** --> [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte) (Int8)
/// - **HasBlocks** --> [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte) (Int8)
/// - **IndexSort** --> [`DataOutput::write_vint`](crate::store::data_output::DataOutput::write_vint) (Int32) count, followed by `count` SortField
/// - **SortField** --> [`DataOutput::write_string`](crate::store::data_output::DataOutput::write_string) (String) sort class, followed by a per-sort bytestream
///   (see [`SortFieldProvider::read_sort_field`](crate::index::sort_field_provider::SortFieldProvider::read_sort_field))
/// - **Footer** --> [`CodecUtil::write_footer`](crate::codecs::codec_util::write_footer)
///
/// # Field Descriptions
///
/// - **SegVersion**: The code version that created the segment.
/// - **SegMinVersion**: The minimum code version that contributed documents to the segment.
/// - **SegSize**: The number of documents contained in the segment index.
/// - **IsCompoundFile**: Records whether the segment is written as a compound file or not. If this is `-1`, the segment is not a compound file. If it is `1`, the segment is a compound file.
/// - **HasBlocks**: Records whether the segment contains documents written as a block and guarantees consecutive document IDs for all documents in the block.
/// - **Diagnostics Map**: Privately written by [`IndexWriter`](crate::index::index_writer::IndexWriter), as a debugging aid, for each segment it creates. It includes metadata like the current Lucene version, OS, Java version, why the segment was created (merge, flush, addIndexes), etc.
/// - **Files**: A list of files referred to by this segment.
///
/// # See Also
/// - [`SegmentInfos`](crate::index::segment_infos::SegmentInfos)
///
/// # Lucene Experimental
/// This API is experimental and may change in future versions.
pub struct Lucene99SegmentInfoFormat;

pub const SI_EXTENSION: &str = "si";
pub const CODEC_NAME: &str = "Lucene90SegmentInfo";
pub const VERSION_START: u32 = 0;
pub const VERSION_CURRENT: u32 = VERSION_START;

impl Lucene99SegmentInfoFormat {
    fn parse_segment_info<'a, D: Directory, T: DataInput>(
        dir: &'a mut D,
        input: &mut T,
        segment: &str,
        segment_id: Vec<u8>,
    ) -> Result<SegmentInfo<'a, D>, LuceneError> {
        let major = input.read_int()?;
        debug_assert!(major >= 0);
        let minor = input.read_int()?;
        debug_assert!(minor >= 0);
        let bug_fix = input.read_int()?;
        debug_assert!(bug_fix >= 0);
        let version = Version::from_bits(major as u32, minor as u32, bug_fix as u32)?;

        let has_min_version = input.read_byte()?;
        let min_version = match has_min_version {
            0 => None,
            1 => {
                let major = input.read_int()?;
                debug_assert!(major >= 0);
                let minor = input.read_int()?;
                debug_assert!(minor >= 0);
                let bug_fix = input.read_int()?;
                debug_assert!(bug_fix >= 0);
                Some(Version::from_bits(
                    major as u32,
                    minor as u32,
                    bug_fix as u32,
                )?)
            }
            _ => {
                return Err(LuceneError::corrupt_index(format!(
                    "Illegal boolean value : {} (resource={})",
                    has_min_version, input
                )))
            }
        };

        let doc_count = input.read_int()?;
        if doc_count < 0 {
            return Err(LuceneError::corrupt_index(format!(
                "Invalid docCount: {} (resource={})",
                doc_count, input
            )));
        }
        let is_compound_file = input.read_byte()? == YES as u8;
        let has_blocks = input.read_byte()? == YES as u8;
        let diagnostics = input.read_map_of_strings()?;
        let files = input.read_set_of_strings()?;
        let attributes = input.read_map_of_strings()?;
        let num_sort_fields = input.read_vint()?;
        let index_sort = match num_sort_fields.cmp(&0) {
            std::cmp::Ordering::Greater => {
                let mut sort_fields = Vec::with_capacity(num_sort_fields as usize);
                for _ in 0..num_sort_fields {
                    let name = input.read_string()?;
                    let sort_field = for_name(&name).read_sort_field(input)?;
                    sort_fields.push(sort_field);
                }
                Some(Sort::new_with_fields(sort_fields)?)
            }
            std::cmp::Ordering::Less => {
                return Err(LuceneError::corrupt_index(format!(
                    "invalid index sort field count: {} (resource={})",
                    num_sort_fields, input
                )));
            }
            std::cmp::Ordering::Equal => None,
        };

        let mut si = SegmentInfo::new(
            dir,
            Option::from(version),
            min_version,
            segment.to_string(),
            Option::from(doc_count as u32),
            is_compound_file,
            has_blocks,
            None,
            diagnostics,
            segment_id,
            attributes,
            index_sort,
        )?;
        si.set_files(files);
        Ok(si)
    }
    fn write_segment_info<T: DataOutput, D: Directory>(
        output: &mut T,
        si: &SegmentInfo<D>,
    ) -> Result<(), LuceneError> {
        let version_wrap = si.get_version();
        debug_assert!(version_wrap.is_some());
        let version = version_wrap.unwrap();
        if version.major < 7 {
            return Err(LuceneError::illegal_argument(format!(
                "invalid major version: should be >= 7 but got: {} segment={}",
                version.major, si
            )));
        }
        output.write_int(version.major as i32)?;
        output.write_int(version.minor as i32)?;
        output.write_int(version.bug_fix as i32)?;

        // Write the min Lucene version that contributed docs to the segment, since 7.0
        if let Some(min_version) = si.get_min_version() {
            output.write_byte(1)?;
            output.write_int(min_version.major as i32)?;
            output.write_int(min_version.minor as i32)?;
            output.write_int(min_version.bug_fix as i32)?;
        } else {
            output.write_byte(0)?;
        }

        debug_assert_eq!(version.prerelease, 0);
        output.write_int(si.max_doc()? as i32)?;

        output.write_byte(if si.get_use_compound_file() {
            YES as u8
        } else {
            NO as u8
        })?;
        output.write_byte(if si.get_has_blocks() {
            YES as u8
        } else {
            NO as u8
        })?;
        output.write_map_of_strings(si.get_diagnostics())?;

        let files = si.files()?;
        for file in files {
            if IndexFileNames::parse_segment_name(file) != si.name {
                return Err(LuceneError::illegal_argument(format!(
                    "invalid files: expected segment={}, got file={}",
                    si.name, file
                )));
            }
        }
        output.write_set_of_strings(files)?;
        output.write_map_of_strings(&si.get_attributes())?;

        if let Some(index_sort) = si.get_index_sort() {
            let sort_fields = index_sort.get_sort();
            let num_sort_fields = sort_fields.len();
            output.write_vint(num_sort_fields as i32)?;

            for sort_field in sort_fields {
                if let Some(sorter) = sort_field.get_index_sorter() {
                    output.write_string(sorter.get_provider_name())?;
                    write(sort_field, output)?;
                } else {
                    return Err(LuceneError::illegal_argument(format!(
                        "cannot serialize SortField {}",
                        sort_field
                    )));
                }
            }
        } else {
            output.write_vint(0)?;
        }
        Ok(())
    }
}

impl SegmentInfoFormat for Lucene99SegmentInfoFormat {
    fn read<'a, D: Directory>(
        &self,
        dir: &'a mut D,
        segment: &str,
        segment_id: Vec<u8>,
        _context: &IOContext,
    ) -> Result<SegmentInfo<'a, D>, LuceneError> {
        let file_name = IndexFileNames::segment_file_name(segment, "", SI_EXTENSION);
        let mut input = dir.open_checksum_input(&file_name)?;

        let mut prior_e: Option<LuceneError> = None;
        let mut si: Option<SegmentInfo<D>> = None;
        {
            let result = {
                let check_result = check_index_header(
                    &mut input,
                    CODEC_NAME,
                    VERSION_START,
                    VERSION_CURRENT,
                    &segment_id,
                    "",
                );
                match check_result {
                    Ok(_) => match Self::parse_segment_info(dir, &mut input, segment, segment_id) {
                        Ok(parsed_info) => {
                            si = Some(parsed_info);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                }
            };

            // Catch the exception if there was one during the reading process
            if let Err(exception) = result {
                prior_e = Some(exception);
            }
        }
        if prior_e.is_some() {
            check_footer_with_error(&mut input, &mut prior_e.unwrap())?;
        } else {
            check_footer(&mut input)?;
        }
        si.ok_or_else(|| {
            LuceneError::corrupt_index(format!("Failed to parse segment info for {}", segment))
        })
    }

    fn write<D: Directory>(
        &self,
        dir: &mut D,
        si: &mut SegmentInfo<D>,
        io_context: IOContext,
    ) -> Result<(), LuceneError> {
        let file_name = IndexFileNames::segment_file_name(&si.name, "", SI_EXTENSION);
        let mut output = dir.create_output(&file_name, io_context)?;
        si.add_file(file_name.clone())?;
        write_index_header(
            &mut output,
            CODEC_NAME,
            VERSION_CURRENT,
            si.get_id().as_slice(),
            "",
        )?;
        Self::write_segment_info(&mut output, si)?;
        write_footer(&mut output)?;

        Ok(())
    }
}
