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
use crate::store::DataInput;
use crate::util::error::data_io_error_enum::DataIOError;
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
    todo!()
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
        return Err(DataIOError::index_format(format!(
            "Format version is not supported (resource {}): {} (needs to be between {} and {}) ",
            data_input, actual_version, min_version, max_version
        )));
    }
    if (actual_version as u32) > max_version {
        return Err(DataIOError::index_format(format!("Format version is not supported (resource {}): {} (needs to be between {} and {}). This version of Lucene only supports indexes created with release ", data_input, actual_version, min_version, max_version)));
    }
    todo!()
}

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
pub fn read_be_int(out: &mut impl DataInput) -> Result<i32, DataIOError> {
    let byte1 = out.read_byte()? as i32;
    let byte2 = out.read_byte()? as i32;
    let byte3 = out.read_byte()? as i32;
    let byte4 = out.read_byte()? as i32;

    Ok((byte1 << 24) | (byte2 << 16) | (byte3 << 8) | byte4)
}
