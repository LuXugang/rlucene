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
use crate::index::BytesRef;
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::check_sum_index_input::ChecksumIndexInput;
use crate::store::data_output::DataOutput;
use crate::store::index_input::IndexInput;
use crate::store::{DataInput, IndexOutput};
use crate::util::error::lucene_error::LuceneError;
use crate::util::version::MIN_SUPPORTED_MAJOR;
use crate::util::StringHelper;
use std::cmp::Ordering;

/// Utility class for reading and writing versioned headers.
///
/// Writing codec headers is useful to ensure that a file is in the format you expect it to be.
///
/// # Experimental
/// This is an experimental API and may be subject to change in future versions.
#[allow(dead_code)] // for quick search
pub struct CodecUtil;
impl CodecUtil {
    /// Constant to identify the start of a codec header.
    pub const CODEC_MAGIC: u32 = 0x3fd76c17;
    /// Constant to identify the start of a codec footer.
    pub const FOOTER_MAGIC: u32 = !Self::CODEC_MAGIC;
    /// Writes a codec header, which records both a string to identify the file and a version number.
    /// This header can be parsed and validated with [`check_header`].
    ///
    /// # Format
    /// `CodecHeader -> Magic, CodecName, Version`
    ///
    /// - **Magic**:  
    ///   A `u32` (written using `write_int`). This identifies the start of the header.  
    ///   It is always [`CodecUtil::CODEC_MAGIC`].
    ///
    /// - **CodecName**:  
    ///   A string (written using `write_string`). This is a string to identify this file.
    ///
    /// - **Version**:  
    ///   A `u32` (written using `write_int`). Records the version of the file.
    ///
    /// # Notes
    /// The length of a codec header depends only on the name of the codec. This length can be computed
    /// at any time with [`header_length`].
    ///
    /// # Parameters
    /// - `out`: The output stream to write to.
    /// - `codec`: A string to identify this file. It should be simple ASCII and less than 128 characters
    ///   in length.
    /// - `version`: The version number.
    ///
    /// # Errors
    /// - `IoError`: If there is an I/O error writing to the underlying medium.
    /// - `IllegalArgumentError`: If the codec name is not simple ASCII or is more than 127 characters in length.
    ///
    /// # See Also
    /// - [`check_header`]
    /// - [`header_length`]
    pub fn write_header(
        out: &mut impl DataOutput,
        codec: &str,
        version: u32,
    ) -> Result<(), LuceneError> {
        let bytes = BytesRef::new_from_string(codec);
        if bytes.length as usize != codec.len() || bytes.length >= 128 {
            return Err(LuceneError::illegal_argument(format!(
                "codec must be simple ASCII, less than 128 characters in length got {}",
                codec
            )));
        }
        Self::write_be_int(out, CodecUtil::CODEC_MAGIC)?;
        out.write_string(codec)?;
        Self::write_be_int(out, version)?;
        Ok(())
    }

