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
use crate::index::bytes_ref::BytesRef;
use crate::util::VecCopyOps;

/// A builder for {@link BytesRef} instances.
pub struct BytesRefBuilder {
    bytes_ref: BytesRef,
}
impl Default for BytesRefBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesRefBuilder {
    pub fn new() -> BytesRefBuilder {
        BytesRefBuilder {
            bytes_ref: BytesRef::new(),
        }
    }
    /// Return a reference to the bytes of this builder.
    pub fn bytes_ref(&self) -> &BytesRef {
        &self.bytes_ref
    }

    /// Return the number of bytes in this buffer.
    pub fn length(&self) -> u32 {
        self.bytes_ref.length
    }

    /// Set the length.
    pub fn set_length(&mut self, length: u32) {
        self.bytes_ref.length = length;
        self.bytes_ref.bytes.clear();
    }

    /// Return the byte at the given offset.
    pub fn byte_at(&self, index: u32) -> u8 {
        self.bytes_ref.bytes[(index) as usize]
    }

    /// Set a byte.
    pub fn set_byte_at(&mut self, offset: u32, value: u8) {
        self.bytes_ref.bytes[offset as usize] = value;
    }
    fn grow(&mut self, capacity: u32) {
        let current_len = self.bytes_ref.bytes.len();
        if capacity as usize <= current_len {
            return;
        }
        for _i in 0..(capacity as usize - current_len) {
            self.bytes_ref.bytes.push(0);
        }
    }
    fn grow_no_copy(&mut self, capacity: u32) {
        self.grow(capacity);
    }

    /// Append a single byte to this builder.
    pub fn append_byte(&mut self, b: u8) {
        self.bytes_ref.bytes.push(b);
        self.bytes_ref.length += 1;
    }

    /// Append the provided bytes to this builder.
    pub fn append(&mut self, b: &[u8], off: u32, len: u32) {
        self.bytes_ref
            .bytes
            .extend_from_slice(&b[off as usize..(off + len) as usize]);
        self.bytes_ref.length += len;
    }

    /// Append the provided bytes to this builder.
    pub fn append_ref(&mut self, b: &BytesRef) {
        self.append(&b.bytes, b.offset, b.length);
    }

    /// Reset this builder to the empty state.
    pub fn append_builder(&mut self, b: &mut BytesRefBuilder) {
        self.append_ref(b.get())
    }
    pub fn clear(&mut self) {
        self.set_length(0);
        self.bytes_ref.bytes.clear();
        self.bytes_ref.offset = 0;
    }

    /// Replaces the content of this builder with the provided bytes.
    ///
    /// This is equivalent to calling [`clear`](BytesRefBuilder::clear) and then [`append`](BytesRefBuilder::append) with the specified `Vec<u8>`
    /// and range parameters (`start` and `end`).
    ///
    /// # Parameters
    /// - `bytes`: The byte vector to replace the current content.
    /// - `start`: The starting index of the byte slice to append.
    /// - `end`: The ending index of the byte slice to append.
    ///
    /// # See Also
    /// - [`clear`](BytesRefBuilder::clear)
    /// - [`append`](BytesRefBuilder::append)
    pub fn copy_bytes_with_vec(&mut self, b: &[u8], off: u32, len: u32) {
        self.grow(len);
        assert_eq!(self.bytes_ref.offset, 0);
        self.bytes_ref.length = len;
        self.grow_no_copy(len);
        self.bytes_ref
            .bytes
            .copy_from(&b[off as usize..(off + len) as usize], 0);
    }
    pub fn copy_bytes_with_ref(&mut self, b: &BytesRef) {
        self.copy_bytes_with_vec(&b.bytes, b.offset, b.length);
    }
    pub fn copy_bytes_with_builder(&mut self, b: &mut BytesRefBuilder) {
        self.copy_bytes_with_ref(b.get());
    }
    pub fn copy_chars_with_string(&mut self, s: &str) {
        debug_assert!(s.len() <= u32::MAX as usize);
        self.copy_chars_range(s, 0, s.len() as u32);
    }
    pub fn copy_chars_range(&mut self, s: &str, off: u32, len: u32) {
        debug_assert!(s.len() <= u32::MAX as usize);
        let sub_bytes = s.as_bytes()[off as usize..(off + len) as usize].to_vec();
        self.copy_chars_with_vec(&sub_bytes, 0, sub_bytes.len() as u32);
    }
    pub fn copy_chars_with_vec(&mut self, s: &[u8], off: u32, len: u32) {
        self.grow(len);
        self.bytes_ref
            .bytes
            .copy_from(&s[off as usize..(off + len) as usize], off as usize);
        self.bytes_ref.length = len;
        self.bytes_ref.offset = 0;
    }

    /// Return a BytesRef that points to the internal content of this builder. Any update to
    ///  the content of this builder might invalidate the provided bytes_ref and vice-versa.
    pub fn get(&mut self) -> &mut BytesRef {
        assert_eq!(
            self.bytes_ref.offset, 0,
            "Modifying the offset of the returned ref is illegal"
        );
        &mut self.bytes_ref
    }
    /// Build a new BytesRef that has the same content as this buffer.
    pub fn to_bytes_ref(&self) -> BytesRef {
        self.bytes_ref.clone()
    }
}
