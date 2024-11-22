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
use crate::util::comparator::Comparator;

/**
 * Specialized BytesRef comparator that StringSorter has optimizations for.
 *
 */
pub trait BytesRefComparator: Comparator<BytesRef> {
    /**
     * Return the unsigned byte to use for comparison at index i, or -1 if all bytes
     * that are useful for comparisons are exhausted. This may only be called with a value of i between
     * 0 included and `compared_bytes_count` excluded.
     */
    fn byte_at(&self, bytes_ref: &BytesRef, i: i32) -> i32;
    fn compare_with_offset(&self, o1: &BytesRef, o2: &BytesRef, k: i32) -> i32 {
        for i in k..self.compared_bytes_count() {
            let b1 = self.byte_at(o1, i);
            let b2 = self.byte_at(o2, i);
            if b1 != b2 {
                return b1 - b2;
            } else if b1 == -1 {
                break;
            }
        }
        0
    }
    fn compared_bytes_count(&self) -> i32;
}

pub struct Natural {
    compared_bytes_count: i32,
}
impl Default for Natural {
    fn default() -> Self {
        Natural {
            compared_bytes_count: i32::MAX,
        }
    }
}

impl Comparator<BytesRef> for Natural {
    fn compare(&self, a: &BytesRef, b: &BytesRef) -> i32 {
        self.compare_with_offset(a, b, 0)
    }
}

impl BytesRefComparator for Natural {
    fn byte_at(&self, bytes_ref: &BytesRef, i: i32) -> i32 {
        if i < bytes_ref.length {
            bytes_ref.bytes[(i + bytes_ref.offset) as usize] as i32
        } else {
            -1
        }
    }

    fn compare_with_offset(&self, o1: &BytesRef, o2: &BytesRef, k: i32) -> i32 {
        let start1 = (o1.offset + k) as usize;
        let start2 = (o2.offset + k) as usize;

        let slice1 = &o1.bytes[start1..(o1.offset + o1.length) as usize];
        let slice2 = &o2.bytes[start2..(o2.offset + o2.length) as usize];

        for (byte_a, byte_b) in slice1.iter().zip(slice2.iter()) {
            if byte_a != byte_b {
                return (*byte_a - *byte_b) as i32;
            }
        }
        (slice1.len() as i32) - (slice2.len() as i32)
    }

    fn compared_bytes_count(&self) -> i32 {
        self.compared_bytes_count
    }
}
