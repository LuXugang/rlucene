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
use crate::core::index::bytes_ref_builder::BytesRefBuilder;
use crate::core::util::access::SharedAccessVec;

fn assert_case(chars: &[char], expected_len: usize) {
  let mut b: BytesRefBuilder<Vec<u8>> = BytesRefBuilder::new();
  let len = chars.len();
  b.copy_chars_from_chars(chars, 0, len);
  let br = b.get_bytes_ref();

  let s: String = chars.iter().collect();
  assert_eq!(s.len(), expected_len);

  assert_eq!(br.length, expected_len);

  let expected_bytes = s.into_bytes();
  br.bytes.access(|bytes| {
    assert_eq!(
      &bytes[br.offset..br.offset + br.length],
      expected_bytes.as_slice()
    );
  });

  assert_eq!(br.offset, 0);
}

/// Extra test: Rust Lucene-only, not in upstream Lucene
#[test]
fn test_copy_chars_with_chars_lengths_0_to_4() {
  let mut expected_len = 'a'.len_utf8();
  assert_case(&['a'], expected_len); // 1 byte
  expected_len = '\u{e9}'.len_utf8();
  assert_case(&['\u{e9}'], expected_len); // 2 bytes
  expected_len = '\u{4e2d}'.len_utf8();
  assert_case(&['\u{4e2d}'], expected_len); // 3 bytes
  expected_len = '\u{1f980}'.len_utf8();
  assert_case(&['\u{1f980}'], expected_len); // 4 bytes
}
