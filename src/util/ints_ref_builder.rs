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
use crate::index::BytesRef;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::ints_ref::IntsRef;
use crate::util::SliceCopyOps;
use std::cell::RefCell;
use std::rc::Rc;

/// A builder for [`IntsRef`] instances.
///
/// Internal utility used during FST construction.
///
/// # Lucene internal
pub struct IntsRefBuilder {
    ints_ref: IntsRef,
}
impl Default for IntsRefBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IntsRefBuilder {
    pub fn new() -> Self {
        Self {
            ints_ref: IntsRef::new(),
        }
    }

    /// Returns a mutable reference to the underlying int buffer.
    pub fn ints(&mut self) -> Rc<RefCell<Vec<i32>>> {
        self.ints_ref.ints.clone()
    }

    /// Returns the number of ints in this buffer.
    pub fn len(&self) -> i32 {
        self.ints_ref.length
    }

    /// Sets the current length of the buffer.
    pub fn set_length(&mut self, length: i32) {
        self.ints_ref.length = length;
    }

    /// Empties this builder.
    pub fn clear(&mut self) {
        self.set_length(0);
    }
    /// Returns the int at the given offset.
    pub fn int_at(&self, offset: i32) -> i32 {
        self.ints_ref.ints.borrow()[offset as usize]
    }

    /// Sets the int at the given offset.
    pub fn set_int_at(&mut self, offset: i32, value: i32) {
        self.ints_ref.ints.borrow_mut()[offset as usize] = value;
    }

    /// Appends the provided int to this buffer.
    pub fn append(&mut self, i: i32) -> Result<()> {
        let mut len = self.ints_ref.length;
        self.grow(len + 1)?;
        len = self.ints_ref.length;
        self.ints_ref.ints.borrow_mut()[len as usize] = i;
        self.ints_ref.length += 1;
        Ok(())
    }

    /// Grows the reference array to at least `new_length`.
    ///
    /// In general, this should not be used directly, as it does not take offset into account.
    pub fn grow(&mut self, new_length: i32) -> Result<()> {
        ArrayUtil::grow_with_len(&mut *self.ints_ref.ints.borrow_mut(), new_length)
    }

    /// Grows the reference array to at least `new_length`, without copying original data.
    pub fn grow_no_copy(&mut self, new_length: i32) -> Result<()> {
        let result = ArrayUtil::grow_no_copy(&self.ints_ref.ints.borrow(), new_length)?;
        if let Some(new_ints) = result {
            self.ints_ref.ints = Rc::new(RefCell::new(new_ints));
        }
        Ok(())
    }

    /// Copies the given slice into this instance.
    pub fn copy_ints(&mut self, other: &[i32], other_offset: i32, other_length: i32) -> Result<()> {
        self.grow_no_copy(other_length)?;
        let target = self.ints_ref.ints.borrow_mut();
        let dest = &mut *self.ints_ref.ints.borrow_mut();
        dest.copy_from(
            &other[other_offset as usize..(other_offset + other_length) as usize],
            0,
        );
        self.ints_ref.length = other_length;
        Ok(())
    }
    /// Copies the given [`IntsRef`] into this instance.
    pub fn copy_ints_ref(&mut self, ints: &IntsRef) -> Result<()> {
        self.copy_ints(&ints.ints.borrow(), ints.offset, ints.length)
    }

    /// Copies the given UTF-8 bytes into this builder.
    pub fn copy_utf8_bytes(&mut self, bytes: &BytesRef) -> Result<()> {
        self.grow_no_copy(bytes.length)?;
        self.ints_ref.length = bytes.length;
        Ok(())
    }

    /// Returns a reference to the internal [`IntsRef`] content.
    ///
    /// Any modification to this builder may invalidate the returned value.
    pub fn get(&self) -> &IntsRef {
        debug_assert_eq!(
            self.ints_ref.offset, 0,
            "Modifying the offset of the returned ref is illegal"
        );
        &self.ints_ref
    }

    /// Builds a new [`IntsRef`] that has the same content as this builder.
    pub fn to_ints_ref(&self) -> IntsRef {
        IntsRef::deep_copy_of(&self.ints_ref)
    }
}
