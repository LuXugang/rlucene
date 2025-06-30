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
use crate::store::{DataOutput, IndexInput};
use crate::util::error::lucene_error::Result;
use crate::util::group_vint_util::GroupVIntUtil;
/// Utility struct to encode/decode postings block.
#[allow(unused)]
pub(crate) struct PostingsUtil;
#[allow(unused)]
impl PostingsUtil {
    /// Read values that have been written using variable-length encoding and
    /// group-varint encoding instead of bit-packing.
    pub(crate) fn read_vint_block(
        doc_in: &mut impl IndexInput,
        doc_buffer: &mut [i32],
        freq_buffer: &mut [i32],
        num: i32,
        index_has_freq: bool,
        decode_freq: bool,
    ) -> Result<()> {
        GroupVIntUtil::read_group_vints_i32(doc_in, doc_buffer, num)?;
        let num = num as usize;
        if index_has_freq && decode_freq {
            for i in 0..num {
                freq_buffer[i] = doc_buffer[i] & 0x01;
                doc_buffer[i] = ((doc_buffer[i] as u32) >> 1) as i32;
                if freq_buffer[i] == 0 {
                    freq_buffer[i] = doc_in.read_vint()?;
                }
            }
        } else if index_has_freq {
            for val in doc_buffer.iter_mut().take(num) {
                *val = ((*val as u32) >> 1) as i32;
            }
        }
        Ok(())
    }
    /// Write freq buffer with variable-length encoding and doc buffer with
    /// group-varint encoding.
    pub(crate) fn write_vint_block(
        doc_out: &mut impl DataOutput,
        doc_buffer: &mut [i32],
        freq_buffer: &[i32],
        num: i32,
        write_freqs: bool,
    ) -> Result<()> {
        if write_freqs {
            for i in 0..num as usize {
                doc_buffer[i] = (doc_buffer[i] << 1) | if freq_buffer[i] == 1 { 1 } else { 0 };
            }
        }
        doc_out.write_group_vints_i32(doc_buffer, num)?;
        let num = num as usize;

        if write_freqs {
            for &freq in freq_buffer.iter().take(num) {
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

    use rand::Rng;

    use crate::codecs::lucene101::for_util::ForUtil;
    use crate::codecs::lucene101::postings_util::PostingsUtil;
    use crate::store::directory::Directory;
    use crate::store::IOContext;
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::util::error::lucene_error::Result;

    // checks for bug described in https://github.com/apache/lucene/issues/13373
    #[allow(dead_code)] // for quick search
    struct TestPostingsUtil;
    #[test]
    fn test_integer_overflow() -> Result<()> {
        let mut random = random();
        let random_size1: usize = random.random_range(1..3);
        let random_size2: usize = random.random_range(4..=ForUtil::BLOCK_SIZE);
        do_test_integer_overflow(&mut random, random_size1)?;
        do_test_integer_overflow(&mut random, random_size2)?;
        Ok(())
    }
    fn do_test_integer_overflow<R: Rng + ?Sized>(random: &mut R, size: usize) -> Result<()> {
        let mut doc_delta_buffer = vec![0i32; size];
        let freq_buffer = vec![0i32; size];

        let delta = 1 << 30;
        doc_delta_buffer[0] = delta;

        // TODO: ByteBuffersDirectory not Implemented
        let mut dir = new_directory(random)?;
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

        let mut restored_docs = vec![0i32; size];
        let mut restored_freqs = vec![0i32; size];

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
