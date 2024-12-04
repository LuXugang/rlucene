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
use crate::store::data_output::DataOutput;
use crate::store::index_input::IndexInput;
use crate::store::{BufferedChecksum, DataInput, IndexOutput};
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::check_sum_index_input::ChecksumIndexInput;
use crate::util::error::data_io_error_enum::DataIOError;
use crate::util::version::MIN_SUPPORTED_MAJOR;
use crate::util::{id_to_string, ID_LENGTH};

/** Constant to identify the start of a codec header. */
pub const CODEC_MAGIC: i32 = 0x3fd76c17;
/** Constant to identify the start of a codec footer. */
pub const FOOTER_MAGIC: i32 = !CODEC_MAGIC;
/**
 * Utility class for reading and writing versioned headers.
 *
 * <p>Writing codec headers is useful to ensure that a file is in the format you think it is.
 *
 * @lucene.experimental
 */
#[allow(dead_code)] // for quick search
struct CodecUtil;

pub fn write_header(
    out: &mut impl DataOutput,
    codec: &str,
    version: i32,
) -> Result<(), DataIOError> {
    let bytes = BytesRef::new_from_string(codec);
    if bytes.length != codec.len() as i32 || bytes.length >= 128 {
        return Err(DataIOError::illegal_argument(format!(
            "codec must be simple ASCII, less than 128 characters in length got {}",
            codec
        )));
    }
    write_be_int(out, CODEC_MAGIC)?;
    out.write_string(codec)?;
    write_be_int(out, version)?;
    Ok(())
}

/**
 * Writes a codec header for an index file, which records both a string to identify the format of
 * the file, a version number, and data to identify the file instance (ID and auxiliary suffix
 * such as generation).
*/
pub fn write_index_header(
    out: &mut impl DataOutput,
    codec: &str,
    version: i32,
    id: &[u8],
    suffix: &str,
) -> Result<(), DataIOError> {
    if id.len() != ID_LENGTH as usize {
        return Err(DataIOError::illegal_argument(format!(
            "Invalid id: {}",
            id_to_string(Option::from(id))
        )));
    }
    write_header(out, codec, version)?;
    out.write_bytes_range(id, 0, ID_LENGTH as usize)?;
    let suffix_bytes = BytesRef::new_from_string(suffix);
    if suffix_bytes.length != suffix.len() as i32 || suffix_bytes.length >= 256 {
        return Err(DataIOError::illegal_argument(format!(
            "suffix must be simple ASCII, less than 256 characters in length got {}",
            suffix
        )));
    }
    out.write_byte(suffix_bytes.length as u8)?;
    out.write_bytes_range(
        &suffix_bytes.bytes,
        suffix_bytes.offset as usize,
        suffix_bytes.length as usize,
    )?;
    Ok(())
}
/**
 * Computes the length of a codec header.
 *
 */
pub fn header_length(codec: &str) -> u32 {
    9 + codec.len() as u32
}
/**
 * Computes the length of an index header.
 */
pub fn index_header_length(codec: &str, suffix: &str) -> u32 {
    header_length(codec) + ID_LENGTH + 1 + (suffix.len() as u32)
}
/**
 * Reads and validates a header previously written with {@link #writeHeader(DataOutput, &str,
 * i32)}.
 *
 */
pub fn check_header(
    data_input: &mut impl DataInput,
    codec: &str,
    min_version: u32,
    max_version: u32,
) -> Result<i32, DataIOError> {
    let actual_header = read_be_int(data_input)?;
    if actual_header != CODEC_MAGIC {
        return Err(DataIOError::corrupt_index(format!(
            "codec header mismatch: actual header= {} vs expected header= {}",
            actual_header, CODEC_MAGIC
        )));
    }
    check_header_no_magic(data_input, codec, min_version, max_version)
}
/**
 * Like `checkHeader(DataInput,&str,i32,i32)} except this version assumes the first int
 * has already been read and validated from the input.
 */
