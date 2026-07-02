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
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::test_framework::core::util::lucene_test_case::new_bytes_ref_with_length;

#[allow(dead_code)] // for quick search
struct TestBinaryDocValuesUpdates;
pub(crate) fn get_value(
  bdv: &mut impl BinaryDocValues,
) -> crate::core::util::error::lucene_error::Result<i64> {
  let term = bdv.binary_value()?;
  let mut idx = term.offset;
  debug_assert!(term.length > 0);
  let mut b = term.bytes[idx];
  idx += 1;

  let mut value = (b & 0x7F) as i64;
  let mut shift = 7;
  while (b as i64 & 0x80) != 0 {
    b = term.bytes[idx];
    idx += 1;
    value |= ((b & 0x7F) as i64) << shift;
    shift += 7;
  }

  Ok(value)
}
// encodes a long into a BytesRef as VLong so that we get varying number of bytes when we update
pub(crate) fn to_bytes<R>(
  random: &mut R,
  mut value: i64,
) -> crate::core::util::error::lucene_error::Result<BytesRef<Vec<u8>>>
where
  R: rand::Rng + ?Sized,
{
  let mut bytes: BytesRef<Vec<u8>> = new_bytes_ref_with_length(10, random)?;
  let mut upto = 0usize;

  while (value & !0x7f) != 0 {
    bytes.bytes[bytes.offset + upto] = ((value & 0x7f) | 0x80) as u8;
    upto += 1;
    value = ((value as u64) >> 7) as i64;
  }

  bytes.bytes[bytes.offset + upto] = value as u8;
  upto += 1;
  bytes.length = upto;

  Ok(bytes)
}
