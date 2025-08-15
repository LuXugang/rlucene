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
use std::env;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::SystemTime;

use once_cell::sync::Lazy;
use rand::Rng;

use crate::index::BytesRef;
use crate::util::CoreHelper;
use crate::util::access::SharedAccessVec;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::ints_ref::IntsRef;
use crate::with_other;

/// Methods for manipulating strings.
///
/// # Note
/// This is an internal API.
pub struct StringHelper;
impl StringHelper {
    /// Compares two [`BytesRef`], element by element, and returns the number of
    /// elements common to both arrays (from the start of each). This method
    /// assumes `currentTerm` comes after `priorTerm`.
    ///
    /// # Arguments
    ///
    /// * `prior_term` - The first [`BytesRef`] to compare
    /// * `current_term` - The second [`BytesRef`] to compare
    ///
    /// # Returns
    ///
    /// The number of common elements (from the start of each).
    pub fn bytes_difference<AV: SharedAccessVec<u8>>(
        prior_term: &BytesRef<AV>,
        current_term: &BytesRef<AV>,
    ) -> Result<i32> {
        with_other!(
            prior_term.bytes,
            current_term.bytes,
            |prior_term_bytes, current_term_bytes| {
                let mismatch = CoreHelper::miss_match(
                    &prior_term_bytes[prior_term.offset..(prior_term.offset + prior_term.length)],
                    &current_term_bytes
                        [current_term.offset..(current_term.offset + current_term.length)],
                );

                if mismatch < 0 {
                    return Err(LuceneError::illegal_argument(format!(
                        "terms out of order: priorTerm={prior_term}, currentTerm={current_term}"
                    )));
                }
                Ok(mismatch)
            }
        )
    }
    /// Returns the length of `current_term` needed for use as a sort key so
    /// that `BytesRef::compare_to()` still returns the same result.
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
    pub fn sort_key_length<AV: SharedAccessVec<u8>>(
        prior_term: &BytesRef<AV>,
        current_term: &BytesRef<AV>,
    ) -> Result<i32> {
        let difference = Self::bytes_difference(prior_term, current_term)?;
        Ok(difference + 1)
    }

    /// Returns `true` if the given `ref` starts with the given `prefix`.
    /// Otherwise returns `false`.
    ///
    /// # Arguments
    ///
    /// * `ref_bytes` - The `byte[]` to test
    /// * `prefix` - The expected prefix as `BytesRef`
    ///
    /// # Returns
    ///
    /// `true` if `ref_bytes` starts with the given `prefix`, otherwise `false`.
    pub fn starts_with_byte_array<AV>(ref_bytes: &[u8], prefix: &BytesRef<AV>) -> bool
    where
        AV: SharedAccessVec<u8>,
    {
        // Not long enough to start with the prefix
        if ref_bytes.len() < prefix.length {
            return false;
        }
        let ref_slice = &ref_bytes[0..prefix.length];
        prefix.bytes.access(|bytes| {
            let prefix_slice = &bytes[prefix.offset..prefix.offset + prefix.length];
            ref_slice == prefix_slice
        })
    }
    /// Returns `true` if the given `ref` starts with the given `prefix`.
    /// Otherwise returns `false`.
    ///
    /// # Arguments
    ///
    /// * `ref_bytes` - The `BytesRef` to test
    /// * `prefix` - The expected prefix as `BytesRef`
    ///
    /// # Returns
    ///
    /// `true` if `ref_bytes` starts with the given `prefix`, otherwise `false`.
    pub fn starts_with_byte_ref<AV>(ref_bytes: &BytesRef<AV>, prefix: &BytesRef<AV>) -> bool
    where
        AV: SharedAccessVec<u8>,
    {
        with_other!(
            ref_bytes.bytes,
            prefix.bytes,
            |ref_bytes_bytes, prefix_bytes| {
                Self::starts_with(
                    ref_bytes_bytes,
                    ref_bytes.offset,
                    ref_bytes.length,
                    prefix_bytes,
                    prefix.offset,
                    prefix.length,
                )
            }
        )
    }
    pub fn starts_with(
        ref_bytes: &[u8],
        ref_offset: usize,
        ref_length: usize,
        prefix: &[u8],
        prefix_offset: usize,
        prefix_length: usize,
    ) -> bool {
        // Not long enough to start with the prefix
        if ref_length < prefix_length {
            return false;
        }

        // Check if the prefix matches
        let ref_slice = &ref_bytes[ref_offset..(ref_offset + prefix_length)];
        let prefix_slice = &prefix[prefix_offset..(prefix_offset + prefix_length)];

        ref_slice == prefix_slice
    }