pub fn check_header_no_magic(
    data_input: &mut impl DataInput,
    codec: &str,
    min_version: u32,
    max_version: u32,
) -> Result<i32, DataIOError> {
    let actual_codec = data_input.read_string()?;
    if actual_codec != codec {
        return Err(DataIOError::corrupt_index(format!(
            "codec mismatch: actual codec= {} vs expected codec= {}",
            actual_codec, codec
        )));
    }
    let actual_version = read_be_int(data_input)?;
    if (actual_version as u32) < min_version {
        return Err(DataIOError::index_format_too_old(format!("Format version is not supported (resource {}): {} (needs to be between {} and {}). This version of Lucene only supports indexes created with release {}.0 and later", data_input, actual_version, min_version, max_version, *MIN_SUPPORTED_MAJOR)));
    }
    if (actual_version as u32) > max_version {
        return Err(DataIOError::index_format_too_new(format!(
            "Format version is not supported (resource {}): {} (needs to be between {} and {}) ",
            data_input, actual_version, min_version, max_version
        )));
    }
    Ok(actual_version)
}
/**
 * Reads and validates a header previously written with `writeIndexHeader(DataOutput,
 * &str, i32, byte[], &str)`.
 * When reading a file, supply the expected `codec`, expected version range (`minVersion` to `maxVersion`), and object ID and suffix.
 */

/** Expert: just reads and verifies the object ID of an index header */

pub fn check_index_header(
    data_input: &mut impl DataInput,
    codec: &str,
    min_version: u32,
    max_version: u32,
    expected_id: &[u8],
    expected_suffix: &str,
) -> Result<i32, DataIOError> {
    let version = check_header(data_input, codec, min_version, max_version)?;
    check_index_header_id(data_input, expected_id)?;
    check_index_header_suffix(data_input, expected_suffix)?;
    Ok(version)
}

/**
 * Expert: verifies the incoming `IndexInput` has an index header and that its segment ID
 * matches the expected one, and then copies that index header into the provided
 * `DataOutput`. This is useful when building compound files.
 */
pub fn verify_and_copy_index_header(
    data_in: &mut impl IndexInput,
    data_out: &mut impl DataOutput,
    expected_id: &[u8],
) -> Result<(), DataIOError> {
    if data_in.length() < (footer_length() + header_length("")) as u64 {
        return Err(DataIOError::corrupt_index(format!(
            "compound sub-files must have a valid codec header and footer: file is too small ({} bytes): {}",
            data_in.length(),data_in
        )));
    }
    let actual_header = read_be_int(data_in)?;
    if actual_header != CODEC_MAGIC {
        return Err(DataIOError::corrupt_index(format!(
            "compound sub-files must have a valid codec header and footer: codec header mismatch: actual header= {} vs expected header= {}",
            actual_header, CODEC_MAGIC
        )));
    }

    let codec = data_in.read_string()?;
    let version = read_be_int(data_in)?;
    check_index_header_id(data_in, expected_id)?;
    let suffix_length = data_in.read_byte()?;
    let mut suffix_bytes: Vec<u8> = vec![0u8; suffix_length as usize];
    data_in.read_bytes(&mut suffix_bytes, 0, suffix_length as usize)?;
    write_be_int(data_out, CODEC_MAGIC)?;
    data_out.write_string(&codec)?;
    write_be_int(data_out, version)?;
    data_out.write_bytes_range(expected_id, 0, ID_LENGTH as usize)?;
    data_out.write_byte(suffix_length)?;
    data_out.write_bytes_range(&suffix_bytes, 0, suffix_length as usize)?;
    Ok(())
}
/**
 * Retrieves the full index header from the provided `IndexInput`. This throws
 * `Corrupt Error` if this file does not appear to be an index file.
 */
