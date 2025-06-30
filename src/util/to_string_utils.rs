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

use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::access::AccessVec;

pub struct ToStringUtils;

impl ToStringUtils {
    #[allow(unused)]
    pub fn byte_array(buffer: &mut String, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            use std::fmt::Write;
            write!(buffer, "b[{}]={}", i, b).unwrap();
            if i < bytes.len() - 1 {
                buffer.push(',');
            }
        }
    }

    #[allow(dead_code)]
    const HEX: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];

    #[allow(dead_code)]
    pub fn long_hex(_x: u64) -> String {
        // not used in Java Lucene
        "".to_string()
    }

    pub fn bytes_ref_to_string<AV>(b: &BytesRef<AV>) -> String
    where
        AV: AccessVec<u8>,
    {
        b.bytes.access(|bytes| {
            if bytes.is_empty() {
                return "null".to_string();
            }
            let v = b.utf8_to_string();
            match v {
                Ok(s) => s,
                Err(_) => {
                    // If BytesRef isn't actually UTF-8, or it's e.g. a prefix of UTF-8
                    // that ends mid-unicode-char, we fall back to hex:
                    b.to_string()
                },
            }
        })
    }

    pub fn bytes_ref_to_string_from_builder<AV>(b: &BytesRefBuilder<AV>) -> String
    where
        AV: AccessVec<u8>,
    {
        Self::bytes_ref_to_string(b.get_bytes_ref())
    }
    pub fn bytes_ref_to_string_from_bytes(b: Vec<u8>) -> String {
        Self::bytes_ref_to_string(&BytesRef::from_bytes(b))
    }
}
