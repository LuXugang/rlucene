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
use crate::util::error::runtime_error::RuntimeError;
use std::cmp::Ordering;
use std::hash::Hash;

/// Represents a `&[u8]` as a slice (offset + length) into an existing byte array.
/// The `bytes` member should never be `None`;
///
/// # Important Note
/// Unless otherwise noted, this struct is used to represent terms encoded as **UTF-8** bytes in the index.
/// To convert them to a Rust `String` (which is UTF-8), use [`utf8_to_string`](#method.utf8_to_string).
/// Using code like `String::from_utf8_lossy(&bytes[offset..offset+length])` is the correct way to handle this.
/// Avoid constructing strings incorrectly, as it may result in wrong results.
///
/// # Sorting
/// This struct implements `Ord`. The underlying byte arrays are sorted lexicographically, treating elements as unsigned.
/// This is identical to Unicode codepoint order.
pub struct BytesRef {
    /// The contents of the BytesRef. Should never be `None`.
    pub bytes: Vec<u8>,
    pub offset: u32,
    pub length: u32,
}
impl Default for BytesRef {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesRef {
    pub fn new() -> BytesRef {
        BytesRef {
            bytes: Vec::new(),
            offset: 0,
            length: 0,
        }
    }
    pub fn new_from_vec(bytes: Vec<u8>, offset: u32, length: u32) -> BytesRef {
        BytesRef {
            bytes,
            offset,
            length,
        }
    }
    pub fn new_from_bytes(bytes: Vec<u8>) -> BytesRef {
        debug_assert!(bytes.len() <= u32::MAX as usize);
        let length = bytes.len() as u32;
        BytesRef {
            bytes,
            offset: 0,
            length,
        }
    }
    pub fn new_with_capacity(capacity: i32) -> BytesRef {
        BytesRef {
            bytes: Vec::with_capacity(capacity as usize),
            offset: 0,
            length: 0,
        }
    }
    pub fn new_from_string(s: &str) -> BytesRef {
        debug_assert!(s.len() <= u32::MAX as usize);
        BytesRef {
            bytes: s.as_bytes().to_vec(),
            offset: 0,
            length: s.len() as u32,
        }
    }
    /// Expert: compares the bytes against another BytesRef, returning true if the bytes are equal.
    ///
    /// # Arguments
    /// * `other` - Another BytesRef
    ///
    /// # Note
    /// This is an internal method.
    pub fn bytes_equals(&self, other: &BytesRef) -> bool {
        if self.length == other.length {
            for i in 0..self.length {
                if self.bytes[self.offset as usize + i as usize]
                    != other.bytes[other.offset as usize + i as usize]
                {
                    return false;
                }
            }
            return true;
        }
        false
    }
    /// Interprets the stored bytes as UTF-8, returning the resulting string.
    ///
    /// # Errors
    /// - May panic with an assertion error if debug assertions are enabled and the data is not well-formed UTF-8.
    /// - May return an error or panic if the data is not valid UTF-8 during runtime.
    pub fn utf8_to_string(&self) -> Result<String, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes[self.offset as usize..(self.offset + self.length) as usize])
            .map(|s| s.to_owned())
    }
    /// Creates a new `BytesRef` that points to a copy of the bytes from `other`.
    ///
    /// The returned `BytesRef` will have a length equal to `other.length` and an offset of zero.
    pub fn deep_copy_of(other: &BytesRef) -> BytesRef {
        Self::new_from_vec(other.bytes.clone(), 0, other.length)
    }
    /// Performs internal consistency checks. Always returns `true` (or throws `IllegalStateError`).
    pub fn is_valid(&self) -> Result<bool, RuntimeError> {
        if self.length as usize > self.bytes.len() {
            return Err(RuntimeError::illegal_state(format!(
                "length is out of bounds: {},bytes.length= {}",
                self.length,
                self.bytes.len()
            )));
        }
        if self.offset as usize > self.bytes.len() {
            return Err(RuntimeError::illegal_state(format!(
                "offset out of bounds: {},bytes.length= {}",
                self.offset,
                self.bytes.len()
            )));
        }
        if (self.offset + self.length) as usize > self.bytes.len() {
            return Err(RuntimeError::illegal_state(format!(
                "offset+length out of bounds: offset={},length={},bytes.length= {}",
                self.offset,
                self.length,
                self.bytes.len()
            )));
        }
        Ok(true)
    }
}
impl PartialOrd for BytesRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for BytesRef {}

impl Ord for BytesRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl Clone for BytesRef {
    fn clone(&self) -> Self {
        BytesRef {
            //TODO: maybe we should avoid cloning the bytes here
            bytes: self.bytes.clone(),
            offset: self.offset,
            length: self.length,
        }
    }
}
impl Hash for BytesRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
        self.offset.hash(state);
        self.length.hash(state);
    }
}
impl PartialEq for BytesRef {
    fn eq(&self, other: &Self) -> bool {
        self.bytes_equals(other)
    }
}
