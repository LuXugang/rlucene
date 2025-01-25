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
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::ints_ref::IntsRef;
use crate::util::CommonUtil;
use once_cell::sync::Lazy;
use rand::Rng;
use std::env;
use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::SystemTime;

/// Methods for manipulating strings.
///
/// # Note
/// This is an internal API.
pub struct StringHelper;
impl StringHelper {
    /// Compares two [`BytesRef`], element by element, and returns the number of elements common to
    /// both arrays (from the start of each). This method assumes `currentTerm` comes after `priorTerm`.
    ///
    /// # Arguments
    ///
    /// * `prior_term` - The first [`BytesRef`] to compare
    /// * `current_term` - The second [`BytesRef`] to compare
    ///
    /// # Returns
    ///
    /// The number of common elements (from the start of each).
    pub fn bytes_difference(
        prior_term: &BytesRef,
        current_term: &BytesRef,
    ) -> Result<i32, LuceneError> {
        let mismatch = CommonUtil::miss_match(
            &prior_term.bytes
                [prior_term.offset as usize..(prior_term.offset + prior_term.length) as usize],
            &current_term.bytes[current_term.offset as usize
                ..(current_term.offset + current_term.length) as usize],
        );

        if mismatch < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "terms out of order: priorTerm={:?}, currentTerm={:?}",
                prior_term, current_term
            )));
        }

        Ok(mismatch)
    }
    /// Returns the length of `current_term` needed for use as a sort key so that
    /// `BytesRef::compare_to()` still returns the same result.
    /// This method assumes `current_term` comes after `prior_term`.
    ///
    /// # Arguments
    ///
    /// * `prior_term` - The first `BytesRef` to compare
    /// * `current_term` - The second `BytesRef` to compare
    ///
    /// # Returns
    ///
    /// The length needed for the sort key.
    pub fn sort_key_length(
        prior_term: &BytesRef,
        current_term: &BytesRef,
    ) -> Result<i32, LuceneError> {
        let difference = Self::bytes_difference(prior_term, current_term)?;
        Ok(difference + 1)
    }

    /// Returns `true` if the given `ref` starts with the given `prefix`. Otherwise returns `false`.
    ///
    /// # Arguments
    ///
    /// * `ref_bytes` - The `byte[]` to test
    /// * `prefix` - The expected prefix as `BytesRef`
    ///
    /// # Returns
    ///
    /// `true` if `ref_bytes` starts with the given `prefix`, otherwise `false`.
    pub fn starts_with_byte_array(ref_bytes: &[u8], prefix: &BytesRef) -> bool {
        // Not long enough to start with the prefix
        if ref_bytes.len() < prefix.length as usize {
            return false;
        }
        let ref_slice = &ref_bytes[0..prefix.length as usize];
        let prefix_slice =
            &prefix.bytes[prefix.offset as usize..(prefix.offset + prefix.length) as usize];
        ref_slice == prefix_slice
    }
    /// Returns `true` if the given `ref` starts with the given `prefix`. Otherwise returns `false`.
    ///
    /// # Arguments
    ///
    /// * `ref_bytes` - The `BytesRef` to test
    /// * `prefix` - The expected prefix as `BytesRef`
    ///
    /// # Returns
    ///
    /// `true` if `ref_bytes` starts with the given `prefix`, otherwise `false`.
    pub fn starts_with_byte_ref(ref_bytes: &BytesRef, prefix: &BytesRef) -> bool {
        // Not long enough to start with the prefix
        if ref_bytes.length < prefix.length {
            return false;
        }

        // Check if the prefix matches
        let ref_slice = &ref_bytes.bytes
            [ref_bytes.offset as usize..(ref_bytes.offset + prefix.length) as usize];
        let prefix_slice =
            &prefix.bytes[prefix.offset as usize..(prefix.offset + prefix.length) as usize];

        ref_slice == prefix_slice
    }

    /// Returns `true` if the `ref` ends with the given `suffix`. Otherwise returns `false`.
    ///
    /// # Arguments
    ///
    /// * `ref` - The `BytesRef` to test
    /// * `suffix` - The expected suffix as `BytesRef`
    ///
    /// # Returns
    ///
    /// `True` if `ref` ends with the given `suffix`, otherwise `false`.
    pub fn ends_with(ref_bytes: &BytesRef, suffix: &BytesRef) -> bool {
        let start_at = ref_bytes.length - suffix.length;
        // Not long enough to start with the suffix
        if start_at < 0 {
            return false;
        }

        let ref_slice = &ref_bytes.bytes[ref_bytes.offset as usize + start_at as usize
            ..(ref_bytes.offset + start_at + suffix.length) as usize];
        let suffix_slice =
            &suffix.bytes[suffix.offset as usize..(suffix.offset + suffix.length) as usize];
        ref_slice == suffix_slice
    }

    /// Returns the MurmurHash3_x86_32 hash.
    /// Original source/tests at <https://github.com/yonik/java_util>
    pub fn murmurhash3_x86_32_with_byte(data: &[u8], offset: usize, len: usize, seed: i32) -> i32 {
        let c1: i32 = 0xcc9e2d51u32 as i32;
        let c2: i32 = 0x1b873593u32 as i32;

        let mut h1 = seed;
        let rounded_end = offset + (len & 0xfffffffc); // round down to 4 byte block

        let mut i = offset;
        while i < rounded_end {
            let k1 = BitUtil::get_i32_le(data, i);
            let mut k1 = k1.wrapping_mul(c1);
            k1 = k1.rotate_left(15);
            k1 = k1.wrapping_mul(c2);

            h1 ^= k1;
            h1 = h1.rotate_left(13);
            h1 = h1.wrapping_mul(5).wrapping_add(0xe6546b64u32 as i32);

            i += 4;
        }

        // tail
        let mut k1 = 0i32;
        match len & 0x03 {
            3 => {
                k1 = (data[rounded_end + 2] as i32) << 16;
                k1 |= (data[rounded_end + 1] as i32) << 8;
                k1 |= data[rounded_end] as i32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            }
            // fallthrough
            2 => {
                k1 |= (data[rounded_end + 1] as i32) << 8;
                k1 |= data[rounded_end] as i32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            }
            // fallthrough
            1 => {
                k1 |= data[rounded_end] as i32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            }
            _ => {}
        }

        // Finalization
        debug_assert!(len <= i32::MAX as usize);
        h1 ^= len as i32;
        // fmix(h1);
        h1 ^= (h1 as u32 >> 16) as i32;
        h1 = h1.wrapping_mul(0x85ebca6bu32 as i32);
        h1 ^= (h1 as u32 >> 13) as i32;
        h1 = h1.wrapping_mul(0xc2b2ae35u32 as i32);

        // Return the final hash value as i32
        h1 ^ ((h1 as u32 >> 16) as i32)
    }
    pub fn murmurhash3_x86_32(bytes: &BytesRef, seed: i32) -> i32 {
        Self::murmurhash3_x86_32_with_byte(
            &bytes.bytes,
            bytes.offset as usize,
            bytes.length as usize,
            seed,
        )
    }

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
    pub fn ints_ref_to_bytes_ref(_ints: IntsRef) -> Result<BytesRef, String> {
        unimplemented!()
    }
}
/// A constant seed used for hashing, intended to prevent hash key collision attacks and ensure
/// reproducibility of test failures if needed.
///
/// This seed is based on a system property `tests.seed` if present, or the current system time
/// if not. It's used as a seed for the MurmurHash3 algorithm to ensure a different salt/seed for
/// each run.
pub static GOOD_FAST_HASH_SEED: Lazy<i32> = Lazy::new(|| {
    if let Ok(prop) = env::var("tests.seed") {
        // If the system property `tests.seed` is set, use it as the seed.
        let mut hasher = DefaultHasher::new();
        prop.hash(&mut hasher);
        hasher.finish() as i32
    } else {
        // Otherwise, fall back to using the current system time in milliseconds.
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i32
    }
});