    /// Returns `true` if the `ref` ends with the given `suffix`. Otherwise
    /// returns `false`.
    ///
    /// # Arguments
    ///
    /// * `ref` - The `BytesRef` to test
    /// * `suffix` - The expected suffix as `BytesRef`
    ///
    /// # Returns
    ///
    /// `True` if `ref` ends with the given `suffix`, otherwise `false`.
    pub fn ends_with<AV>(ref_bytes: &BytesRef<AV>, suffix: &BytesRef<AV>) -> bool
    where
        AV: SharedAccessVec<u8>,
    {
        with_other!(
            ref_bytes.bytes,
            suffix.bytes,
            |ref_bytes_bytes, suffix_bytes| {
                // Not long enough to start with the suffix
                if ref_bytes.length < suffix.length {
                    return false;
                }
                let start_at = ref_bytes.length - suffix.length;

                let ref_slice = &ref_bytes_bytes
                    [ref_bytes.offset + start_at..(ref_bytes.offset + start_at + suffix.length)];
                let suffix_slice = &suffix_bytes[suffix.offset..(suffix.offset + suffix.length)];
                ref_slice == suffix_slice
            }
        )
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
            },
            // fallthrough
            2 => {
                k1 |= (data[rounded_end + 1] as i32) << 8;
                k1 |= data[rounded_end] as i32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            },
            // fallthrough
            1 => {
                k1 |= data[rounded_end] as i32;
                k1 = k1.wrapping_mul(c1);
                k1 = k1.rotate_left(15);
                k1 = k1.wrapping_mul(c2);
                h1 ^= k1;
            },
            _ => {},
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
    pub fn murmurhash3_x86_32<AV>(bytes: &BytesRef<AV>, seed: i32) -> i32
    where
        AV: SharedAccessVec<u8>,
    {
        bytes.bytes.access(|bytes_ref| {
            Self::murmurhash3_x86_32_with_byte(bytes_ref, bytes.offset, bytes.length, seed)
        })
    }

    pub const ID_LENGTH: usize = 16;
    pub fn random_id() -> [u8; StringHelper::ID_LENGTH] {
        let mut rng = rand::rng();
        rng.random::<[u8; StringHelper::ID_LENGTH]>()
    }
    /// Helper method to render an ID as a string for debugging.
    ///
    /// Returns the string `"null"` if the ID is `None`. Otherwise, returns a
    /// string representation for debugging. Never throws an exception. The
    /// returned string may indicate if the ID is definitely invalid.
    pub fn id_to_string(id: Option<&[u8; StringHelper::ID_LENGTH]>) -> String {
        if let Some(id) = id {
            let big_int = num_bigint::BigUint::from_bytes_be(id);
            big_int.to_str_radix(36)
        } else {
            "(null)".to_string()
        }
    }
    /// Converts each `i32` in the incoming [`IntsRef`] to a `u8` in the
    /// returned [`BytesRef`].
    ///
    /// Throws an [`IllegalArgument`](crate::util::error::IllegalArgumentError)
    /// if any int value is out of bounds for a byte.
    pub fn ints_ref_to_bytes_ref<AV: SharedAccessVec<i32>, AV1: SharedAccessVec<u8>>(
        ints: &IntsRef<AV>,
    ) -> Result<BytesRef<AV1>> {
        let mut bytes = AV1::with_capacity(ints.length);
        for i in 0..ints.length {
            ints.ints.access(|v| {
                let x = v[ints.offset + i];
                if !(0..=255).contains(&x) {
                    return Err(LuceneError::illegal_argument(format!(
                        "int at pos={i} with value={x} is out-of-bounds for byte"
                    )));
                }
                bytes.access_mut(|v| {
                    v.push(x as u8);
                });
                // Help the compiler infer types.
                Ok::<(), LuceneError>(())
            })?;
        }
        Ok(BytesRef::from_bytes(bytes))
    }
}
/// A constant seed used for hashing, intended to prevent hash key collision
/// attacks and ensure reproducibility of test failures if needed.
///
/// This seed is based on a system property `tests.seed` if present, or the
/// current system time if not. It's used as a seed for the MurmurHash3
/// algorithm to ensure a different salt/seed for each run.
pub static GOOD_FAST_HASH_SEED: Lazy<i32> = Lazy::new(|| {
    if let Ok(prop) = env::var("tests.seed") {
        // If the system property `tests.seed` is set, use it as the seed.
        let mut hasher = DefaultHasher::new();
        prop.hash(&mut hasher);
        hasher.finish() as i32
    } else {
        // Otherwise, fall back to using the current system time in
        // milliseconds.
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i32
    }
});