    /// Writes a codec header, which records both a string to identify the file and a version number.
    /// This header can be parsed and validated with [`check_header`].
    ///
    /// # Format
    /// `CodecHeader -> Magic, CodecName, Version`
    ///
    /// - **Magic**:  
    ///   A `u32` (written using `write_int`). This identifies the start of the header.
    ///   It is always [`CodecUtil::CODEC_MAGIC`].
    ///
    /// - **CodecName**:  
    ///   A string (written using `write_string`). This is a string to identify this file.
    ///
    /// - **Version**:  
    ///   A `u32` (written using `write_int`). Records the version of the file.
    ///
    /// # Notes
    /// The length of a codec header depends only on the name of the codec. This length can be computed
    /// at any time with [`header_length`].
    ///
    /// # Parameters
    /// - `out`: The output stream.
    /// - `codec`: A string to identify this file. It should be simple ASCII and less than 128 characters
    ///   in length.
    /// - `version`: The version number.
    ///
    /// # Errors
    /// - Returns an error if there is an I/O error writing to the underlying medium.
    /// - Returns an error if the codec name is not simple ASCII or exceeds 127 characters in length.
    ///
    /// # See Also
    /// - [`check_header`]
    /// - [`header_length`]
    pub fn write_index_header(
        out: &mut impl DataOutput,
        codec: &str,
        version: u32,
        id: &[u8],
        suffix: &str,
    ) -> Result<(), LuceneError> {
        if id.len() != StringHelper::ID_LENGTH as usize {
            return Err(LuceneError::illegal_argument(format!(
                "Invalid id: {}",
                StringHelper::id_to_string(Option::from(id))
            )));
        }
        Self::write_header(out, codec, version)?;
        out.write_bytes_range(id, 0, StringHelper::ID_LENGTH)?;
        let suffix_bytes = BytesRef::new_from_string(suffix);
        if !suffix.is_ascii() || suffix_bytes.length >= 256 {
            return Err(LuceneError::illegal_argument(format!(
                "suffix must be simple ASCII, less than 256 characters in length got {}",
                suffix
            )));
        }
        out.write_byte(suffix_bytes.length as u8)?;
        out.write_bytes_range(
            &suffix_bytes.bytes,
            suffix_bytes.offset,
            suffix_bytes.length,
        )?;
        Ok(())
    }
    /// Computes the length of a codec header.
    ///
    /// # Parameters
    /// - `codec`: The codec name.
    ///
    /// # Returns
    /// The length of the entire codec header.
    ///
    /// # See Also
    /// - [`write_header`]
    pub fn header_length(codec: &str) -> u32 {
        9 + codec.len() as u32
    }
    /// Computes the length of an index header.
    ///
    /// # Parameters
    /// - `codec`: The codec name.
    ///
    /// # Returns
    /// The length of the entire index header.
    ///
    /// # See Also
    /// - [`write_index_header`]
    pub fn index_header_length(codec: &str, suffix: &str) -> u32 {
        Self::header_length(codec) + StringHelper::ID_LENGTH + 1 + (suffix.len() as u32)
    }
    /// Reads and validates a header previously written with [`write_header`].
    ///
    /// When reading a file, supply the expected `codec` and an expected version range
    /// (`min_version` to `max_version`).
    ///
    /// # Parameters
    /// - `input`: The input stream, positioned at the point where the header was previously written.
    ///   Typically, this is located at the beginning of the file.
    /// - `codec`: The expected codec name.
    /// - `min_version`: The minimum supported version number.
    /// - `max_version`: The maximum supported version number.
    ///
    /// # Returns
    /// The actual version found if a valid header is found that matches `codec`, with an actual version
    /// satisfying `min_version <= actual <= max_version`. Otherwise, an error is returned.
    ///
    /// # Errors
    /// - `CorruptIndexError`: If the first four bytes are not [`CodecUtil::CODEC_MAGIC`] or if the codec does not match `codec`.
    /// - `IndexFormatTooOldError`: If the actual version is less than `min_version`.
    /// - `IndexFormatTooNewError`: If the actual version is greater than `max_version`.
    /// - `IoError`: If there is an I/O error reading from the underlying medium.
    ///
    /// # See Also
    /// - [`write_header`]
    pub fn check_header(
        data_input: &mut impl DataInput,
        codec: &str,
        min_version: u32,
        max_version: u32,
    ) -> Result<u32, LuceneError> {
        let actual_header = Self::read_be_int(data_input)?;
        if actual_header != CodecUtil::CODEC_MAGIC {
            return Err(LuceneError::corrupt_index(format!(
                "codec header mismatch: actual header={} vs expected header={}",
                actual_header,
                CodecUtil::CODEC_MAGIC
            )));
        }
        Self::check_header_no_magic(data_input, codec, min_version, max_version)
    }
    /// Similar to [`check_header`], except this version assumes the first `u32`
    /// has already been read and validated from the input.
    ///
    /// # See Also
    /// - [`check_header`]
    pub fn check_header_no_magic(
        data_input: &mut impl DataInput,
        codec: &str,
        min_version: u32,
        max_version: u32,
    ) -> Result<u32, LuceneError> {
        let actual_codec = data_input.read_string()?;
        if actual_codec != codec {
            return Err(LuceneError::corrupt_index(format!(
                "codec mismatch: actual codec={} vs expected codec={}",
                actual_codec, codec
            )));
        }
        let actual_version = Self::read_be_int(data_input)?;
        if (actual_version as u32) < min_version {
            return Err(LuceneError::index_format_too_old(format!("Format version is not supported (resource {}): {} (needs to be between {} and {}). This version of Lucene only supports indexes created with release {}.0 and later", data_input, actual_version, min_version, max_version, *MIN_SUPPORTED_MAJOR)));
        }
        if (actual_version as u32) > max_version {
            return Err(LuceneError::index_format_too_new(format!(
                "Format version is not supported (resource {}): {} (needs to be between {} and {}) ",
                data_input, actual_version, min_version, max_version
            )));
        }
        Ok(actual_version)
    }
    /// Reads and validates a header previously written with [`write_index_header`].
    ///
    /// When reading a file, supply the expected `codec`, expected version range (`min_version` to
    /// `max_version`), object ID, and suffix.
    ///
    /// # Parameters
    /// - `input`: The input stream, positioned at the point where the header was previously written.
    ///   Typically, this is located at the beginning of the file.
    /// - `codec`: The expected codec name.
    /// - `min_version`: The minimum supported version number.
    /// - `max_version`: The maximum supported version number.
    /// - `expected_id`: The expected object identifier for this file.
    /// - `expected_suffix`: The expected auxiliary suffix for this file.
    ///
    /// # Returns
    /// The actual version found, if a valid header is present that matches `codec`, `expected_id`, and
    /// `expected_suffix`, with a version satisfying `min_version <= actual <= max_version`.
    ///
    /// # Errors
    /// - `CorruptIndexError`: If the first four bytes are not [`CodecUtil::CODEC_MAGIC`], the codec does not match
    ///   `codec`, or `expected_id` or `expected_suffix` do not match.
    /// - `IndexFormatTooOldError`: If the actual version is less than `min_version`.
    /// - `IndexFormatTooNewError`: If the actual version is greater than `max_version`.
    /// - `IoError`: If there is an I/O error reading from the underlying medium.
    ///
    /// # See Also
    /// - [`write_index_header`]
    pub fn check_index_header(
        data_input: &mut impl DataInput,
        codec: &str,
        min_version: u32,
        max_version: u32,
        expected_id: &[u8],
        expected_suffix: &str,
    ) -> Result<u32, LuceneError> {
        let version = Self::check_header(data_input, codec, min_version, max_version)?;
        Self::check_index_header_id(data_input, expected_id)?;
        Self::check_index_header_suffix(data_input, expected_suffix)?;
        Ok(version)
    }

