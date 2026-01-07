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
use crate::core::store::{DataOutput, IndexInput};
use crate::core::util::error::lucene_error::{LuceneError, Result};

pub(crate) struct StoredFieldsInts;
impl StoredFieldsInts {
    const BLOCK_SIZE: usize = 128;
    const BLOCK_SIZE_MINUS_ONE: usize = Self::BLOCK_SIZE - 1;
    pub(crate) fn write_ints(
        values: &[i32],
        start: i32,
        count: i32,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        let start = start as usize;

        let mut all_equal = true;
        for i in 1..count as usize {
            if values[start + i] != values[start] {
                all_equal = false;
                break;
            }
        }

        if all_equal {
            out.write_byte(0)?;
            out.write_vint(values[0])?;
        } else {
            let mut max: u64 = 0;
            for i in 0..count as usize {
                max |= values[start + i] as u32 as u64;
            }
            if max <= 0xff {
                out.write_byte(8)?;
                Self::write_ints8(out, count, values, start as i32)?;
            } else if max <= 0xffff {
                out.write_byte(16)?;
                Self::write_ints16(out, count, values, start as i32)?;
            } else {
                out.write_byte(32)?;
                Self::write_ints32(out, count, values, start as i32)?;
            }
        }

        Ok(())
    }

    fn write_ints8(
        out: &mut impl DataOutput,
        count: i32,
        values: &[i32],
        offset: i32,
    ) -> Result<()> {
        let mut k = 0;
        while k < (count - Self::BLOCK_SIZE_MINUS_ONE as i32) {
            let step = (offset + k) as usize;
            for i in 0..16 {
                let l = ((values[step + i] as i64) << 56)
                    | ((values[step + 16 + i] as i64) << 48)
                    | ((values[step + 32 + i] as i64) << 40)
                    | ((values[step + 48 + i] as i64) << 32)
                    | ((values[step + 64 + i] as i64) << 24)
                    | ((values[step + 80 + i] as i64) << 16)
                    | ((values[step + 96 + i] as i64) << 8)
                    | (values[step + 112 + i] as i64);
                out.write_long(l)?;
            }
            k += Self::BLOCK_SIZE as i32;
        }
        let offset = offset as usize;
        for i in k as usize..count as usize {
            out.write_byte(values[offset + i] as u8)?;
        }

        Ok(())
    }
    fn write_ints16(
        out: &mut impl DataOutput,
        count: i32,
        values: &[i32],
        offset: i32,
    ) -> Result<()> {
        let mut k = 0;
        while k < (count - Self::BLOCK_SIZE_MINUS_ONE as i32) {
            let step = (offset + k) as usize;
            for i in 0..32 {
                let l = ((values[step + i] as i64) << 48)
                    | ((values[step + 32 + i] as i64) << 32)
                    | ((values[step + 64 + i] as i64) << 16)
                    | (values[step + 96 + i] as i64);
                out.write_long(l)?;
            }
            k += Self::BLOCK_SIZE as i32;
        }
        let offset = offset as usize;
        for i in k as usize..count as usize {
            out.write_short(values[offset + i] as i16)?;
        }

        Ok(())
    }

    fn write_ints32(
        out: &mut impl DataOutput,
        count: i32,
        values: &[i32],
        offset: i32,
    ) -> Result<()> {
        let mut k = 0;
        while k < (count - Self::BLOCK_SIZE_MINUS_ONE as i32) {
            let step = (offset + k) as usize;
            for i in 0..64 {
                let l = ((values[step + i] as i64) << 32) | (values[step + 64 + i] as i64);
                out.write_long(l)?;
            }
            k += Self::BLOCK_SIZE as i32;
        }
        let offset = offset as usize;
        for i in k as usize..count as usize {
            out.write_int(values[offset + i])?;
        }

        Ok(())
    }
    pub(crate) fn read_ints(
        input: &mut impl IndexInput,
        count: i32,
        values: &mut [i64],
        offset: i32,
    ) -> Result<()> {
        let bpv = input.read_byte()? as i32;
        match bpv {
            0 => {
                let v = input.read_vint()? as i64;
                let start = offset as usize;
                let end = start + count as usize;
                values[start..end].fill(v);
            },
            8 => Self::read_ints8(input, count, values, offset)?,
            16 => Self::read_ints16(input, count, values, offset)?,
            32 => Self::read_ints32(input, count, values, offset)?,
            _ => {
                return Err(LuceneError::illegal_state(format!(
                    "Unsupported number of bits per value: {bpv}"
                )));
            },
        }
        Ok(())
    }

