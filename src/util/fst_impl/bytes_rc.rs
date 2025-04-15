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
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone)]
pub struct BytesRc {
    pub bytes: Rc<Vec<u8>>,
    pub offset: i32,
    pub length: i32,
}
impl Default for BytesRc {
    fn default() -> Self {
        Self::new()
    }
}
impl BytesRc {
    pub fn new() -> Self {
        BytesRc {
            bytes: Rc::new(vec![]),
            offset: 0,
            length: 0,
        }
    }
    pub fn from_vec(bytes: Rc<Vec<u8>>, offset: i32, length: i32) -> BytesRc {
        BytesRc {
            bytes,
            offset,
            length,
        }
    }
    pub fn from_bytes(bytes: Rc<Vec<u8>>) -> BytesRc {
        debug_assert!(bytes.len() <= i32::MAX as usize);
        let length = bytes.len() as i32;
        Self::from_vec(bytes, 0, length)
    }
    pub fn with_capacity(capacity: i32) -> BytesRc {
        BytesRc {
            bytes: Rc::new(vec![0; capacity as usize]),
            offset: 0,
            length: 0,
        }
    }
}
impl Display for BytesRc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let end = self.offset + self.length;

        for (i, &byte) in self.bytes[self.offset as usize..end as usize]
            .iter()
            .enumerate()
        {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{:02x}", byte)?;
        }

        write!(f, "]")
    }
}
impl PartialOrd for BytesRc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for BytesRc {}

impl Ord for BytesRc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes[self.offset as usize..(self.offset + self.length) as usize]
            .cmp(&other.bytes[other.offset as usize..(other.offset + other.length) as usize])
    }
}
impl PartialEq for BytesRc {
    fn eq(&self, other: &Self) -> bool {
        self.bytes[self.offset as usize..(self.offset + self.length) as usize]
            == other.bytes[other.offset as usize..(other.offset + other.length) as usize]
    }
}
impl Hash for BytesRc {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let slice = &self.bytes[self.offset as usize..(self.offset + self.length) as usize];
        slice.hash(state);
        self.offset.hash(state);
        self.length.hash(state);
    }
}