    /// Expert: verifies that the incoming [`IndexInput`] has an index header and that its segment ID
    /// matches the expected one, and then copies that index header into the provided [`DataOutput`].
    /// This is useful when building compound files.
    ///
    /// # Parameters
    /// - `input`: The input stream, positioned at the point where the index header was previously written.
    ///   Typically, this is located at the beginning of the file.
    /// - `output`: The output stream, where the header will be copied to.
    /// - `expected_id`: The expected segment ID.
    ///
    /// # Errors
    /// - `CorruptIndexError`: If the first four bytes are not [`CodecUtil::CODEC_MAGIC`] or if the `expected_id`
    ///   does not match.
    /// - `IoError`: If there is an I/O error reading from the underlying medium.
    ///
    /// # Internal
    /// This is an internal API and is intended for use within Lucene-like systems.
    pub fn verify_and_copy_index_header(
        data_in: &mut impl IndexInput,
        data_out: &mut impl DataOutput,
        expected_id: &[u8],
    ) -> Result<(), LuceneError> {
        if data_in.length() < (Self::footer_length() + Self::header_length("")) as u64 {
            return Err(LuceneError::corrupt_index(format!(
                "compound sub-files must have a valid codec header and footer: file is too small ({} bytes): (resource={})",
                data_in.length(),data_in
            )));
        }
        let actual_header = Self::read_be_int(data_in)?;
        if actual_header != CodecUtil::CODEC_MAGIC {
            return Err(LuceneError::corrupt_index(format!(
                "compound sub-files must have a valid codec header and footer: codec header mismatch: actual header={} vs expected header={}",
                actual_header, CodecUtil::CODEC_MAGIC
            )));
        }

        let codec = data_in.read_string()?;
        let version = Self::read_be_int(data_in)?;
        Self::check_index_header_id(data_in, expected_id)?;
        let suffix_length = data_in.read_byte()?;
        let mut suffix_bytes: Vec<u8> = vec![0u8; suffix_length as usize];
        data_in.read_bytes(&mut suffix_bytes, 0, suffix_length as u32)?;
        Self::write_be_int(data_out, CodecUtil::CODEC_MAGIC)?;
        data_out.write_string(&codec)?;
        Self::write_be_int(data_out, version)?;
        data_out.write_bytes_range(expected_id, 0, StringHelper::ID_LENGTH)?;
        data_out.write_byte(suffix_length)?;
        data_out.write_bytes_range(&suffix_bytes, 0, suffix_length as u32)?;
        Ok(())
    }
    /// Retrieves the full index header from the provided [`IndexInput`].
    ///
    /// # Errors
    /// - `CorruptIndexError`: If the file does not appear to be a valid index file.
    pub fn read_index_header(data_input: &mut impl IndexInput) -> Result<Vec<u8>, LuceneError> {
        let actual_header = Self::read_be_int(data_input)?;
        if actual_header != CodecUtil::CODEC_MAGIC {
            return Err(LuceneError::corrupt_index(format!(
                "codec header mismatch: actual header={} vs expected header={}",
                actual_header,
                CodecUtil::CODEC_MAGIC
            )));
        }
        let codec = data_input.read_string()?;
        Self::read_be_int(data_input)?;
        data_input.seek(data_input.get_file_pointer() + StringHelper::ID_LENGTH as u64)?;
        let suffix_length = data_input.read_byte()?;
        let bytes_len =
            (Self::header_length(&codec) + StringHelper::ID_LENGTH + 1 + suffix_length as u32)
                as usize;
        let mut bytes: Vec<u8> = vec![0u8; bytes_len];
        data_input.seek(0)?;
        data_input.read_bytes(&mut bytes, 0, bytes_len as u32)?;
        Ok(bytes)
    }

