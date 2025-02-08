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
use crate::test::util::lucene_test_case::new_bytes_ref_from_string;
use crate::test::util::lucene_test_case::random;
use crate::test::util::test_error::TestError;
use crate::util::error::lucene_error::LuceneError;
use crate::util::StringHelper;

#[allow(dead_code)] // for quick search
pub struct TestStringHelper;
#[test]
fn test_bytes_difference() -> Result<(), TestError> {
    let mut random = random();
    let left = new_bytes_ref_from_string(&mut random, "foobar")?;
    let right = new_bytes_ref_from_string(&mut random, "foozo")?;
    assert_eq!(StringHelper::bytes_difference(&left, &right)?, 3);

    assert_eq!(
        StringHelper::bytes_difference(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "for")?
        )?,
        2
    );
    assert_eq!(
        StringHelper::bytes_difference(
            &new_bytes_ref_from_string(&mut random, "foo1234")?,
            &new_bytes_ref_from_string(&mut random, "for1234")?
        )?,
        2
    );
    assert_eq!(
        StringHelper::bytes_difference(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "fz")?
        )?,
        1
    );
    assert_eq!(
        StringHelper::bytes_difference(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "g")?
        )?,
        0
    );
    assert_eq!(
        StringHelper::bytes_difference(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "food")?
        )?,
        3
    );
    assert_eq!(
        StringHelper::bytes_difference(
            &new_bytes_ref_from_string(&mut random, "food")?,
            &new_bytes_ref_from_string(&mut random, "foo")?
        )?,
        3
    );
    // we can detect terms are out of order if we see a duplicate
    let result = StringHelper::bytes_difference(
        &new_bytes_ref_from_string(&mut random, "ab")?,
        &new_bytes_ref_from_string(&mut random, "ab")?,
    );
    assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
    Ok(())
}
#[test]
fn test_starts_with() -> Result<(), TestError> {
    let mut random = random();
    let ref_bytes = new_bytes_ref_from_string(&mut random, "foobar")?;
    let slice = new_bytes_ref_from_string(&mut random, "foo")?;
    assert!(StringHelper::starts_with_byte_ref(&ref_bytes, &slice));
    Ok(())
}
#[test]
fn test_ends_with() -> Result<(), TestError> {
    let mut random = random();
    let ref_bytes = new_bytes_ref_from_string(&mut random, "foobar")?;
    let slice = new_bytes_ref_from_string(&mut random, "bar")?;
    assert!(StringHelper::ends_with(&ref_bytes, &slice));
    Ok(())
}
#[test]
fn test_starts_with_whole() -> Result<(), TestError> {
    let mut random = random();
    let ref_bytes = new_bytes_ref_from_string(&mut random, "foobar")?;
    let slice = new_bytes_ref_from_string(&mut random, "foobar")?;
    assert!(StringHelper::starts_with_byte_ref(&ref_bytes, &slice));
    Ok(())
}
#[test]
fn test_ends_with_whole() -> Result<(), TestError> {
    let mut random = random();
    let ref_bytes = new_bytes_ref_from_string(&mut random, "foobar")?;
    let slice = new_bytes_ref_from_string(&mut random, "foobar")?;
    assert!(StringHelper::ends_with(&ref_bytes, &slice));
    Ok(())
}
#[test]
fn test_murmur_hash3() -> Result<(), TestError> {
    let mut random = random();
    // Hashes computed using murmur3_32 from https://code.google.com/p/pyfasthash
    assert_eq!(
        StringHelper::murmurhash3_x86_32(&new_bytes_ref_from_string(&mut random, "foo")?, 0),
        0xf6a5c420u32 as i32
    );
    assert_eq!(
        StringHelper::murmurhash3_x86_32(&new_bytes_ref_from_string(&mut random, "foo")?, 16),
        0xcd018ef6u32 as i32
    );
    assert_eq!(
        StringHelper::murmurhash3_x86_32(
            &new_bytes_ref_from_string(
                &mut random,
                "You want weapons? We're in a library! Books! The best weapons in the world!"
            )?,
            0
        ),
        0x111e7435
    );
    assert_eq!(
        StringHelper::murmurhash3_x86_32(
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
fn test_sort_key_length() -> Result<(), TestError> {
    let mut random = random();
    assert_eq!(
        StringHelper::sort_key_length(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "for")?
        )?,
        3
    );
    assert_eq!(
        StringHelper::sort_key_length(
            &new_bytes_ref_from_string(&mut random, "foo1234")?,
            &new_bytes_ref_from_string(&mut random, "for1234")?
        )?,
        3
    );
    assert_eq!(
        StringHelper::sort_key_length(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "fz")?
        )?,
        2
    );
    assert_eq!(
        StringHelper::sort_key_length(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "g")?
        )?,
        1
    );
    assert_eq!(
        StringHelper::sort_key_length(
            &new_bytes_ref_from_string(&mut random, "foo")?,
            &new_bytes_ref_from_string(&mut random, "food")?
        )?,
        4
    );

    // We can detect terms are out of order if we see a duplicate
    let result = StringHelper::sort_key_length(
        &new_bytes_ref_from_string(&mut random, "ab")?,
        &new_bytes_ref_from_string(&mut random, "ab")?,
    );
    assert!(
        result.is_err(),
        "Expected an error when the terms are equal"
    );

    Ok(())
}