pub fn read_index_header(data_input: &mut impl IndexInput) -> Result<Vec<u8>, DataIOError> {
    let actual_header = read_be_int(data_input)?;
    if actual_header != CODEC_MAGIC {
        return Err(DataIOError::corrupt_index(format!(
            "codec header mismatch: actual header= {} vs expected header= {}",
            actual_header, CODEC_MAGIC
        )));
    }
    let codec = data_input.read_string()?;
    read_be_int(data_input)?;
    data_input.seek(data_input.get_file_pointer() + ID_LENGTH as u64)?;
    let suffix_length = data_input.read_byte()?;
    let bytes_len = (header_length(&codec) + ID_LENGTH + 1 + suffix_length as u32) as usize;
    let mut bytes: Vec<u8> = vec![0u8; bytes_len];
    data_input.seek(0)?;
    data_input.read_bytes(&mut bytes, 0, bytes_len)?;
    Ok(bytes)
}

/**
 * Retrieves the full footer from the provided `IndexInput`. This throws `Corrupt Error` if this file does not have a valid footer.
 */
pub fn read_footer(data_input: &mut impl IndexInput) -> Result<Vec<u8>, DataIOError> {
    if data_input.length() < footer_length() as u64 {
        return Err(DataIOError::corrupt_index(format!(
            "misplaced codec footer (file truncated?): length= {} but footerLength== {}: {}",
            data_input.length(),
            footer_length(),
            data_input
        )));
    }
    data_input.seek(data_input.length() - footer_length() as u64)?;
    validate_footer(data_input)?;
    data_input.seek(data_input.length() - footer_length() as u64)?;
    let mut bytes: Vec<u8> = vec![0u8; footer_length() as usize];
    data_input.read_bytes(&mut bytes, 0, footer_length() as usize)?;
    Ok(bytes)
}

pub fn check_index_header_id(
    data_input: &mut impl DataInput,
    expected_id: &[u8],
) -> Result<(), DataIOError> {
    let mut id: Vec<u8> = vec![0u8; ID_LENGTH as usize];
    data_input.read_bytes(&mut id, 0, ID_LENGTH as usize)?;
    if id != expected_id {
        return Err(DataIOError::corrupt_index(format!(
            "file mismatch, expected id={}, got={}: {}",
            id_to_string(Option::from(expected_id)),
            id_to_string(Option::from(&id[0..id.len()])),
            data_input
        )));
    }
    Ok(())
}
/** Expert: just reads and verifies the suffix of an index header */
pub fn check_index_header_suffix(
    data_input: &mut impl DataInput,
    expected_suffix: &str,
) -> Result<(), DataIOError> {
    let suffix_length = data_input.read_byte()?;
    let mut suffix: Vec<u8> = vec![0u8; suffix_length as usize];
    data_input.read_bytes(&mut suffix, 0, suffix_length as usize)?;
    let actual_suffix = String::from_utf8(suffix)?;
    if actual_suffix != expected_suffix {
        return Err(DataIOError::corrupt_index(format!(
            "file mismatch, expected suffix= {}, got= {}: {}",
            expected_suffix, actual_suffix, data_input
        )));
    }
    Ok(())
}
/**
 * Writes a codec footer, which records both a checksum algorithm ID and a checksum. This footer
 * can be parsed and validated with {@link #checkFooter(ChecksumIndexInput) checkFooter()}.
 */
pub fn write_footer(out: &mut impl IndexOutput) -> Result<(), DataIOError> {
    write_be_int(out, FOOTER_MAGIC)?;
    write_be_int(out, 0)?;
    write_crc(out)?;
    Ok(())
}

/**
 * Computes the length of a codec footer.
 */
pub fn footer_length() -> u32 {
    16
}

/**
 * Validates the codec footer previously written by {@link #writeFooter}.
 *
 */
pub fn check_footer(checksum_in:&mut  impl ChecksumIndexInput) -> Result<u64, DataIOError> {
    validate_footer(checksum_in)?;
    let actual_checksum = checksum_in.get_checksum();
    let expected_checksum = read_crc(checksum_in)?;
    if actual_checksum != expected_checksum {
        return Err(DataIOError::corrupt_index(format!(
            "checksum failed (hardware problem?): expected= {} but got= {}: {}",
            expected_checksum, actual_checksum, checksum_in
        )));
    }
    Ok(actual_checksum)
}