    /// Retrieves the full footer from the provided [`IndexInput`].
    ///
    /// # Errors
    /// - `CorruptIndexError`: If the file does not have a valid footer.
    pub fn read_footer(data_input: &mut impl IndexInput) -> Result<Vec<u8>, LuceneError> {
        if data_input.length() < Self::footer_length() as u64 {
            return Err(LuceneError::corrupt_index(format!(
                "misplaced codec footer (file truncated?): length={} but footerLength=={} (resource={})",
                data_input.length(),
                Self::footer_length(),
                data_input
            )));
        }
        data_input.seek(data_input.length() - Self::footer_length() as u64)?;
        Self::validate_footer(data_input)?;
        data_input.seek(data_input.length() - Self::footer_length() as u64)?;
        let mut bytes: Vec<u8> = vec![0u8; Self::footer_length() as usize];
        data_input.read_bytes(&mut bytes, 0, Self::footer_length())?;
        Ok(bytes)
    }
    /// Expert: reads and verifies the object ID of an index header.
    pub fn check_index_header_id(
        data_input: &mut impl DataInput,
        expected_id: &[u8],
    ) -> Result<(), LuceneError> {
        let mut id: Vec<u8> = vec![0u8; StringHelper::ID_LENGTH as usize];
        data_input.read_bytes(&mut id, 0, StringHelper::ID_LENGTH)?;
        if id != expected_id {
            return Err(LuceneError::corrupt_index(format!(
                "file mismatch, expected id={}, got={} (resource={})",
                StringHelper::id_to_string(Option::from(expected_id)),
                StringHelper::id_to_string(Option::from(&id[0..id.len()])),
                data_input
            )));
        }
        Ok(())
    }
    /// Expert: reads and verifies the suffix of an index header.
    pub fn check_index_header_suffix(
        data_input: &mut impl DataInput,
        expected_suffix: &str,
    ) -> Result<(), LuceneError> {
        let suffix_length = data_input.read_byte()?;
        let mut suffix: Vec<u8> = vec![0u8; suffix_length as usize];
        data_input.read_bytes(&mut suffix, 0, suffix_length as u32)?;
        let actual_suffix = String::from_utf8(suffix)?;
        if actual_suffix != expected_suffix {
            return Err(LuceneError::corrupt_index(format!(
                "file mismatch, expected suffix={}, got={} (resource={})",
                expected_suffix, actual_suffix, data_input
            )));
        }
        Ok(())
    }
    /// Writes a codec footer, which records both a checksum algorithm ID and a checksum.
    /// This footer can be parsed and validated with [`check_footer`].
    ///
    /// # Format
    /// `CodecFooter -> Magic, AlgorithmID, Checksum`
    ///
    /// - **Magic**:  
    ///   A `u32` (written using `write_int`). This identifies the start of the footer.  
    ///   It is always [`CodecUtil::FOOTER_MAGIC`].
    ///
    /// - **AlgorithmID**:  
    ///   A `u32` (written using `write_int`). This indicates the checksum algorithm used.  
    ///   Currently, this is always 0, for zlib-crc32.
    ///
    /// - **Checksum**:  
    ///   A `u64` (written using `write_long`). The actual checksum value for all previous bytes in the stream,  
    ///   including the bytes from Magic and AlgorithmID.
    ///
    /// # Parameters
    /// - `out`: The output stream to write to.
    ///
    /// # Errors
    /// - `IoError`: If there is an I/O error writing to the underlying medium.
    pub fn write_footer(out: &mut impl IndexOutput) -> Result<(), LuceneError> {
        Self::write_be_int(out, CodecUtil::FOOTER_MAGIC)?;
        Self::write_be_int(out, 0)?;
        Self::write_crc(out)?;
        Ok(())
    }