    fn read_ints8(
        input: &mut impl IndexInput,
        count: i32,
        values: &mut [i64],
        offset: i32,
    ) -> Result<()> {
        let mut k = 0;
        while k < (count - Self::BLOCK_SIZE_MINUS_ONE as i32) {
            let step = (offset + k) as usize;
            input.read_longs(values, step, 16)?;
            for i in 0..16 {
                let l = values[step + i];
                values[step + i] = (l >> 56) & 0xFF;
                values[step + 16 + i] = (l >> 48) & 0xFF;
                values[step + 32 + i] = (l >> 40) & 0xFF;
                values[step + 48 + i] = (l >> 32) & 0xFF;
                values[step + 64 + i] = (l >> 24) & 0xFF;
                values[step + 80 + i] = (l >> 16) & 0xFF;
                values[step + 96 + i] = (l >> 8) & 0xFF;
                values[step + 112 + i] = l & 0xFF;
            }
            k += Self::BLOCK_SIZE as i32;
        }
        let offset = offset as usize;
        for i in k as usize..count as usize {
            values[offset + i] = input.read_byte()? as i64;
        }
        Ok(())
    }

    fn read_ints16(
        input: &mut impl IndexInput,
        count: i32,
        values: &mut [i64],
        offset: i32,
    ) -> Result<()> {
        let mut k = 0;
        while k < (count - Self::BLOCK_SIZE_MINUS_ONE as i32) {
            let step = (offset + k) as usize;
            input.read_longs(values, step, 32)?;
            for i in 0..32 {
                let l = values[step + i];
                values[step + i] = (l >> 48) & 0xFFFF;
                values[step + 32 + i] = (l >> 32) & 0xFFFF;
                values[step + 64 + i] = (l >> 16) & 0xFFFF;
                values[step + 96 + i] = l & 0xFFFF;
            }
            k += Self::BLOCK_SIZE as i32;
        }
        let offset = offset as usize;
        for i in k as usize..count as usize {
            values[offset + i] = input.read_short()? as u16 as i64;
        }
        Ok(())
    }

    fn read_ints32(
        input: &mut impl IndexInput,
        count: i32,
        values: &mut [i64],
        offset: i32,
    ) -> Result<()> {
        let mut k = 0;
        while k < (count - Self::BLOCK_SIZE_MINUS_ONE as i32) {
            let step = (offset + k) as usize;
            input.read_longs(values, step, 64)?;
            for i in 0..64 {
                let l = values[step + i];
                values[step + i] = (l >> 32) & 0xFFFFFFFF;
                values[step + 64 + i] = l & 0xFFFFFFFF;
            }
            k += Self::BLOCK_SIZE as i32;
        }
        let offset = offset as usize;
        for i in k as usize..count as usize {
            values[offset + i] = input.read_int()? as u32 as i64;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {

    use rand::Rng;

    use crate::core::codecs::compressing::stored_fields_ints::StoredFieldsInts;
    use crate::core::store::directory::Directory;
    use crate::core::store::{DataOutput, IOContext, IndexInput, IndexOutput};
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, new_directory, random,
    };
    use crate::test::util::test_util::TestUtil;

    #[allow(dead_code)] // for quick search
    struct TestStoredFieldsInt;
    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let num_iters = at_least(&mut random, 100);
        let dir = new_directory(&mut random)?;

        for _ in 0..num_iters {
            let len = random.random_range(1..=5000);
            let bpv = TestUtil::next_int(&mut random, 1, 31);
            let mut values = vec![0; len];
            for v in values.iter_mut().take(len) {
                *v = TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
            }
            test(&mut random, &dir, &values)?;
        }

        Ok(())
    }

    #[test]
    fn test_all_equals() -> Result<()> {
        let mut random = random();
        let dir = new_directory(&mut random)?;
        let len = random.random_range(1..=5000);
        let bpv = TestUtil::next_int(&mut random, 1, 31);
        let value = TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
        let values = vec![value; len];
        test(&mut random, &dir, &values)?;
        Ok(())
    }

    fn test<R: Rng + ?Sized>(random: &mut R, dir: &impl Directory, ints: &[i32]) -> Result<()> {
        let len;
        {
            let mut out = dir.create_output("tmp", &IOContext::default_io_context()?)?;
            StoredFieldsInts::write_ints(ints, 0, ints.len() as i32, &mut out)?;
            len = out.get_file_pointer();
            if random.random_bool(0.5) {
                out.write_long(0)?;
            }
        }

        {
            let mut input = dir.open_input("tmp", &IOContext::read_once_io_context()?)?;
            let offset = random.random_range(0..=4);
            let mut read = vec![0i64; ints.len() + offset];
            StoredFieldsInts::read_ints(&mut input, ints.len() as i32, &mut read, offset as i32)?;

            let read_ints: Vec<i32> = read[offset..offset + ints.len()]
                .iter()
                .map(|&v| v as i32)
                .collect();

            assert_eq!(ints, read_ints.as_slice());
            assert_eq!(len, input.get_file_pointer()?);
        }

        dir.delete_file("tmp")?;
        Ok(())
    }
}
