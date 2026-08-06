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
use crate::test_framework::core::util::lucene_test_case::{at_least, is_night_mode};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::io::Cursor;

pub(crate) trait AbstractTestCompressionMode {
  fn get_mode(&self) -> CompressionModeEnum;
  fn random_array<R>(random: &mut R) -> Vec<u8>
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
    Self::random_array_impl(random, length, max)
  }
  fn random_array_impl<R>(random: &mut R, length: i32, max: i32) -> Vec<u8>
  where
    R: Rng + ?Sized,
  {
    let mut arr = vec![0u8; length as usize];
    for byte in &mut arr {
      *byte = random.random_range(0..=max) as u8;
    }
    arr
  }

  fn compress(
    &self,
    decompressed: &[u8],
    off: i32,
    len: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let mut compressor = self.get_mode().new_compressor();
    Self::compress_with_compressor(&mut compressor, decompressed, off, len)
  }

  fn compress_with_compressor(
    compressor: &mut impl Compressor,
    decompressed: &[u8],
    off: i32,
    len: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let compressed_len = len * 3 + 16;
    let compressed = vec![0; compressed_len as usize]; // should be enough
    let mut input = ByteBuffersDataInput::new(vec![Cursor::new(decompressed)], decompressed.len())?
      .slice(off as usize, len as usize)?;
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
      let decompressed = Self::random_array(random);
      let decompressed_len = decompressed.len() as i32;
      let off = if random.random_bool(0.5) {
        0
      } else {
        TestUtil::next_int(random, 0, decompressed_len)
      };
      let len = if random.random_bool(0.5) {
        decompressed_len - off
      } else {
        TestUtil::next_int(random, 0, decompressed_len - off)
      };
      let compressed = self.compress(decompressed.as_slice(), off, len)?;
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
      let decompressed = Self::random_array(random);
      let decompressed_len = decompressed.len() as i32;
      let compressed = self.compress(&decompressed, 0, decompressed_len)?;
      let (offset, length) = if decompressed_len == 0 {
        (0, 0)
      } else {
        let offset_inner = random.random_range(0..decompressed_len);
        (
          offset_inner,
          random.random_range(0..decompressed_len - offset_inner),
        )
      };
      let restored = self.decompress_with_range(&compressed, decompressed_len, offset, length)?;
      assert_eq!(
        ArrayUtil::copy_of_sub_array(&decompressed, offset as usize, (offset + length) as usize),
        restored
      );
    }
    Ok(())
  }

  fn test(&self, decompressed: &[u8]) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    self.test_with_range(decompressed, 0, decompressed.len() as i32)
  }

  fn test_with_range(
    &self,
    decompressed: &[u8],
    off: i32,
    len: i32,
  ) -> crate::core::util::error::lucene_error::Result<Vec<u8>> {
    let compressed = self.compress(decompressed, off, len)?;
    let restored = self.decompress(&compressed, len)?;
    assert_eq!(len as usize, restored.len());
    Ok(compressed)
  }

  fn test_empty_sequence(&self) -> crate::core::util::error::lucene_error::Result<()> {
    self.test(&[])?;
    Ok(())
  }

  fn test_short_sequence<R>(
    &self,
    random: &mut R,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    self.test(&[random.random_range(0..256) as u8])?;
    Ok(())
  }

  fn test_incompressible<R>(
    &self,
    random: &mut R,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut decompressed = vec![0; random.random_range(20..=256)];
    for (i, byte) in decompressed.iter_mut().enumerate() {
      *byte = i as u8;
    }
    self.test(&decompressed)?;
    Ok(())
  }

  fn test_constant<R>(&self, random: &mut R) -> crate::core::util::error::lucene_error::Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut decompressed = vec![0; TestUtil::next_int(random, 1, 10000) as usize];
    decompressed.fill(random.random());
    self.test(&decompressed)?;
    Ok(())
  }

  fn test_extremely_large_input(&self) -> crate::core::util::error::lucene_error::Result<()> {
    let limit = 1 << 24; // 16MB
    let mut decompressed = vec![0u8; limit as usize];
    for (i, byte) in decompressed.iter_mut().enumerate() {
      *byte = (i & 0x0F) as u8
    }
    self.test(&decompressed)?;
    Ok(())
  }
}
