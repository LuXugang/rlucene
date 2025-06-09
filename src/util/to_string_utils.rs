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
