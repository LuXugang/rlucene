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
use crate::index::BytesRef;
use crate::util::access::AccessVec;
use crate::util::array_util::ArrayUtil;
use crate::util::ints_ref::IntsRef;
use crate::util::SliceCopyOps;

/// A builder for [`IntsRef`] instances.
///
/// Internal utility used during FST construction.
///
/// # Lucene internal
#[derive(Clone, Debug)]
pub struct IntsRefBuilder<AV>
where
    AV: AccessVec<i32>,
{
    ints_ref: IntsRef<AV>,
}
impl<AV> Default for IntsRefBuilder<AV>
where
    AV: AccessVec<i32>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<AV> IntsRefBuilder<AV>
where
    AV: AccessVec<i32>,
{
    pub fn new() -> Self {
        Self {
            ints_ref: IntsRef::default(),
        }
    }

    /// Returns a mutable reference to the underlying int buffer.
    pub fn ints(&mut self) -> AV {
        self.ints_ref.ints.clone()
    }

    /// Returns the number of ints in this buffer.
    pub fn length(&self) -> usize {
        self.ints_ref.length
    }

    /// Sets the current length of the buffer.
    pub fn set_length(&mut self, length: usize) {
        self.ints_ref.length = length;
    }

    /// Empties this builder.
    pub fn clear(&mut self) {
        self.set_length(0);
    }
    /// Returns the int at the given offset.
    pub fn int_at(&self, offset: usize) -> i32 {
        self.ints_ref.ints.access(|ints_bytes| ints_bytes[offset])
    }

    /// Sets the int at the given offset.
    pub fn set_int_at(&mut self, offset: usize, value: i32) {
        self.ints_ref.ints.access_mut(|ints_bytes| {
            ints_bytes[offset] = value;
        })
    }

    /// Appends the provided int to this buffer.
    pub fn append(&mut self, i: i32) {
        let mut len = self.ints_ref.length;
        self.grow(len + 1);
        len = self.ints_ref.length;
        self.ints_ref.ints.access_mut(|ints_bytes| {
            ints_bytes[len] = i;
        });
        self.ints_ref.length += 1;
    }

    /// Grows the reference array to at least `new_length`.
    ///
    /// In general, this should not be used directly, as it does not take offset
    /// into account.
    pub fn grow(&mut self, new_length: usize) {
        self.ints_ref
            .ints
            .access_mut(|ints_bytes| ArrayUtil::grow_with_len(ints_bytes, new_length))
    }

    /// Grows the reference array to at least `new_length`, without copying
    /// original data.
    pub fn grow_no_copy(&mut self, new_length: usize) {
        let v = self
            .ints_ref
            .ints
            .access_mut(|ints_bytes| ArrayUtil::grow_no_copy(ints_bytes, new_length));
        if let Some(v) = v {
            self.ints_ref.ints = AV::from_vec(v);
        }
    }

    /// Copies the given slice into this instance.
    pub fn copy_ints(&mut self, other: &[i32], other_offset: usize, other_length: usize) {
        self.grow_no_copy(other_length);
        self.ints_ref.ints.access_mut(|ints_bytes| {
            ints_bytes.copy_from(&other[other_offset..(other_offset + other_length)], 0);
            self.ints_ref.length = other_length;
        });
    }
    /// Copies the given [`IntsRef`] into this instance.
    pub fn copy_ints_ref(&mut self, ints: &IntsRef<AV>) {
        ints.ints
            .access(|ints_bytes| self.copy_ints(ints_bytes, ints.offset, ints.length))
    }

    /// Copies the given UTF-8 bytes into this builder.
    pub fn copy_utf8_bytes(&mut self, bytes: &BytesRef<Vec<u8>>) {
        self.grow_no_copy(bytes.length);
        self.ints_ref.length = bytes.length;
    }

    /// Returns a reference to the internal [`IntsRef`] content.
    ///
    /// Any modification to this builder may invalidate the returned value.
    pub fn get(&self) -> &IntsRef<AV> {
        debug_assert_eq!(
            self.ints_ref.offset, 0,
            "Modifying the offset of the returned ref is illegal"
        );
        &self.ints_ref
    }
    pub fn get_owner(&mut self) -> IntsRef<AV> {
        std::mem::take(&mut self.ints_ref)
    }

    /// Builds a new [`IntsRef`] that has the same content as this builder.
    pub fn to_ints_ref(&self) -> IntsRef<AV> {
        IntsRef::deep_copy_of(&self.ints_ref)
    }
}
