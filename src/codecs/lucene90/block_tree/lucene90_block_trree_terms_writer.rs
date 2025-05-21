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
pub struct Lucene90BlockTreeTermsWriter;

pub(crate) mod lucene90_bttw_util {
    use crate::store::DataOutput;
    use crate::util::error::lucene_error::Result;

    pub(crate) fn write_msb_vlong(out: &mut impl DataOutput, mut l: i64) -> Result<()> {
        debug_assert!(l >= 0);
        // Keep zero bits on most significant byte to have more chance to get prefix
        // bytes shared. e.g. we expect 0x7FFF stored as [0x81, 0xFF, 0x7F] but
        // not [0xFF, 0xFF, 0x40]
        let bits = 64 - l.leading_zeros();
        let bytes_needed = ((bits.saturating_sub(1)) / 7 + 1) as usize;
        l <<= 64 - bytes_needed * 7;
        for _ in 1..bytes_needed {
            let byte = ((l >> 57) & 0x7F) as u8 | 0x80;
            out.write_byte(byte)?;
            l <<= 7;
        }
        let last_byte = ((l >> 57) & 0x7F) as u8;
        out.write_byte(last_byte)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::codecs::block_tree::field_reader::field_reader_util;
    use crate::codecs::block_tree::lucene90_block_trree_terms_writer::lucene90_bttw_util;
    use crate::store::{ByteArrayDataInput, ByteArrayDataOutput};
    use crate::test::util::lucene_test_case::{at_least, random};
    use crate::util::error::lucene_error::Result;
    #[allow(dead_code)] // for quick search
    struct TestMSBVLong;

    #[test]
    fn test_msb_vlong() -> Result<()> {
        assert_msb_vlong(i64::MAX)?;
        let mut random = random();
        let iter = at_least(&mut random, 10000) as i64;
        for i in 0..iter {
            assert_msb_vlong(i)?;
        }
        Ok(())
    }

    fn assert_msb_vlong(l: i64) -> Result<()> {
        let buffer = vec![0u8; 10];
        let mut output = ByteArrayDataOutput::with_bytes(buffer);
        lucene90_bttw_util::write_msb_vlong(&mut output, l)?;
        let buffer = output.bytes.clone();
        let len = output.get_position();
        let mut input = ByteArrayDataInput::with_range(buffer, 0, len);
        let recovered = field_reader_util::read_msb_vlong(&mut input)?;
        assert_eq!(
            recovered, l,
            "Mismatch in MSB VLong roundtrip: {} != {}",
            l, recovered
        );

        Ok(())
    }
}
