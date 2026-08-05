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

use crate::core::store::data_output::DataOutput;
use crate::core::store::index_output::IndexOutput;
use crate::core::store::output_stream_index_output::OutputStreamIndexOutput;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestOutputStreamIndexOutput;

#[test]
fn test_data_types() -> Result<()> {
  for offset in 0..12 {
    do_test_data_types(offset)?;
  }
  Ok(())
}

fn do_test_data_types(offset: usize) -> Result<()> {
  let mut buffer = Vec::new();
  {
    let resource_description = format!("test{offset}");
    let mut out = OutputStreamIndexOutput::new(&resource_description, "test", &mut buffer, 12)?;
    for i in 0..offset {
      out.write_byte(i as u8)?;
    }
    out.write_short(12345)?;
    out.write_int(1234567890)?;
    out.write_long(1234567890123456789)?;
    assert_eq!(out.get_file_pointer()?, (offset + 14));
    out.close()?;
  }

  let mut reader = Cursor::new(buffer);
  for i in 0..offset {
    assert_eq!(reader.read_u8()?, i as u8);
  }

  assert_eq!(reader.read_i16::<LittleEndian>()?, 12345);
  assert_eq!(reader.read_i32::<LittleEndian>()?, 1234567890);
  assert_eq!(reader.read_i64::<LittleEndian>()?, 1234567890123456789);
  assert_eq!(reader.position() as usize, reader.get_ref().len());

  Ok(())
}