    /// Computes the length of a codec footer.
    ///
    /// # Returns
    /// The length of the entire codec footer.
    ///
    /// # See Also
    /// - [`write_footer`]
    pub fn footer_length() -> u32 {
        16
    }

    /// Validates the codec footer previously written by [`write_footer`].
    ///
    /// # Returns
    /// The actual checksum value.
    ///
    /// # Errors
    /// - `IoError`: If the footer is invalid, the checksum does not match, or the input is not properly
    ///   positioned before the footer at the end of the stream.
    pub fn check_footer(checksum_in: &mut impl ChecksumIndexInput) -> Result<u64, LuceneError> {
        Self::validate_footer(checksum_in)?;
        let actual_checksum = checksum_in.get_checksum();
        let expected_checksum = Self::read_crc(checksum_in)?;
        if actual_checksum != expected_checksum {
            return Err(LuceneError::corrupt_index(format!(
                "checksum failed (hardware problem?): expected={} but got={} (resource={})",
                expected_checksum, actual_checksum, checksum_in
            )));
        }
        Ok(actual_checksum)
    }

    /// Validates the codec footer previously written by [`write_footer`], optionally handling
    /// an unexpected exception that has already occurred.
    ///
    /// When a `prior_exception` is provided, this method will add a suppressed exception indicating
    /// whether the checksum for the stream passes, fails, or cannot be computed, and rethrow it.
    /// Otherwise, it behaves the same as [`check_footer`].
    ///
    /// # Parameters
    /// - `input`: The input stream to validate.
    /// - `prior_exception`: An optional previously occurred exception to handle.
    ///
    /// # Errors
    /// - `IoError`: If the footer is invalid, the checksum does not match, or the input is not
    ///   properly positioned before the footer at the end of the stream.
    /// - `PriorException`: If a prior exception is provided and rethrown after adding supplemental information.
    // TODO:Implemented a naive error propagation mechanism; we may use thiserror#[source] to standardize error nesting.
    pub fn check_footer_with_error(
        checksum_in: &mut impl ChecksumIndexInput,
        prior_error: &mut LuceneError,
    ) -> Result<(), LuceneError> {
        // If we have evidence of corruption then we return the corruption as the
        // main exception and the prior exception gets suppressed. Otherwise, we
        // return the prior exception with a suppressed exception that notifies
        // the user that checksums matched.
        let error = prior_error.to_string();
        let mut error_message: String = "".to_string();
        let remaining = checksum_in.length() - checksum_in.get_file_pointer();
        if remaining < Self::footer_length() as u64 {
            // corruption caused us to read into the checksum footer already: we can't proceed
            error_message = format!( "{} cause by checksum status indeterminate: remaining={}, ; please run check index for more details: {} ",
                                     error,
                                     remaining,
                                     checksum_in
            );
        } else {
            // otherwise, skip any unread bytes.
            let result =
                DataInput::skip_bytes(checksum_in, remaining - Self::footer_length() as u64);
            if result.is_err() {
                error_message = format!(
                    "{} cause by: checksum status indeterminate: unexpected exception: {} {}",
                    error,
                    checksum_in,
                    result.unwrap_err()
                );
            } else {
                // now check the footer
                let result = Self::check_footer(checksum_in);
                if result.is_err() {
                    error_message = format!(
                        "{} cause by checksum status indeterminate: unexpected exception: {} {} ",
                        error,
                        checksum_in,
                        result.unwrap_err(),
                    );
                } else {
                    let checksum = result?;
                    // If the index format is too old and no corruption, do not add checksums
                    // matching message since this may tend to unnecessarily alarm people who
                    // see "JVM bug" in their logs
                    if !matches!(prior_error, LuceneError::IndexFormatTooOld(_)) {
                        error_message= format!(
                            "checksum passed ({}). possibly transient resource issue, or a Lucene : {}, cause by: {}",
                            checksum,
                            checksum_in,
                            error
                        );
                    }
                }
            }
        }
        Err(LuceneError::corrupt_index(error_message))
    }

