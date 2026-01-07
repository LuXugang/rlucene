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
use crate::core::store::Checksum;
use crate::core::util::SliceCopyOps;

/// Wraps another [`Checksum`] with an internal buffer to speed up checksum
/// calculations.
pub struct BufferedChecksum<T: Checksum> {
    buffer: Vec<u8>,
    upto: usize,
    checksum: T,
}

impl<T: Checksum> BufferedChecksum<T> {
    /// Default buffer size: 1024
    pub const DEFAULT_BUFFER_SIZE: u32 = 1024;
    pub fn new(checksum: T) -> Self {
        Self {
            buffer: vec![0; Self::DEFAULT_BUFFER_SIZE as usize],
            upto: 0,
            checksum,
        }
    }
    /// Create a new BufferedChecksum with the specified bufferSize
    pub fn with_buffer_size(checksum: T, buffer_size: u32) -> Self {
        Self {
            buffer: vec![0; buffer_size as usize],
            upto: 0,
            checksum,
        }
    }
    fn flush(&mut self) {
        if self.upto > 0 {
            self.checksum.update_bytes(&self.buffer, 0, self.upto);
            self.upto = 0;
        }
    }
}
impl<T: Checksum> Checksum for BufferedChecksum<T> {
    fn update(&mut self, b: u8) {
        debug_assert!(self.buffer.len() <= i32::MAX as usize);
        if self.upto == self.buffer.len() {
            self.flush();
        }
        self.buffer[self.upto] = b;
        self.upto += 1;
    }

    fn update_bytes(&mut self, bytes: &[u8], offset: usize, len: usize) {
        if len >= self.buffer.len() {
            self.flush();
            self.checksum
                .update_bytes(&bytes[offset..offset + len], 0, len);
        } else {
            if self.upto + len > self.buffer.len() {
                self.flush();
            }
            self.buffer
                .copy_from(&bytes[offset..offset + len], self.upto);
            self.upto += len;
        }
    }

    fn get_value(&mut self) -> i64 {
        self.flush();
        self.checksum.get_value()
    }

    fn reset(&mut self) {
        self.checksum.reset();
        self.upto = 0;
    }
}

#[cfg(test)]
mod tests {
    use crc32fast::Hasher;
    use rand::Rng;

    use crate::core::store::{BufferedChecksum, Checksum, HasherChecksum};

    #[allow(dead_code)] // for quick search
    struct TestBufferedChecksum {}
    #[test]
    fn test_simple() {
        let mut crc = Hasher::new();
        crc.update(&[1]);
        crc.update(&[2]);
        crc.update(&[3]);

        let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));
        buffered.update(1);
        buffered.update(2);
        buffered.update(3);

        assert_eq!(buffered.get_value(), crc.finalize() as i64);
    }

    #[test]
    fn test_random() {
        let mut raw_crc = Hasher::new();
        let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));

        let mut rng = rand::rng();
        let iterations = 10000;

        for _ in 0..iterations {
            match rng.random_range(0..4) {
                0 => {
                    let length = rng.random_range(0..1024);
                    let mut bytes = vec![0; length];
                    rng.fill(bytes.as_mut_slice());
                    raw_crc.update(&bytes);
                    buffered.update_bytes(&bytes, 0, length);
                },
                1 => {
                    let b = rng.random_range(0..=255) as u8;
                    raw_crc.update(&[b]);
                    buffered.update(b);
                },
                2 => {
                    raw_crc = Hasher::new();
                    buffered.reset();
                },
                3 => {
                    assert_eq!(buffered.get_value(), raw_crc.clone().finalize() as i64);
                },
                _ => unreachable!(),
            }
        }

        assert_eq!(buffered.get_value(), raw_crc.finalize() as i64);
    }
    // TODO: not finished
}
