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
use crc32fast::Hasher;

pub trait Checksum {
    fn update(&mut self, b: u8);
    fn update_bytes(&mut self, bytes: &[u8], offset: i32, len: i32);
    fn get_value(&mut self) -> i64;
    fn reset(&mut self);
}

pub struct HasherChecksum {
    hasher: Hasher,
    initial_state: Hasher,
}

impl HasherChecksum {
    pub fn new(hasher: Hasher) -> Self {
        Self {
            hasher: hasher.clone(),
            initial_state: hasher,
        }
    }
}

impl Checksum for HasherChecksum {
    fn update(&mut self, b: u8) {
        self.hasher.update(&[b]);
    }

    fn update_bytes(&mut self, bytes: &[u8], offset: i32, len: i32) {
        let offset = offset as usize;
        let len = len as usize;
        self.hasher.update(&bytes[offset..offset + len]);
    }

    fn get_value(&mut self) -> i64 {
        self.hasher.clone().finalize() as i64
    }

    fn reset(&mut self) {
        self.hasher = self.initial_state.clone();
    }
}
