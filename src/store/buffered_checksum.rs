/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use crate::store::Checksum;
use crate::util::SliceCopyOps;

/// Wraps another [`Checksum`] with an internal buffer to speed up checksum
/// calculations.
pub struct BufferedChecksum<T: Checksum> {
    buffer: Vec<u8>,
    upto: i32,
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
        if self.upto == self.buffer.len() as i32 {
            self.flush();
        }
        self.buffer[self.upto as usize] = b;
        self.upto += 1;
    }

    fn update_bytes(&mut self, bytes: &[u8], offset: i32, len: i32) {
        let offset = offset as usize;
        let len = len as usize;

        if len >= self.buffer.len() {
            self.flush();
            self.checksum
                .update_bytes(&bytes[offset..offset + len], 0, len as i32);
        } else {
            if self.upto as usize + len > self.buffer.len() {
                self.flush();
            }
            self.buffer
                .copy_from(&bytes[offset..offset + len], self.upto as usize);
            self.upto += len as i32;
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

    use crate::store::{BufferedChecksum, Checksum, HasherChecksum};

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
                    buffered.update_bytes(&bytes, 0, length as i32);
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
