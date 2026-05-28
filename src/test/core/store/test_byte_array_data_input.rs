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
use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::core::store::data_input::DataInput;
use crate::core::store::data_output::DataOutput;
use crate::core::store::{ByteArrayDataInput, ByteArrayDataOutput};
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestByteArrayDataInput;

#[test]
fn test_basic() -> Result<()> {
  let bytes = vec![1, 65];
  let mut data_input = ByteArrayDataInput::with_bytes(bytes.as_slice());
  assert_eq!(data_input.read_string()?, "A");
  assert!(data_input.eof());
  Ok(())
}

#[test]
fn test_data_types() -> Result<()> {
  // write some primitives using ByteArrayDataOutput:
  let bytes = vec![0u8; 32];
  let mut out = ByteArrayDataOutput::with_bytes(bytes);

  out.write_byte(43)?;
  out.write_short(12345)?;
  out.write_int(1234567890)?;
  out.write_long(1234567890123456789)?;
  let size = out.get_position();
  assert_eq!(size, 15);

  let mut buf: Cursor<&[u8]> = Cursor::new(&out.bytes[..size]);

  assert_eq!(buf.read_u8()?, 43);
  assert_eq!(buf.read_i16::<LittleEndian>()?, 12345);
  assert_eq!(buf.read_i32::<LittleEndian>()?, 1234567890);
  assert_eq!(buf.read_i64::<LittleEndian>()?, 1234567890123456789);
  assert_eq!(buf.position() as usize, size);
  assert_eq!(buf.get_ref().len() - buf.position() as usize, 0);

  // read the primitives using ByteArrayDataInput:
  let mut data_input = ByteArrayDataInput::with_range(out.bytes.as_slice(), 0, size);
  assert_eq!(data_input.read_byte()?, 43);
  assert_eq!(data_input.read_short()?, 12345);
  assert_eq!(data_input.read_int()?, 1234567890);
  assert_eq!(data_input.read_long()?, 1234567890123456789);
  assert!(data_input.eof());
  Ok(())
}