    /// Returns (but does not validate) the checksum previously written by [`check_footer`].
    ///
    /// # Returns
    /// The actual checksum value.
    ///
    /// # Errors
    /// - `IoError`: If the footer is invalid.
    pub fn retrieve_checksum(input: &mut impl IndexInput) -> Result<u64, LuceneError> {
        if input.length() < Self::footer_length() as u64 {
            return Err(LuceneError::corrupt_index(format!(
                "misplaced codec footer (file truncated?): length={} but footerLength=={} (resource={})",
                input.length(),
                Self::footer_length(),
                input
            )));
        }
        input.seek(input.length() - Self::footer_length() as u64)?;
        Self::validate_footer(input)?;
        Self::read_crc(input)
    }

    /// Returns (but does not validate) the checksum previously written by [`check_footer`].
    ///
    /// # Returns
    /// The actual checksum value.
    ///
    /// # Errors
    /// - `IoError`: If the footer is invalid.
    #[allow(unused)]
    fn retrieve_checksum_with_expected(
        input: &mut impl IndexInput,
        expected_length: u64,
    ) -> Result<u64, LuceneError> {
        if expected_length < Self::footer_length() as u64 {
            return Err(LuceneError::illegal_argument(
                "expectedLength cannot be less than the footer length".to_string(),
            ));
        }
        match input.length().cmp(&expected_length) {
            Ordering::Less => {
                return Err(LuceneError::corrupt_index(format!(
                    "truncated file: length={} but expected_length={} (resource={})",
                    input.length(),
                    expected_length,
                    input
                )));
            }
            Ordering::Greater => {
                return Err(LuceneError::corrupt_index(format!(
                    "file too long: length={} but expected_length={} (resource={})",
                    input.length(),
                    expected_length,
                    input
                )));
            }
            Ordering::Equal => {}
        }
        Self::retrieve_checksum(input)
    }

    fn validate_footer(input: &mut impl IndexInput) -> Result<(), LuceneError> {
        let remaining = input.length() - input.get_file_pointer();
        let expected = Self::footer_length();
        match remaining.cmp(&(expected as u64)) {
            Ordering::Less => {
                return Err(LuceneError::corrupt_index(format!(
                    "misplaced codec footer (file truncated?): remaining={}, expected={}, fp={} (resource={})",
                    remaining,
                    expected,
                    input.get_file_pointer(),
                    input
                )));
            }
            Ordering::Greater => {
                return Err(LuceneError::corrupt_index(format!(
                    "misplaced codec footer (file extended?): remaining={}, expected={}, fp={} (resource={})",
                    remaining,
                    expected,
                    input.get_file_pointer(),
                    input
                )));
            }
            Ordering::Equal => {}
        }
        let magic = Self::read_be_int(input)?;
        if magic != CodecUtil::FOOTER_MAGIC {
            return Err(LuceneError::corrupt_index(format!(
                "codec footer mismatch  (file truncated?): actual footer={} vs expected footer={} (resource={})",
                magic, CodecUtil::FOOTER_MAGIC, input
            )));
        }
        let algorithm_id = Self::read_be_int(input)?;
        if algorithm_id != 0 {
            return Err(LuceneError::corrupt_index(format!(
                "codec footer mismatch: unknown algorithmID={} (resource={})",
                algorithm_id, input
            )));
        }
        Ok(())
    }

