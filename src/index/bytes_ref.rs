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
use std::cmp::Ordering;
use std::hash::Hash;

/**
 * Represents `vec<i16>`, as a slice (offset + length) into an existing `vec<i16>`.
 *
 * <p>`BytesRef` implements `Comparable`. The underlying byte arrays are sorted
 * lexicographically, numerically treating elements as unsigned. This is identical to Unicode
 * codepoint order.</p>
*/
pub struct BytesRef {
    pub bytes: Vec<u8>,
    pub offset: i32,
    pub length: i32,
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
    pub fn new_from_vec(bytes: Vec<u8>, offset: i32, length: i32) -> BytesRef {
        BytesRef {
            bytes,
            offset,
            length,
        }
    }
    pub fn new_from_bytes(bytes: Vec<u8>) -> BytesRef {
        let length = bytes.len() as i32;
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
        BytesRef {
            bytes: s.as_bytes().to_vec(),
            offset: 0,
            length: s.len() as i32,
        }
    }
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

    pub fn utf8_to_string(&self) -> Result<String, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes[self.offset as usize..(self.offset + self.length) as usize])
            .map(|s| s.to_owned())
    }
    pub fn deep_copy_of(other: &BytesRef) -> BytesRef {
        Self::new_from_vec(other.bytes.clone(), 0, other.length)
    }
    pub fn is_valid(&self) -> Result<bool, String> {
        if self.length < 0 {
            return Err(format!("length is negative: {}", self.length));
        }
        if self.length > self.bytes.len() as i32 {
            return Err(format!(
                "length is out of bounds: {},bytes.length= {}",
                self.length,
                self.bytes.len()
            ));
        }
        if self.offset < 0 {
            return Err(format!("offset is negative: {}", self.offset));
        }
        if self.offset > self.bytes.len() as i32 {
            return Err(format!(
                "offset out of bounds: {},bytes.length= {}",
                self.offset,
                self.bytes.len()
            ));
        }
        if self.offset + self.length < 0 {
            return Err(format!(
                "offset+length is negative: offset={},length={}",
                self.offset, self.length
            ));
        }
        if self.offset + self.length > self.bytes.len() as i32 {
            return Err(format!(
                "offset+length out of bounds: offset={},length={},bytes.length= {}",
                self.offset,
                self.length,
                self.bytes.len()
            ));
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
