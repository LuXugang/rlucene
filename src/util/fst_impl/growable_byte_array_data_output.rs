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
use crate::store::DataOutput;
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::SliceCopyOps;
// Storing a single contiguous byte[] for the current node of the FST we are writing. The byte[]
// will only grow, never shrink.
// Note: This is only safe for usage that is bounded in the number of bytes written. Do not make
// this public! Public users should instead use ByteBuffersDataOutput
pub(crate) struct GrowableByteArrayDataOutput {
    bytes: Vec<u8>,
    next_write: i32,
}
impl GrowableByteArrayDataOutput {
    const INITIAL_SIZE: usize = 1 << 8;
    pub(crate) fn new() -> Self {
        Self {
            bytes: vec![0u8; Self::INITIAL_SIZE],
            next_write: 0,
        }
    }
    pub(crate) fn get_position(&self) -> i32 {
        self.next_write
    }
    /// Returns the full byte buffer.
    pub(crate) fn get_bytes(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Set the position of the byte array, increasing the capacity if needed.
    pub(crate) fn set_position(&mut self, new_len: i32) -> Result<()> {
        debug_assert!(new_len >= 0);
        if new_len > self.next_write {
            self.ensure_capacity(new_len - self.next_write)?;
        }
        self.next_write = new_len;
        Ok(())
    }

    /// Ensure we can write additional `capacity_to_write` bytes.
    fn ensure_capacity(&mut self, capacity_to_write: i32) -> Result<()> {
        debug_assert!(capacity_to_write > 0);
        ArrayUtil::grow_with_len(&mut self.bytes, self.next_write + capacity_to_write)
    }
    /// Writes all of our bytes to the target `Write`.
    pub(crate) fn write_to_data_output(&self, out: &mut impl DataOutput) -> Result<()> {
        out.write_bytes_range(&self.bytes, 0, self.next_write)
    }

    /// Copies bytes from this store to a target buffer.
    pub(crate) fn write_to(&self, src_offset: i32, dest: &mut [u8], dest_offset: i32, len: i32) {
        debug_assert!(src_offset + len <= self.next_write);
        dest.copy_from(
            &self.bytes[src_offset as usize..(src_offset + len) as usize],
            dest_offset as usize,
        );
    }
}

impl DataOutput for GrowableByteArrayDataOutput {
    fn write_byte(&mut self, b: u8) -> Result<()> {
        self.ensure_capacity(1)?;
        self.bytes[self.next_write as usize] = b;
        self.next_write += 1;
        Ok(())
    }

    fn write_bytes_range(&mut self, b: &[u8], offset: i32, len: i32) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        self.ensure_capacity(len)?;
        let start = offset as usize;
        let end = start + len as usize;
        self.bytes
            .copy_from(&b[start..end], self.next_write as usize);
        self.next_write += len;
        Ok(())
    }
}
impl Accountable for GrowableByteArrayDataOutput {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
#[cfg(test)]
mod tests {
    use crate::store::directory::Directory;
    use crate::store::output_stream_data_output::OutputStreamDataOutput;
    use crate::store::{ByteArrayDataInput, DataOutput, IOContext};
    use crate::test::util::lucene_test_case::{at_least, is_night_mode, new_directory, random};
    use crate::test::util::test_util::TestUtil;
    use crate::util::error::lucene_error::Result;
    use crate::util::fst_impl::growable_byte_array_data_output::GrowableByteArrayDataOutput;
    use crate::util::SliceCopyOps;
    use rand::{Rng, RngCore};

    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let iters = at_least(&mut random, 10);
        let max_bytes = if is_night_mode() { 200_000 } else { 20_000 };

