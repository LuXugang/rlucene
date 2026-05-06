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
use crate::core::codecs::compression::compression_mode::{
  CompressionModeBase, CompressionModeEnum,
};
use crate::core::codecs::compression::compressor::Compressor;
use crate::core::codecs::compression::decompressor::Decompressor;
use crate::core::index::BytesRef;
use crate::core::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::core::store::{ByteArrayDataInput, ByteArrayDataOutput};
use crate::core::util::array_util::ArrayUtil;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{at_least, is_night_mode};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::io::Cursor;

pub(crate) trait AbstractTestCompressionMode {
  fn get_mode(&self) -> CompressionModeEnum;
  fn random_array<R>(random: &mut R) -> (Vec<u8>, i32)
  where
    R: Rng + ?Sized,
  {
    let bigsize = if is_night_mode() {
      192 * 1024
    } else {
      33 * 1024
    };
    let max = if random.random_bool(0.5) {
      random.random_range(0..4)
    } else {
      random.random_range(0..255)
    };
    let length = if random.random_bool(0.5) {
      random.random_range(0..20)
    } else {
      random.random_range(0..bigsize)
    };
    (Self::random_array_impl(random, length, max), length)
  }
  fn random_array_impl<R>(random: &mut R, length: i32, max: i32) -> Vec<u8>
  where
    R: Rng + ?Sized,
  {
    let remainder = length % 1024;
    let new_length = if remainder == 0 {
      length
    } else {
      length + (1024 - remainder)
    };
    if length == 0 {
      vec![0u8; 1024]
    } else {
      let mut arr = vec![0u8; new_length as usize];
      for i in 0..length {
        arr[i as usize] = random.random_range(0..=max) as u8;
      }
      arr
    }
  }

  fn compress(
    &self,
    decompressed: &[u8],
    off: i32,
    len: i32,
    limit: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let mut compressor = self.get_mode().new_compressor();
    Self::compress_with_compressor(&mut compressor, decompressed, off, len, limit)
  }

  fn compress_with_compressor(
    compressor: &mut impl Compressor,
    decompressed: &[u8],
    off: i32,
    len: i32,
    limit: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let compressed_len = len * 3 + 16;
    let compressed = vec![0; compressed_len as usize]; // should be enough
    let mut cursor_vec = Vec::new();
    let chunk_size = 1024;
    let decompressed_len = decompressed.len() as i64;
    let vec = vec![0u8; chunk_size];
    let empty = vec.as_slice();
    if decompressed_len == 0 {
      cursor_vec.push(Cursor::new(empty));
    } else {
      for chunk in decompressed.chunks(chunk_size) {
        cursor_vec.push(Cursor::new(chunk));
      }
    }

    let mut input =
      ByteBuffersDataInput::new(cursor_vec, limit as usize)?.slice(off as usize, len as usize)?;
    let mut out = ByteArrayDataOutput::with_bytes(compressed);

    compressor.compress(&mut input, &mut out)?;
    let compressed_len = out.get_position();
    let result = ArrayUtil::copy_of_sub_array(&out.bytes, 0, compressed_len);
    Ok(result)
  }

  fn decompress(
    &self,
    compressed: &[u8],
    original_length: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let mut decompressor = self.get_mode().new_decompressor();
    Self::decompress_with_decompressor(&mut decompressor, compressed, original_length)
  }

  fn decompress_with_decompressor(
    decompressor: &mut impl Decompressor,
    compressed: &[u8],
    original_length: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let mut bytes = BytesRef::default();
    let mut input = ByteArrayDataInput::with_bytes(compressed);
    decompressor.decompress(&mut input, original_length, 0, original_length, &mut bytes)?;
    Ok(BytesRef::deep_copy_of(&bytes).bytes)
  }
  fn decompress_with_range(
    &self,
    compressed: &[u8],
    original_length: i32,
    offset: i32,
    length: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let mut decompressor = self.get_mode().new_decompressor();
    let mut bytes = BytesRef::default();
    let mut input = ByteArrayDataInput::with_bytes(compressed);
    decompressor.decompress(&mut input, original_length, offset, length, &mut bytes)?;
    Ok(BytesRef::deep_copy_of(&bytes).bytes)
  }

