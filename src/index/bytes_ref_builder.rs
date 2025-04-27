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
use crate::util::access::AccessVec;
use crate::util::array_util::ArrayUtil;
use crate::util::SliceCopyOps;

/// A builder for {@link BytesRef} instances.
pub struct BytesRefBuilder<AV>
where
    AV: AccessVec<u8>,
{
    pub(crate) bytes_ref: BytesRef<AV>,
}
impl<AV> Default for BytesRefBuilder<AV>
where
    AV: AccessVec<u8>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<AV> BytesRefBuilder<AV>
where
    AV: AccessVec<u8>,
{
    pub fn new() -> BytesRefBuilder<AV> {
        BytesRefBuilder {
            bytes_ref: BytesRef::new(),
        }
    }
    /// Return a reference to the bytes of this builder.
    pub fn bytes_ref(&mut self) -> &mut BytesRef<AV> {
        &mut self.bytes_ref
    }

    /// Return the number of bytes in this buffer.
    pub fn length(&self) -> usize {
        self.bytes_ref.bytes.len()
    }

    /// Set the length.
    pub fn set_length(&mut self, length: usize) {
        self.bytes_ref.length = length;
        self.bytes_ref.bytes.access_mut(|bytes| {
            bytes.truncate(length);
        })
    }

    /// Return the byte at the given offset.
    pub fn byte_at(&self, index: usize) -> u8 {
        self.bytes_ref.bytes.access(|bytes| bytes[index])
    }

    /// Set a byte.
    pub fn set_byte_at(&mut self, offset: usize, value: u8) {
        self.bytes_ref.bytes.access_mut(|bytes| {
            bytes[offset] = value;
        })
    }
    pub fn grow(&mut self, capacity: usize) {
        self.bytes_ref
            .bytes
            .access_mut(|bytes| ArrayUtil::grow_with_len(bytes, capacity))
    }
    pub fn grow_no_copy(&mut self, capacity: usize) {
        self.grow(capacity)
    }

    /// Append a single byte to this builder.
    pub fn append_byte(&mut self, b: u8) {
        self.bytes_ref.bytes.access_mut(|bytes| {
            bytes.push(b);
        });
        self.bytes_ref.length += 1;
    }

    /// Append the provided bytes to this builder.
    pub fn append_with_range(&mut self, b: &[u8], off: usize, len: usize) {
        self.grow(self.bytes_ref.length + len);
        let pos = self.bytes_ref.length;
        self.bytes_ref()
            .bytes
            .access_mut(|bytes| bytes.copy_from(&b[off..off + len], pos));
        self.bytes_ref.length += len;
    }

    /// Append the provided bytes to this builder.
    pub fn append_ref(&mut self, b: &BytesRef<AV>) {
        b.bytes
            .access(|bytes| self.append_with_range(bytes, b.offset, b.length))
    }

    /// Reset this builder to the empty state.
    pub fn append_builder(&mut self, b: &mut BytesRefBuilder<AV>) {
        self.append_ref(b.get_bytes_ref())
    }
    pub fn clear(&mut self) {
        self.set_length(0);
        self.bytes_ref.bytes.access_mut(|bytes| bytes.clear());
        self.bytes_ref.offset = 0;
    }

    /// Replaces the content of this builder with the provided bytes.
    ///
    /// This is equivalent to calling [`clear`](BytesRefBuilder::clear) and then
    /// [`append`](BytesRefBuilder::append_with_range) with the specified
    /// `Vec<u8>` and range parameters (`start` and `end`).
    ///
    /// # Parameters
    /// - `bytes`: The byte vector to replace the current content.
    /// - `start`: The starting index of the byte slice to append.
    /// - `End`: The ending index of the byte slice to append.
    ///
    /// # See Also
    /// - [`clear`](BytesRefBuilder::clear)
    /// - [`append`](BytesRefBuilder::append_with_range)
    pub fn copy_bytes_with_vec(&mut self, b: &[u8], off: usize, len: usize) {
        debug_assert_eq!(self.bytes_ref.offset, 0);
        self.bytes_ref.length = len;
        self.grow_no_copy(len);
        self.bytes_ref()
            .bytes
            .access_mut(|bytes| bytes.copy_from(&b[off..off + len], 0))
    }
    pub fn copy_bytes_with_ref(&mut self, b: &BytesRef<AV>) {
        b.bytes
            .access(|bytes| self.copy_bytes_with_vec(bytes, b.offset, b.length))
    }
    pub fn copy_bytes_with_builder(&mut self, b: &mut BytesRefBuilder<AV>) {
        self.copy_bytes_with_ref(b.get_bytes_ref())
    }
    pub fn copy_chars_with_string(&mut self, s: &str) {
        self.copy_chars_range(s, 0, s.len())
    }
    pub fn copy_chars_range(&mut self, s: &str, off: usize, len: usize) {
        let sub_bytes = s.as_bytes()[off..(off + len)].to_vec();
        self.copy_chars_with_vec(&sub_bytes, 0, sub_bytes.len())
    }
    pub fn copy_chars_with_vec(&mut self, s: &[u8], off: usize, len: usize) {
        self.grow(len);
        self.bytes_ref
            .bytes
            .access_mut(|bytes| bytes.copy_from(&s[off..(off + len)], off));
        self.bytes_ref.length = len;
        self.bytes_ref.offset = 0;
    }

    /// Return a BytesRef that points to the internal content of this builder.
    /// Any update to  the content of this builder might invalidate the
    /// provided bytes_ref and vice versa.
    pub fn get_bytes_ref(&mut self) -> &mut BytesRef<AV> {
        debug_assert_eq!(
            self.bytes_ref.offset, 0,
            "Modifying the offset of the returned ref is illegal"
        );
        &mut self.bytes_ref
    }
    /// # Note
    /// This method should be only called with BytesRef<Vec<u8>>
    pub fn get_bytes(&mut self) -> BytesRef<AV> {
        std::mem::take(&mut self.bytes_ref)
    }
    /// Build a new BytesRef that has the same content as this buffer.
    pub fn get_bytes_ref_copy(&mut self) -> BytesRef<AV> {
        BytesRef::from_bytes(self.bytes_ref.bytes.slice_clone(0, self.bytes_ref.length))
    }
}