        for iter in 0..iters {
            let num_bytes = TestUtil::next_int(&mut random, 1, max_bytes);
            let mut expected = vec![0u8; num_bytes as usize];
            let mut bytes = GrowableByteArrayDataOutput::new();

            if cfg!(feature = "test_log_verbose") {
                println!("TEST: iter={} num_bytes={}", iter, num_bytes);
            }

            let mut pos = 0;
            while pos < num_bytes {
                if cfg!(feature = "test_log_verbose") {
                    println!("  cycle pos={}", pos);
                }

                match random.random_range(0..2) {
                    0 => {
                        // write single byte
                        let b = random.random::<u8>();
                        if cfg!(feature = "test_log_verbose") {
                            println!("    write_byte b={}", b);
                        }
                        expected[pos as usize] = b;
                        bytes.write_byte(b)?;
                        pos += 1;
                    }
                    1 => {
                        // write byte array
                        let max_len = std::cmp::min(num_bytes - pos, 100);
                        let len = random.random_range(0..max_len);
                        let mut temp = vec![0u8; len as usize];
                        random.fill_bytes(&mut temp);
                        if cfg!(feature = "test_log_verbose") {
                            println!("    write_bytes len={}, bytes={:?}", len, temp);
                        }
                        expected.copy_from(&temp[0..temp.len()], pos as usize);
                        bytes.write_bytes_range(&temp, 0, len)?;
                        pos += len;
                    }
                    _ => unreachable!(),
                }

                assert_eq!(pos, bytes.get_position());

                // maybe truncate
                if pos > 0 && random.random_range(0..50) == 17 {
                    let len = TestUtil::next_int(&mut random, 1, std::cmp::min(pos, 100));
                    pos -= len;
                    bytes.set_position(pos)?;
                    for i in pos..pos + len {
                        expected[i as usize] = 0;
                    }
                    if cfg!(feature = "test_log_verbose") {
                        println!("    truncate len={} new_pos={}", len, pos);
                    }
                }

                // maybe verify
                if pos > 0 && random.random_range(0..200) == 17 {
                    verify(&bytes, &expected, pos)?;
                }
            }

            let bytes_to_verify = if random.random_bool(0.5) {
                if cfg!(feature = "test_log_verbose") {
                    println!("TEST: save/load final bytes");
                }
                let mut dir = new_directory(&mut random)?;
                {
                    let mut out = dir.create_output("bytes", &IOContext::default_io_context()?)?;
                    bytes.write_to_data_output(&mut out)?;
                }

                let mut in_ = dir.open_input("bytes", &IOContext::default_io_context()?)?;
                let mut bytes_to_verify = GrowableByteArrayDataOutput::new();
                bytes_to_verify.copy_bytes(&mut in_, num_bytes as i64)?;
                bytes_to_verify
            } else {
                bytes
            };

            verify(&bytes_to_verify, &expected, num_bytes)?;
        }

        Ok(())
    }

    #[test]
    fn test_copy_bytes_on_byte_store() -> Result<()> {
        let mut random = random();
        let mut bytes = vec![0u8; 1024 * 8 + 10];
        let mut bytes_out = vec![0u8; bytes.len()];
        random.fill_bytes(&mut bytes);

        let offset = TestUtil::next_int(&mut random, 0, 100);
        let len = (bytes.len() - offset as usize) as i32;

        let bytes_clone = bytes.clone();
        let mut input = ByteArrayDataInput::with_range(bytes, offset, len);
        let mut o = GrowableByteArrayDataOutput::new();

        o.copy_bytes(&mut input, len as i64)?;
        o.write_to(0, &mut bytes_out, 0, len);

        let expected = &bytes_clone[offset as usize..(offset + len) as usize];
        let actual = &bytes_out[..len as usize];

        assert_eq!(actual, expected);
        Ok(())
    }

    #[allow(dead_code)] // for quick search
    struct TestGrowableByteArrayDataOutput;
    fn verify(
        bytes: &GrowableByteArrayDataOutput,
        expected: &[u8],
        total_length: i32,
    ) -> Result<()> {
        assert_eq!(bytes.get_position(), total_length);
        if total_length == 0 {
            return Ok(());
        }
        if cfg!(feature = "test_log_verbose") {
            println!("  verify...");
        }

        // First verify the whole thing in one blast:
        let mut buffer = Vec::new();
        let mut output = OutputStreamDataOutput::new(&mut buffer);
        bytes.write_to_data_output(&mut output)?;

        let data = output.os.into_inner().unwrap();
        assert_eq!(data.len(), total_length as usize);

        for i in 0..total_length as usize {
            assert_eq!(expected[i], data[i], "byte @ index={}", i);
        }
        Ok(())
    }
}