/**
 * Validates the codec footer previously written by `writeFooter`, optionally passing an
 * unexpected exception that has already occurred.
 *
 * When a `prior error` is provided, this method will add a suppressed exception
 * indicating whether the checksum for the stream passes, fails, or cannot be computed, and
 * rethrow it. Otherwise, it behaves the same as `checkFooter(ChecksumIndexInput)`.
 *
 */
pub fn check_footer_with_error(checksum_in: &mut impl ChecksumIndexInput, prior_error: &mut DataIOError) -> Result<(), DataIOError> {
    // If we have evidence of corruption then we return the corruption as the
    // main exception and the prior exception gets suppressed. Otherwise, we
    // return the prior exception with a suppressed exception that notifies
    // the user that checksums matched.
    let error = prior_error.to_string();
    let mut error_message:String = "".to_string();
    let remaining = checksum_in.length() - checksum_in.get_file_pointer();
    if remaining < footer_length() as u64 {
        // corruption caused us to read into the checksum footer already: we can't proceed
        error_message = format!( "checksum status indeterminate: remaining={}, ; please run checkindex for more details: {} {}",
                                 checksum_in,
                                 error,
                                 remaining,
        );
    }else {
        // otherwise, skip any unread bytes.
        let result = DataInput::skip_bytes(checksum_in,remaining - footer_length() as u64);
        if result.is_err() {
            error_message = format!(
                "checksum status indeterminate: unexpected exception: {} {} {}",
                checksum_in,
                result.unwrap_err(),
                error,
           );
        }else {
            // now check the footer
            let result = check_footer(checksum_in);
            if result.is_err() {
                error_message = format!(
                    "checksum status indeterminate: unexpected exception: {} {} {}",
                    checksum_in,
                    result.unwrap_err(),
                    error,
                );
            }else {
                let checksum = result?;
                // If the index format is too old and no corruption, do not add checksums
                // matching message since this may tend to unnecessarily alarm people who
                // see "JVM bug" in their logs
                if matches!(prior_error, DataIOError::IndexFormatTooOld(_) ){
                    error_message= format!(
                        "{}, checksum passed ({}). possibly transient resource issue, or a Lucene : {}",
                        error,
                        checksum,
                        checksum_in
                    );
                }
            }
        }
    }
    Err(DataIOError::corrupt_index(error_message))
    
}

/**
 * Returns (but does not validate) the checksum previously written by `checkFooter`.
 */
pub fn retrieve_checksum(input: &mut impl IndexInput) -> Result<u64, DataIOError> {
    if input.length() < footer_length() as u64 {
        return Err(DataIOError::corrupt_index(format!(
            "misplaced codec footer (file truncated?): length= {} but footerLength== {}: {}",
            input.length(),
            footer_length(),
            input
        )));
    }
    input.seek(input.length() - footer_length() as u64)?;
    validate_footer(input)?;
    read_crc(input)
}

/**
 * Returns (but does not validate) the checksum previously written by `checkFooter`.
 */
fn retrieve_checksum_with_expected(
    input: &mut impl IndexInput,
    expected_length: u64,
) -> Result<u64, DataIOError> {
    if expected_length < footer_length() as u64 {
        return Err(DataIOError::illegal_argument(
            "expectedLength cannot be less than the footer length".to_string(),
        ));
    }
    if input.length() < expected_length {
        return Err(DataIOError::corrupt_index(format!(
            "truncated file: length= {} but expected_length= {}: {}",
            input.length(),
            expected_length,
            input
        )));
    } else if input.length() > expected_length {
        return Err(DataIOError::corrupt_index(format!(
            "file too long: length= {} but expected_length= {}: {}",
            input.length(),
            expected_length,
            input
        )));
    }
    retrieve_checksum(input)
}

