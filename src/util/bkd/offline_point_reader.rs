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
use std::rc::Rc;

use crate::codecs::CodecUtil;
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::directory::Directory;
use crate::store::{DataInput, IOContext, IndexInput};
use crate::util::bit_util::BitUtil;
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::point_reader::PointReader;
use crate::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::util::error::lucene_error::{LuceneError, Result};

pub struct OfflinePointReader<I>
where
    I: IndexInput,
{
    count_left: i64,
    // TODO:Perhaps we can use an enum here to encapsulate
    // BufferedChecksumIndexInput and other types of Input
    input: Option<I>,
    check_sum_input: Option<BufferedChecksumIndexInput<I>>,
    offset: i32,
    checked: bool,
    config: Rc<BKDConfig>,
    points_in_buffer: i32,
    max_point_on_heap: i32,
    // File name we are reading
    #[allow(unused)]
    name: String,
    pub(crate) point_value: PointValueEnum,
}

impl<I> OfflinePointReader<I>
where
    I: IndexInput,
{
    pub fn new<D>(
        config: Rc<BKDConfig>,
        temp_dir: &mut D,
        temp_file_name: &str,
        start: i64,
        length: i64,
        reusable_buffer: Vec<u8>,
    ) -> Result<Self>
    where
        D: Directory<IndexInputType = I>,
    {
        let bytes_per_doc = config.bytes_per_doc() as i64;
        let footer_length = CodecUtil::footer_length() as i64;
        let file_length = temp_dir.file_length(temp_file_name)?;
        if ((start + length) * bytes_per_doc + footer_length) > file_length {
            return Err(LuceneError::illegal_argument(format!(
                "requested slice is beyond the length of this file: start={} length={} bytesPerDoc={} fileLength={} tempFileName={}",
                start,
                length,
                config.bytes_per_doc(),
                file_length,
                temp_file_name
            )));
        }
        let reusable_buffer_len = reusable_buffer.len();
        if reusable_buffer_len < config.bytes_per_doc() as usize {
            return Err(LuceneError::illegal_argument(format!(
                "Length of reusableBuffer must be bigger than {}",
                config.bytes_per_doc()
            )));
        }

        debug_assert!(reusable_buffer_len <= i32::MAX as usize);
        let max_point_on_heap = reusable_buffer_len as i32 / config.bytes_per_doc();
        let name = temp_file_name.to_string();
        let seek_fp = start * bytes_per_doc;
        let (check_sum_input, input) =
            if start == 0 && (length * bytes_per_doc == file_length - footer_length) {
                let mut check_sum_input = temp_dir.open_checksum_input(temp_file_name)?;
                IndexInput::seek(&mut check_sum_input, seek_fp)?;
                (Some(check_sum_input), None)
            } else {
                let mut input =
                    temp_dir.open_input(temp_file_name, &IOContext::read_once_io_context()?)?;
                input.seek(seek_fp)?;
                (None, Some(input))
            };

        let count_left = length;
        let point_value = PointValueEnum::Offline(OfflinePointValue::new(&config, reusable_buffer));

        Ok(OfflinePointReader {
            count_left,
            input,
            check_sum_input,
            offset: 0,
            checked: false,
            config,
            points_in_buffer: 0,
            max_point_on_heap,
            name,
            point_value,
        })
    }
}
impl<I> PointReader for OfflinePointReader<I>
where
    I: IndexInput,
{
    fn next(&mut self) -> Result<bool> {
        let bytes_per_doc = self.config.bytes_per_doc();
        if self.points_in_buffer == 0 {
            if self.count_left == 0 {
                return Ok(false);
            }
            let read_len;
            if self.count_left > self.max_point_on_heap as i64 {
                read_len = self.max_point_on_heap * bytes_per_doc;
                match &mut self.point_value {
                    PointValueEnum::Offline(offline) => {
                        if self.check_sum_input.is_some() {
                            self.check_sum_input.as_mut().unwrap().read_bytes(
                                &mut offline.value[0..read_len as usize],
                                0,
                                read_len,
                            )?;
                        } else {
                            self.input.as_mut().unwrap().read_bytes(
                                &mut offline.value[0..read_len as usize],
                                0,
                                read_len,
                            )?;
                        }
                    },
                    _ => {
                        debug_assert!(false, "PointValueEnum must be Offline");
                    },
                }

                self.points_in_buffer = self.max_point_on_heap - 1;
                self.count_left -= self.max_point_on_heap as i64;
            } else {
                read_len = self.count_left as i32 * bytes_per_doc;
                match &mut self.point_value {
                    PointValueEnum::Offline(offline) => {
                        if self.check_sum_input.is_some() {
                            self.check_sum_input.as_mut().unwrap().read_bytes(
                                &mut offline.value[0..read_len as usize],
                                0,
                                read_len,
                            )?;
                        } else {
                            self.input.as_mut().unwrap().read_bytes(
                                &mut offline.value[0..read_len as usize],
                                0,
                                read_len,
                            )?;
                        }
                    },
                    _ => {
                        debug_assert!(false, "PointValueEnum must be Offline");
                    },
                }
                self.points_in_buffer = (self.count_left - 1) as i32;
                self.count_left = 0;
            }
            self.offset = 0;
        } else {
            self.points_in_buffer -= 1;
            self.offset += bytes_per_doc;
        }
        Ok(true)
    }

    fn point_value(&mut self) -> &PointValueEnum {
        match &mut self.point_value {
            PointValueEnum::Offline(offline) => {
                offline.set_offset(self.offset);
            },
            _ => {
                debug_assert!(false, "PointValueEnum must be Offline");
            },
        }
        &self.point_value
    }
}
impl<I> Drop for OfflinePointReader<I>
where
    I: IndexInput,
{
    fn drop(&mut self) {
        if self.count_left == 0 && self.check_sum_input.is_some() && !self.checked {
            self.checked = true;
            match CodecUtil::check_footer(self.check_sum_input.as_mut().unwrap()) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Failed to check footer: {:?}", e);
                },
            }
        }
    }
}

/// Reusable implementation for a point value offline.
#[allow(unused)]
pub(crate) struct OfflinePointValue {
    pub(crate) offset: i32,
    pub(crate) value: Vec<u8>,
    pub(crate) packed_value_length: i32,
    pub(crate) packed_value_doc_id_length: i32,
}
impl OfflinePointValue {
    pub fn new(config: &BKDConfig, value: Vec<u8>) -> Self {
        Self {
            offset: 0,
            value,
            packed_value_length: config.packed_bytes_length(),
            packed_value_doc_id_length: config.bytes_per_doc(),
        }
    }
}
impl PointValue for OfflinePointValue {
    fn set_offset(&mut self, offset: i32) {
        self.offset = offset;
    }

    fn packed_value(&self) -> (&[u8], i32, i32) {
        (&self.value, self.offset, self.packed_value_length)
    }

    fn doc_id(&self) -> i32 {
        let position = (self.offset + self.packed_value_length) as usize;
        BitUtil::get_i32_be(&self.value[position..], 0)
    }

    fn packed_value_doc_id_bytes(&self) -> (&[u8], i32, i32) {
        (&self.value, self.offset, self.packed_value_doc_id_length)
    }
}