#[cfg(test)]
mod tests {
    use crate::index::BytesRef;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_bytes_ref_from_string, random,
    };
    use crate::util::StringHelper;
    use crate::util::error::lucene_error::{LuceneError, Result};

    #[allow(dead_code)] // for quick search
    pub struct TestStringHelper;
    #[test]
    fn test_bytes_difference() -> Result<()> {
        let mut random = random();
        let left: BytesRef<Vec<u8>> = new_bytes_ref_from_string(&mut random, "foobar")?;
        let right = new_bytes_ref_from_string(&mut random, "foozo")?;
        assert_eq!(StringHelper::bytes_difference(&left, &right)?, 3);

        assert_eq!(
            StringHelper::bytes_difference::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "for")?
            )?,
            2
        );
        assert_eq!(
            StringHelper::bytes_difference::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo1234")?,
                &new_bytes_ref_from_string(&mut random, "for1234")?
            )?,
            2
        );
        assert_eq!(
            StringHelper::bytes_difference::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "fz")?
            )?,
            1
        );
        assert_eq!(
            StringHelper::bytes_difference::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "g")?
            )?,
            0
        );
        assert_eq!(
            StringHelper::bytes_difference::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "food")?
            )?,
            3
        );
        assert_eq!(
            StringHelper::bytes_difference::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "food")?,
                &new_bytes_ref_from_string(&mut random, "foo")?
            )?,
            3
        );
        // we can detect terms are out of order if we see a duplicate
        let result = StringHelper::bytes_difference::<Vec<u8>>(
            &new_bytes_ref_from_string(&mut random, "ab")?,
            &new_bytes_ref_from_string(&mut random, "ab")?,
        );
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
    #[test]
    fn test_starts_with() -> Result<()> {
        let mut random = random();
        let ref_bytes: BytesRef<Vec<u8>> = new_bytes_ref_from_string(&mut random, "foobar")?;
        let slice = new_bytes_ref_from_string(&mut random, "foo")?;
        assert!(StringHelper::starts_with_byte_ref(&ref_bytes, &slice));
        Ok(())
    }
    #[test]
    fn test_ends_with() -> Result<()> {
        let mut random = random();
        let ref_bytes: BytesRef<Vec<u8>> = new_bytes_ref_from_string(&mut random, "foobar")?;
        let slice = new_bytes_ref_from_string(&mut random, "bar")?;
        assert!(StringHelper::ends_with(&ref_bytes, &slice));
        Ok(())
    }
    #[test]
    fn test_starts_with_whole() -> Result<()> {
        let mut random = random();
        let ref_bytes: BytesRef<Vec<u8>> = new_bytes_ref_from_string(&mut random, "foobar")?;
        let slice = new_bytes_ref_from_string(&mut random, "foobar")?;
        assert!(StringHelper::starts_with_byte_ref(&ref_bytes, &slice));
        Ok(())
    }
    #[test]
    fn test_ends_with_whole() -> Result<()> {
        let mut random = random();
        let ref_bytes: BytesRef<Vec<u8>> = new_bytes_ref_from_string(&mut random, "foobar")?;
        let slice = new_bytes_ref_from_string(&mut random, "foobar")?;
        assert!(StringHelper::ends_with(&ref_bytes, &slice));
        Ok(())
    }
    #[test]
    fn test_murmur_hash3() -> Result<()> {
        let mut random = random();
        // Hashes computed using murmur3_32 from https://code.google.com/p/pyfasthash
        assert_eq!(
            StringHelper::murmurhash3_x86_32::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                0
            ),
            0xf6a5c420u32 as i32
        );
        assert_eq!(
            StringHelper::murmurhash3_x86_32::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                16
            ),
            0xcd018ef6u32 as i32
        );
        assert_eq!(
            StringHelper::murmurhash3_x86_32::<Vec<u8>>(
                &new_bytes_ref_from_string(
                    &mut random,
                    "You want weapons? We're in a library! Books! The best weapons in the world!"
                )?,
                0
            ),
            0x111e7435
        );
        assert_eq!(
            StringHelper::murmurhash3_x86_32::<Vec<u8>>(
                &new_bytes_ref_from_string(
                    &mut random,
                    "You want weapons? We're in a library! Books! The best weapons in the world!"
                )?,
                3476
            ),
            0x2c628cd0
        );
        Ok(())
    }
    #[test]
    fn test_sort_key_length() -> Result<()> {
        let mut random = random();
        assert_eq!(
            StringHelper::sort_key_length::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "for")?
            )?,
            3
        );
        assert_eq!(
            StringHelper::sort_key_length::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo1234")?,
                &new_bytes_ref_from_string(&mut random, "for1234")?
            )?,
            3
        );
        assert_eq!(
            StringHelper::sort_key_length::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "fz")?
            )?,
            2
        );
        assert_eq!(
            StringHelper::sort_key_length::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "g")?
            )?,
            1
        );
        assert_eq!(
            StringHelper::sort_key_length::<Vec<u8>>(
                &new_bytes_ref_from_string(&mut random, "foo")?,
                &new_bytes_ref_from_string(&mut random, "food")?
            )?,
            4
        );

        // We can detect terms are out of order if we see a duplicate
        let result = StringHelper::sort_key_length::<Vec<u8>>(
            &new_bytes_ref_from_string(&mut random, "ab")?,
            &new_bytes_ref_from_string(&mut random, "ab")?,
        );
        assert!(
            result.is_err(),
            "Expected an error when the terms are equal"
        );

        Ok(())
    }
}