fn validate_footer(input: &mut impl IndexInput) -> Result<(), DataIOError> {
    let remaining = input.length() - input.get_file_pointer();
    let expected = footer_length();
    if remaining < expected as u64 {
        return Err(DataIOError::corrupt_index(format!(
            "misplaced codec footer (file truncated?): remaining= {}, expected= {}, fp={}: {}",
            remaining,
            expected,
            input.get_file_pointer(),
            input
        )));
    } else if remaining > expected as u64 {
        return Err(DataIOError::corrupt_index(format!(
            "misplaced codec footer (file extended?): remaining= {}, expected= {}, fp={}: {}",
            remaining,
            expected,
            input.get_file_pointer(),
            input
        )));
    }
    let magic = read_be_int(input)?;
    if magic != FOOTER_MAGIC {
        return Err(DataIOError::corrupt_index(format!(
            "codec footer mismatch  (file truncated?): actual footer= {} vs expected footer= {}: {}",
            magic, FOOTER_MAGIC, input
        )));
    }
    let algorithm_id = read_be_int(input)?;
    if algorithm_id != 0 {
        return Err(DataIOError::corrupt_index(format!(
            "codec footer mismatch: unknown algorithmID= {}: {}",
            algorithm_id, input
        )));
    }
    Ok(())
}

/**
 * Clones the provided input, reads all bytes from the file, and calls `checkFooter`
 *
 * Note that this method may be slow, as it must process the entire file. If you just need to
 * extract the checksum value, call `retrieveChecksum`.
*/
pub fn check_sum_entire_file(input: &mut impl IndexInput) -> Result<u64, DataIOError> {
    let mut clone = input.clone();
    clone.seek(0)?;
    let mut checksum_in = BufferedChecksumIndexInput::new(clone);
    assert_eq!(checksum_in.get_file_pointer(), 0);
    if checksum_in.length() < footer_length() as u64 {
        return Err(DataIOError::corrupt_index(format!(
            "misplaced codec footer (file truncated?): length={} but footerLength=={}: {}",
            checksum_in.length(),
            footer_length(),
            input
        )));
    }
    let checksum_len = checksum_in.length();
    IndexInput::seek(&mut checksum_in,checksum_len - footer_length() as u64)?;
    check_footer(&mut checksum_in)
}

/**
 * Reads CRC32 value as a 64-bit long from the input.
 */
pub fn read_crc(input: &mut impl IndexInput) -> Result<u64, DataIOError> {
    let value = read_be_long(input)?;
    if value & 0xFFFFFFFF00000000 != 0 {
        return Err(DataIOError::corrupt_index(format!(
            "Illegal CRC-32 checksum: {}: {}",
            value, input
        )));
    }
    Ok(value)
}

/**
 * Writes CRC32 value as a 64-bit long to the output.
 */
pub fn write_crc(out: &mut impl IndexOutput) -> Result<(), DataIOError> {
    let value = out.get_check_sum();
    if value as u64 & 0xFFFFFFFF00000000 != 0 {
        return Err(DataIOError::illegal_state(format!(
            "Illegal CRC-32 checksum: {} +  (resource= {})",
            value, out
        )));
    }
    write_be_long(out, value)
}

/** write int value on header / footer with big endian order */
pub fn write_be_int(out: &mut impl DataOutput, i: i32) -> Result<(), DataIOError> {
    let bytes = [
        ((i >> 24) & 0xFF) as u8,
        ((i >> 16) & 0xFF) as u8,
        ((i >> 8) & 0xFF) as u8,
        (i & 0xFF) as u8,
    ];
    out.write_bytes_range(&bytes, 0, 4)?;
    Ok(())
}
/** write long value on header / footer with big endian order */
pub fn write_be_long(out: &mut impl DataOutput, i: i64) -> Result<(), DataIOError> {
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
/** read int value from header / footer with big endian order */
pub fn read_be_int(out: &mut impl DataInput) -> Result<i32, DataIOError> {
    let byte1 = out.read_byte()? as i32;
    let byte2 = out.read_byte()? as i32;
    let byte3 = out.read_byte()? as i32;
    let byte4 = out.read_byte()? as i32;

    Ok((byte1 << 24) | (byte2 << 16) | (byte3 << 8) | byte4)
}

/** read long value from header / footer with big endian order */
pub fn read_be_long(out: &mut impl DataInput) -> Result<u64, DataIOError> {
    let mut buffer = [0u8; 8];
    out.read_bytes(&mut buffer, 0, 8)?;
    Ok(u64::from_be_bytes(buffer))
}