    /// Clones the provided input, reads all bytes from the file, and calls [`check_footer`].
    ///
    /// # Notes
    /// This method may be slow, as it must process the entire file.  
    /// If you just need to extract the checksum value, call [`retrieve_checksum`].
    pub fn checksum_entire_file(input: &mut impl IndexInput) -> Result<u64, LuceneError> {
        let mut clone = input.clone();
        clone.seek(0)?;
        let mut checksum_in = BufferedChecksumIndexInput::new(clone);
        assert_eq!(checksum_in.get_file_pointer(), 0);
        if checksum_in.length() < Self::footer_length() as u64 {
            return Err(LuceneError::corrupt_index(format!(
                "misplaced codec footer (file truncated?): length={} but footerLength=={} (resource={})",
                checksum_in.length(),
                Self::footer_length(),
                input
            )));
        }
        let checksum_len = checksum_in.length();
        IndexInput::seek(
            &mut checksum_in,
            checksum_len - Self::footer_length() as u64,
        )?;
        Self::check_footer(&mut checksum_in)
    }

    /// Reads the CRC32 value as a 64-bit integer from the input.
    ///
    /// # Errors
    /// - `CorruptIndexError`: If the CRC is formatted incorrectly (wrong bits set).
    /// - `IoError`: If an I/O error occurs.
    pub fn read_crc(input: &mut impl IndexInput) -> Result<u64, LuceneError> {
        let value = Self::read_be_long(input)?;
        if value & 0xFFFFFFFF00000000 != 0 {
            return Err(LuceneError::corrupt_index(format!(
                "Illegal CRC-32 checksum: {} (resource={})",
                value, input
            )));
        }
        Ok(value)
    }

    /// Writes the CRC32 value as a 64-bit integer to the output.
    ///
    /// # Errors
    /// - `IllegalStateError`: If the CRC is formatted incorrectly (wrong bits set).
    /// - `IoError`: If an I/O error occurs.
    pub fn write_crc(out: &mut impl IndexOutput) -> Result<(), LuceneError> {
        let value = out.get_check_sum();
        if value as u64 & 0xFFFFFFFF00000000 != 0 {
            return Err(LuceneError::illegal_state(format!(
                "Illegal CRC-32 checksum: {} +  (resource={})",
                value, out
            )));
        }
        Self::write_be_long(out, value)
    }

    /// Writes an integer value to the header or footer in big-endian order.
    pub fn write_be_int(out: &mut impl DataOutput, i: u32) -> Result<(), LuceneError> {
        let bytes = [
            ((i >> 24) & 0xFF) as u8,
            ((i >> 16) & 0xFF) as u8,
            ((i >> 8) & 0xFF) as u8,
            (i & 0xFF) as u8,
        ];
        out.write_bytes_range(&bytes, 0, 4)?;
        Ok(())
    }
    /// Writes a long value to the header or footer in big-endian order.
    pub fn write_be_long(out: &mut impl DataOutput, i: i64) -> Result<(), LuceneError> {
        let bytes = [
            ((i >> 56) & 0xFF) as u8,
            ((i >> 48) & 0xFF) as u8,
            ((i >> 40) & 0xFF) as u8,
            ((i >> 32) & 0xFF) as u8,
            ((i >> 24) & 0xFF) as u8,
            ((i >> 16) & 0xFF) as u8,
            ((i >> 8) & 0xFF) as u8,
            (i & 0xFF) as u8,
        ];
        out.write_bytes_range(&bytes, 0, 8)?;
        Ok(())
    }
    /// Reads an integer value from the header or footer in big-endian order.
    pub fn read_be_int(out: &mut impl DataInput) -> Result<u32, LuceneError> {
        let byte1 = out.read_byte()? as i32;
        let byte2 = out.read_byte()? as i32;
        let byte3 = out.read_byte()? as i32;
        let byte4 = out.read_byte()? as i32;

        Ok(((byte1 << 24) | (byte2 << 16) | (byte3 << 8) | byte4) as u32)
    }

    /// Reads a long value from the header or footer in big-endian order.
    pub fn read_be_long(out: &mut impl DataInput) -> Result<u64, LuceneError> {
        let mut buffer = [0u8; 8];
        out.read_bytes(&mut buffer, 0, 8)?;
        Ok(u64::from_be_bytes(buffer))
    }
}
