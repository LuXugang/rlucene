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
use crate::store::dummy::dummy_index_output::DummyIndexOutput;
use crate::store::IndexOutput;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::Result;

pub struct Lucene90CompressingStoredFieldsWriter<O>
where
    O: IndexOutput,
{
    meta_stream: O,
    fields_stream: O,
}
impl Lucene90CompressingStoredFieldsWriter<DummyIndexOutput> {
    // -0 isn't compressed.
    pub(crate) const NEGATIVE_ZERO_FLOAT: u32 = (-0f32).to_bits();
    pub(crate) const NEGATIVE_ZERO_DOUBLE: u64 = (-0f64).to_bits();

    // for compression of timestamps
    pub(crate) const SECOND: i64 = 1_000;
    pub(crate) const HOUR: i64 = 60 * 60 * Self::SECOND;
    pub(crate) const DAY: i64 = 24 * Self::HOUR;

    pub(crate) const SECOND_ENCODING: i32 = 0x40;
    pub(crate) const HOUR_ENCODING: i32 = 0x80;
    pub(crate) const DAY_ENCODING: i32 = 0xC0;
}
impl<O> Lucene90CompressingStoredFieldsWriter<O>
where
    O: IndexOutput,
{
    /// Writes a float in a variable-length format. Writes between one and five bytes.
    /// Small integral values typically take fewer bytes.
    ///
    /// ZFloat --> Header, Bytes*?
    ///
    /// - Header --> [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte) (Uint8). When it is equal to `0xFF` then the value
    ///   is negative and stored in the next 4 bytes. Otherwise, if the first bit is set, then the
    ///   other bits in the header encode the value plus one and no other bytes are read.
    ///   Otherwise, the value is a positive float value whose first byte is the header, and 3
    ///   bytes need to be read to complete it.
    /// - Bytes --> Potential additional bytes to read depending on the header.
    pub(crate) fn write_zfloat(out: &mut O, f: f32) -> Result<()> {
        let int_val = f as i32;
        let float_bits = f.to_bits();

        if f == int_val as f32
            && (-1..=0x7D).contains(&int_val)
            && float_bits != Lucene90CompressingStoredFieldsWriter::NEGATIVE_ZERO_FLOAT
        {
            // small integer [-1..125]: single byte
            out.write_byte((0x80 | (1 + int_val)) as u8)?;
        } else if (float_bits >> 31) == 0 {
            // other positive floats: 4 bytes
            out.write_byte((float_bits >> 24) as u8)?;
            out.write_short((float_bits >> 8) as i16)?;
            out.write_byte(float_bits as u8)?;
        } else {
            // negative float or special: 5 bytes
            out.write_byte(0xFFu8)?;
            out.write_int(float_bits as i32)?;
        }
        Ok(())
    }
    /// Writes a float in a variable-length format. Writes between one and five bytes.
    /// Small integral values typically take fewer bytes.
    ///
    /// ZFloat --> Header, Bytes*?
    ///
    /// - Header --> [`DataOutput::write_byte`](crate::store::data_output::DataOutput::write_byte) (Uint8). When it is equal to `0xFF` then the value
    ///   is negative and stored in the next 8 bytes. When it is equal to `0xFE` then the value is
    ///   stored as a float in the next 4 bytes. Otherwise if the first bit is set then the other
    ///   bits in the header encode the value plus one and no other bytes are read. Otherwise, the
    ///   value is a positive float value whose first byte is the header, and 7 bytes need to be
    ///   read to complete it.
    /// - Bytes --> Potential additional bytes to read depending on the header.
    pub(crate) fn write_zdouble(out: &mut O, d: f64) -> Result<()> {
        let int_val = d as i32;
        let double_bits = d.to_bits(); // u64

        if d == int_val as f64
            && (-1..=0x7C).contains(&int_val)
            && double_bits != Lucene90CompressingStoredFieldsWriter::NEGATIVE_ZERO_DOUBLE
        {
            // small integer value [-1..124]: single byte
            out.write_byte((0x80 | (int_val + 1)) as u8)?;
        } else if d == (d as f32) as f64 {
            // d has an accurate float representation: 5 bytes
            out.write_byte(0xFE)?;
            out.write_int((d as f32).to_bits() as i32)?;
        } else if (double_bits >> 63) == 0 {
            // other positive doubles: 8 bytes
            out.write_byte((double_bits >> 56) as u8)?;
            out.write_int((double_bits >> 24) as u32 as i32)?; // lower 32 bits as i32
            out.write_short((double_bits >> 8) as i16)?;
            out.write_byte(double_bits as u8)?;
        } else {
            // other negative doubles: 9 bytes
            out.write_byte(0xFF)?;
            out.write_long(double_bits as i64)?;
        }
        Ok(())
    }
    /// Writes a long in a variable-length format. Writes between one and ten bytes.
    /// Small values or values representing timestamps with day, hour or second precision
    /// typically require fewer bytes.
    ///
    /// ZLong --> Header, Bytes*?
    ///
    /// - Header --> The first two bits indicate the compression scheme:
    ///   - 00 - uncompressed
    ///   - 01 - multiple of 1000 (second)
    ///   - 10 - multiple of 3600000 (hour)
    ///   - 11 - multiple of 86400000 (day)
    ///
    ///   Then the next bit is a continuation bit, indicating whether more bytes need to be read,
    ///   and the last 5 bits are the lower bits of the encoded value. In order to reconstruct the
    ///   value, you need to combine the 5 lower bits of the header with a vLong in the next bytes
    ///   (if the continuation bit is set to 1). Then [`BitUtil::zig_zag_decode`](BitUtil::zig_zag_decode_i64) it and finally
    ///   multiply by the multiple corresponding to the compression scheme.
    ///
    /// - Bytes --> Potential additional bytes to read depending on the header.
    // T for "timestamp"
    pub(crate) fn write_tlong(out: &mut O, mut l: i64) -> Result<()> {
        let mut header;

        if l % Lucene90CompressingStoredFieldsWriter::SECOND != 0 {
            header = 0;
        } else if l % Lucene90CompressingStoredFieldsWriter::DAY == 0 {
            // timestamp with day precision
            header = Lucene90CompressingStoredFieldsWriter::DAY_ENCODING;
            l /= Lucene90CompressingStoredFieldsWriter::DAY;
        } else if l % Lucene90CompressingStoredFieldsWriter::HOUR == 0 {
            // timestamp with hour precision, or day precision with a timezone
            header = Lucene90CompressingStoredFieldsWriter::HOUR_ENCODING;
            l /= Lucene90CompressingStoredFieldsWriter::HOUR;
        } else {
            // timestamp with second precision
            header = Lucene90CompressingStoredFieldsWriter::SECOND_ENCODING;
            l /= Lucene90CompressingStoredFieldsWriter::SECOND;
        }

        let zigzag_l = BitUtil::zig_zag_encode_i64(l);
        header |= (zigzag_l & 0x1F) as i32; // last 5 bits

        let upper_bits = ((zigzag_l as u64) >> 5) as i64;
        if upper_bits != 0 {
            header |= 0x20;
        }

        out.write_byte(header as u8)?;

        if upper_bits != 0 {
            out.write_vlong(upper_bits)?;
        }
        Ok(())
    }
}
