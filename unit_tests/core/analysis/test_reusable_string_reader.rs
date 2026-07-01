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
use crate::core::analysis::reader::Reader;
use crate::core::analysis::reusable_string_reader::ReusableStringReader;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestReusableStringReader;
#[test]
fn test_reusable_string_reader() -> Result<()> {
  let mut reader = ReusableStringReader::new();
  assert_eq!(reader.read()?, -1);
  let mut buf = ['\0'; 1];
  assert_eq!(reader.read_range(&mut buf, 0, 1)?, -1);
  let mut buf2 = ['\0'; 2];
  assert_eq!(reader.read_range(&mut buf2, 1, 1)?, -1);

  reader.set_value("foobar");
  let mut buf = ['\0'; 4];
  assert_eq!(reader.read_range(&mut buf, 0, 4)?, 4);
  assert_eq!(buf.iter().collect::<String>(), "foob");
  assert_eq!(reader.read_range(&mut buf, 0, 2)?, 2);
  assert_eq!(buf[..2].iter().collect::<String>(), "ar");
  assert_eq!(reader.read()?, -1);
  reader.close()?;

  reader.set_value("foobar");
  assert_eq!(reader.read_range(&mut buf, 1, 0)?, 0);
  assert_eq!(reader.read_range(&mut buf, 1, 3)?, 3);
  assert_eq!(buf[1..4].iter().collect::<String>(), "foo");
  assert_eq!(reader.read_range(&mut buf, 2, 2)?, 2);
  assert_eq!(buf[2..4].iter().collect::<String>(), "ba");
  assert_eq!(reader.read()?, 'r' as i32);
  assert_eq!(reader.read()?, -1);
  reader.close()?;

  reader.set_value("foobar");
  let mut sb = String::new();
  loop {
    let ch = reader.read()?;
    if ch == -1 {
      break;
    }
    sb.push(ch as u8 as char);
  }
  reader.close()?;
  assert_eq!(sb, "foobar");

  Ok(())
}