  fn test_decompress<R>(&self, random: &mut R) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let iterations = at_least(random, 3);
    for _ in 0..iterations {
      let (decompressed, limit) = Self::random_array(random);
      let decompressed_len = decompressed.len();
      assert!(decompressed_len <= i32::MAX as usize);
      assert!(limit as usize <= decompressed_len);
      let off = if random.random_bool(0.5) {
        0
      } else {
        TestUtil::next_int(random, 0, limit)
      };
      let len = if random.random_bool(0.5) {
        limit - off
      } else {
        TestUtil::next_int(random, 0, limit - off)
      };
      let compressed = self.compress(decompressed.as_slice(), off, len, limit)?;
      let restored = self.decompress(&compressed, len)?;
      assert_eq!(
        ArrayUtil::copy_of_sub_array(&decompressed, off as usize, (off + len) as usize),
        restored
      );
    }
    Ok(())
  }

  fn test_partial_decompress<R>(
    &self,
    random: &mut R,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let iterations = at_least(random, 3);
    for _ in 0..iterations {
      let (decompressed, limit) = Self::random_array(random);
      let compressed = self.compress(
        &decompressed,
        0,
        std::cmp::min(decompressed.len(), limit as usize) as i32,
        limit,
      )?;
      assert!(decompressed.len() <= i32::MAX as usize);
      let valid_len = std::cmp::min(decompressed.len(), limit as usize) as i32;
      let (offset, length) = if valid_len == 0 {
        (0, 0)
      } else {
        let offset_inner = random.random_range(0..valid_len);
        (
          offset_inner,
          random.random_range(0..valid_len - offset_inner),
        )
      };
      let restored = self.decompress_with_range(&compressed, valid_len, offset, length)?;
      assert_eq!(
        ArrayUtil::copy_of_sub_array(&decompressed, offset as usize, (offset + length) as usize),
        restored
      );
    }
    Ok(())
  }

  fn test(
    &self,
    decompressed: &[u8],
    limit: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    self.test_with_range(decompressed, 0, decompressed.len() as i32, limit)
  }

  fn test_with_range(
    &self,
    decompressed: &[u8],
    off: i32,
    len: i32,
    limit: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    assert!(off <= limit);
    assert!(limit <= len);
    let compressed = self.compress(decompressed, off, std::cmp::min(len, limit), limit)?;
    let restored = self.decompress(&compressed, limit)?;
    assert_eq!(limit as usize, restored.len());
    assert_eq!(
      ArrayUtil::copy_of_sub_array(
        decompressed,
        off as usize,
        (off + std::cmp::min(len, limit)) as usize
      ),
      restored
    );
    Ok(compressed)
  }

  fn test_empty_sequence(&self) -> crate::core::util::error::lucene_error::Result<()> {
    self.test(&[], 0)?;
    Ok(())
  }

  fn test_short_sequence<R>(
    &self,
    random: &mut R,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let limit = random.random_range(0..256);
    let mut bytes = vec![0u8; 1024];
    for byte in bytes.iter_mut().take(limit) {
      *byte = random.random();
    }
    self.test(&bytes, limit as i32)?;
    Ok(())
  }

  fn test_incompressible<R>(
    &self,
    random: &mut R,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let limit = random.random_range(20..=256);
    let mut decompressed = vec![0; 1024];
    for byte in decompressed.iter_mut().take(limit) {
      *byte = random.random();
    }
    self.test(&decompressed, limit as i32)?;
    Ok(())
  }

  fn test_constant<R>(&self, random: &mut R) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let limit = TestUtil::next_int(random, 1, 10000);
    let mut decompressed = vec![0; 10240];
    for byte in decompressed.iter_mut().take(limit as usize) {
      *byte = random.random();
    }
    self.test(&decompressed, limit)?;
    Ok(())
  }

  fn test_extremely_large_input(&self) -> crate::core::util::error::lucene_error::Result<()> {
    let limit = 1 << 24; // 16MB
    let mut decompressed = vec![0u8; limit as usize];
    for (i, byte) in decompressed.iter_mut().enumerate() {
      *byte = (i & 0x0F) as u8
    }
    self.test(&decompressed, limit)?;
    Ok(())
  }
}
