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
use crate::core::codecs::lucene101::for_util::ForUtil;
use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
use crate::core::store::{DataInput, DataOutput, IndexInput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::long_heap::LongHeap;
use crate::core::util::packed::PackedInts;
/// Utility struct to encode sequences of 128 small positive integers.
pub(crate) struct PForUtil {
    for_util: ForUtil,
}
impl PForUtil {
    pub(crate) const MAX_EXCEPTIONS: usize = 7;

    pub(crate) fn new() -> Self {
        Self {
            for_util: ForUtil::new(),
        }
    }

    pub(crate) fn all_equal(arr: &[i32]) -> bool {
        arr.iter().skip(1).all(|&v| v == arr[0])
    }
    /// Encode 128 integers from `ints` into `out`.
    pub(crate) fn encode<O: DataOutput>(&mut self, ints: &mut [i32], out: &mut O) -> Result<()> {
        let mut top = LongHeap::new(Self::MAX_EXCEPTIONS as i32 + 1)?;
        for &v in &ints[..=Self::MAX_EXCEPTIONS] {
            top.push(v as i64);
        }

        let mut top_value = top.top();
        for &v in &ints[Self::MAX_EXCEPTIONS + 1..ForUtil::BLOCK_SIZE] {
            if v as i64 > top_value {
                top_value = top.update_top(v as i64);
            }
        }

        let mut max = 0;
        for i in 1..=top.size() as usize {
            max = max.max(top.get(i));
        }

        let max_bits_required = PackedInts::bits_required(max)?;
        // We store the patch on a byte, so we can't decrease the number of bits
        // required by more than 8
        let patched_bits_required =
            std::cmp::max(PackedInts::bits_required(top_value)?, max_bits_required - 8);

        let mut num_exceptions = 0;
        let max_unpatched_value = (1i64 << patched_bits_required) - 1;
        for i in 2..=top.size() as usize {
            if top.get(i) > max_unpatched_value {
                num_exceptions += 1;
            }
        }

        let mut exceptions = vec![0u8; num_exceptions * 2];
        if num_exceptions > 0 {
            let mut exception_count = 0;
            for (i, v) in ints.iter_mut().enumerate().take(ForUtil::BLOCK_SIZE) {
                if *v as i64 > max_unpatched_value {
                    exceptions[exception_count * 2] = i as u8;
                    exceptions[exception_count * 2 + 1] =
                        (*v as u64 >> patched_bits_required) as u8;
                    *v = ((*v as i64) & max_unpatched_value) as i32;
                    exception_count += 1;
                }
            }
            debug_assert!(exception_count == num_exceptions)
        }

        if Self::all_equal(ints) && max_bits_required <= 8 {
            for i in 0..num_exceptions {
                exceptions[2 * i + 1] =
                    ((exceptions[2 * i + 1] as i32) << patched_bits_required) as u8;
            }
            out.write_byte((num_exceptions << 5) as u8)?;
            out.write_vint(ints[0])?;
        } else {
            let token = (num_exceptions << 5) | (patched_bits_required as usize);
            out.write_byte(token as u8)?;
            self.for_util.encode(ints, patched_bits_required, out)?;
        }

        let len = exceptions.len();
        debug_assert!(len <= i32::MAX as usize);
        out.write_bytes_with_len(&exceptions, len as i32)?;
        Ok(())
    }

    /// Decode 128 integers into `ints`.
    pub(crate) fn decode<I: IndexInput>(
        &mut self,
        pdu: &mut PostingDecodingUtil<I>,
        ints: &mut [i32],
    ) -> Result<()> {
        let token = pdu.input.read_byte()?;
        let bits_per_value = token & 0x1f;

        if bits_per_value == 0 {
            let value = pdu.input.read_vint()?;
            ints[..ForUtil::BLOCK_SIZE].fill(value);
        } else {
            self.for_util.decode(bits_per_value as i32, pdu, ints)?;
        }
        let num_exceptions = (token >> 5) as usize;
        let input = &mut pdu.input;
        for _ in 0..num_exceptions {
            let index = input.read_byte()? as usize;
            let patch = input.read_byte()? as i32;
            ints[index] |= patch << bits_per_value;
        }

        Ok(())
    }

    /// Skip 128 integers.
    pub(crate) fn skip<I: DataInput>(input: &mut I) -> Result<()> {
        let token = input.read_byte()? as i32;
        let bits_per_value = token & 0x1f;
        let num_exceptions = (token as u32 >> 5) as i32;

        if bits_per_value == 0 {
            input.read_vlong()?;
            input.skip_bytes((num_exceptions << 1) as i64)?;
        } else {
            let skip = (ForUtil::num_bytes(bits_per_value)) + (num_exceptions << 1);
            input.skip_bytes(skip as i64)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::core::codecs::lucene101::for_util::ForUtil;
    use crate::core::codecs::lucene101::pfor_util::PForUtil;
    use crate::core::internal::vectorization::posting_decoding_util::PostingDecodingUtil;
    use crate::core::store::directory::Directory;
    use crate::core::store::{IOContext, IndexInput, IndexOutput};
    use crate::core::util::error::lucene_error::Result;
    use crate::core::util::packed::PackedInts;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{new_directory, random};
    use crate::test::util::test_util::TestUtil;
    #[allow(dead_code)] // for quick search
    struct TestPForUtil;

    #[test]
    fn test_encode_decode() -> Result<()> {
        let mut random = random();
        let iterations = random.random_range(50..1000);
        let values = create_test_data(iterations, 31, &mut random);

        // TODO: ByteBuffersDirectory not Implemented
        let dir = new_directory(&mut random)?;
        let end_pointer = encode_test_data(iterations, &values, &dir)?;

        let input = dir.open_input("test.bin", &IOContext::read_once_io_context()?)?;
        let mut pdu = PostingDecodingUtil::new(input);
        let mut pfor_util = PForUtil::new();

        for i in 0..iterations {
            {
                if random.random_range(0..5) == 0 {
                    PForUtil::skip(&mut pdu.input)?;
                    continue;
                }
            }
            let mut restored = vec![0i32; ForUtil::BLOCK_SIZE];
            pfor_util.decode(&mut pdu, &mut restored)?;

            let expected = &values[i * ForUtil::BLOCK_SIZE..(i + 1) * ForUtil::BLOCK_SIZE];
            assert_eq!(restored, expected, "Mismatch at iteration {}", i);
        }

        assert_eq!(end_pointer, pdu.input.get_file_pointer());
        Ok(())
    }
    fn create_test_data<R: Rng + ?Sized>(
        iterations: usize,
        max_bpv: i32,
        random: &mut R,
    ) -> Vec<i32> {
        assert!(max_bpv > 0 && max_bpv <= 31);
        let mut values = vec![0i32; iterations * ForUtil::BLOCK_SIZE];
        for i in 0..iterations {
            let bpv = TestUtil::next_int(random, 0, max_bpv);
            for j in 0..ForUtil::BLOCK_SIZE {
                let idx = i * ForUtil::BLOCK_SIZE + j;
                values[idx] = random.random_range(0..=PackedInts::max_value(bpv) as i32);
                if random.random_range(0..100) == 0 {
                    let extra = if random.random_range(0..10) == 0 {
                        TestUtil::next_int(random, 9, 16)
                    } else {
                        TestUtil::next_int(random, 1, 8)
                    };
                    let exception_bpv = (bpv + extra).min(max_bpv);
                    values[idx] |= random.random_range(0..(1 << (exception_bpv - bpv))) << bpv;
                }
            }
        }
        values
    }
    fn encode_test_data(iterations: usize, values: &[i32], dir: &impl Directory) -> Result<i64> {
        let mut out = dir.create_output("test.bin", &IOContext::default_io_context()?)?;
        let mut pfor_util = PForUtil::new();

        for i in 0..iterations {
            let mut source = [0i32; ForUtil::BLOCK_SIZE];
            for j in 0..ForUtil::BLOCK_SIZE {
                source[j] = values[i * ForUtil::BLOCK_SIZE + j];
            }
            pfor_util.encode(&mut source, &mut out)?;
        }

        let end_pointer = out.get_file_pointer();
        Ok(end_pointer)
    }
}
