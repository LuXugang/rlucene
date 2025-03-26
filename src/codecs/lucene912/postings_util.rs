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
/// Utility class to encode/decode postings block.
use crate::store::{DataOutput, IndexInput};
use crate::util::error::lucene_error::Result;
use crate::util::group_vint_util::GroupVIntUtil;

pub(crate) struct PostingsUtil;

impl PostingsUtil {
    /// Read values that have been written using variable-length encoding and group-varint encoding
    /// instead of bit-packing.
    pub(crate) fn read_vint_block<I: IndexInput>(
        doc_in: &mut I,
        doc_buffer: &mut [i64],
        freq_buffer: &mut [i64],
        num: i32,
        index_has_freq: bool,
        decode_freq: bool,
    ) -> Result<()> {
        GroupVIntUtil::read_group_vints(doc_in, doc_buffer, num)?;
        let num = num as usize;
        if index_has_freq && decode_freq {
            for i in 0..num {
                freq_buffer[i] = doc_buffer[i] & 0x01;
                doc_buffer[i] >>= 1;
                if freq_buffer[i] == 0 {
                    freq_buffer[i] = doc_in.read_vint()? as i64;
                }
            }
        } else if index_has_freq {
            for val in doc_buffer.iter_mut().take(num) {
                *val >>= 1;
            }
        }
        Ok(())
    }
    /// Write freq buffer with variable-length encoding and doc buffer with group-varint encoding.
    pub(crate) fn write_vint_block<O: DataOutput>(
        doc_out: &mut O,
        doc_buffer: &mut [i64],
        freq_buffer: &[i64],
        num: i32,
        write_freqs: bool,
    ) -> Result<()> {
        if write_freqs {
            for i in 0..num as usize {
                doc_buffer[i] = (doc_buffer[i] << 1) | if freq_buffer[i] == 1 { 1 } else { 0 };
            }
        }
        doc_out.write_group_vints(doc_buffer, num)?;
        let num = num as usize;

        if write_freqs {
            for i in 0..num {
                let freq = freq_buffer[i] as i32;
                if freq != 1 {
                    doc_out.write_vint(freq)?;
                }
            }
        }

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::codecs::lucene912::for_util::ForUtil;
    use crate::codecs::lucene912::postings_util::PostingsUtil;
    use crate::store::directory::Directory;
    use crate::store::IOContext;
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::util::error::lucene_error::Result;
    use rand::Rng;
    // checks for bug described in https://github.com/apache/lucene/issues/13373
    #[allow(dead_code)] // for quick search
    struct TestPostingsUtil;
    #[test]
    fn test_integer_overflow() -> Result<()> {
        let mut random = random();
        let size = random.random_range(1..=ForUtil::BLOCK_SIZE);
        let mut doc_delta_buffer = vec![0i64; size];
        let freq_buffer = vec![0i64; size];

        let delta = 1 << 30;
        doc_delta_buffer[0] = delta;

        /// TODO: ByteBuffersDirectory not Implemented
        let mut dir = new_directory(&mut random)?;
        {
            let mut out = dir.create_output("test", &IOContext::default_io_context()?)?;
            PostingsUtil::write_vint_block(
                &mut out,
                &mut doc_delta_buffer,
                &freq_buffer,
                size as i32,
                true,
            )?;
        }

        let mut restored_docs = vec![0i64; size];
        let mut restored_freqs = vec![0i64; size];

        {
            let mut input = dir.open_input("test", &IOContext::default_io_context()?)?;
            PostingsUtil::read_vint_block(
                &mut input,
                &mut restored_docs,
                &mut restored_freqs,
                size as i32,
                true,
                true,
            )?;
        }

        assert_eq!(delta, restored_docs[0]);

        Ok(())
    }
}
