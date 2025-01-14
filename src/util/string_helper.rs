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
use rand::Rng;
use std::fmt::Write;

/// Methods for manipulating strings.
///
/// # Note
/// This is an internal API.
pub struct StringHelper;
impl StringHelper {
    pub const ID_LENGTH: i32 = 16;
    pub fn random_id() -> [u8; 16] {
        let mut rng = rand::thread_rng();
        rng.gen::<[u8; 16]>()
    }
    /// Helper method to render an ID as a string for debugging.
    ///
    /// Returns the string `"null"` if the ID is `None`. Otherwise, returns a string
    /// representation for debugging. Never throws an exception. The returned string may indicate if
    /// the ID is definitely invalid.
    pub fn id_to_string(id: Option<&[u8]>) -> String {
        if let Some(id) = id {
            let big_int = num_bigint::BigUint::from_bytes_be(id);
            let mut result = big_int.to_str_radix(36);
            if id.len() != StringHelper::ID_LENGTH as usize {
                write!(&mut result, " (INVALID FORMAT)").unwrap();
            }
            result
        } else {
            "(null)".to_string()
        }
    }
}
