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
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use crate::util::test_error::TestError;
use rlucene::store::data_input::DataInput;
use rlucene::store::data_output::DataOutput;
use rlucene::store::{ByteArrayDataInput, ByteArrayDataOutput};

#[allow(dead_code)] // for quick search
struct TestByteArrayDataInput;

#[test]
fn test_basic() -> Result<(), TestError> {
    let bytes = vec![1, 65];
    let mut data_input = ByteArrayDataInput::with_bytes(bytes);
    assert_eq!(data_input.read_string()?, "A");
    assert!(data_input.eof());
    Ok(())
}

#[test]
fn test_data_types() -> Result<(), TestError> {
    // write some primitives using ByteArrayDataOutput:
    let mut bytes = vec![0u8; 32];
    let mut out = ByteArrayDataOutput::with_bytes(&mut bytes);

    out.write_byte(43)?;
    out.write_short(12345)?;
    out.write_int(1234567890)?;
    out.write_long(1234567890123456789)?;
    let size = out.get_position();
    assert_eq!(size, 15);

    let mut buf: Cursor<&[u8]> = Cursor::new(&bytes[..size as usize]);

    assert_eq!(buf.read_u8()?, 43);
    assert_eq!(buf.read_i16::<LittleEndian>()?, 12345);
    assert_eq!(buf.read_i32::<LittleEndian>()?, 1234567890);
    assert_eq!(buf.read_i64::<LittleEndian>()?, 1234567890123456789);
    assert_eq!(buf.position() as usize, size as usize);
    assert_eq!(buf.get_ref().len() - buf.position() as usize, 0);

    // read the primitives using ByteArrayDataInput:
    let mut data_input = ByteArrayDataInput::with_range(bytes, 0, size);
    assert_eq!(data_input.read_byte()?, 43);
    assert_eq!(data_input.read_short()?, 12345);
    assert_eq!(data_input.read_int()?, 1234567890);
    assert_eq!(data_input.read_long()?, 1234567890123456789);
    assert!(data_input.eof());
    Ok(())
}
