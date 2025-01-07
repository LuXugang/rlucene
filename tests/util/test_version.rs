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
use crate::common::my_random;
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::util::version::{
    Version, LATEST, LUCENE_10_0_0, LUCENE_10_1_0, LUCENE_11_0_0, LUCENE_CURRENT,
};
use std::hash::{DefaultHasher, Hash, Hasher};

#[allow(dead_code)] // for quick search
struct TestVersion;

#[test]
fn test_on_or_after() -> Result<(), TestError> {
    let versions = vec![&*LUCENE_10_0_0, &*LUCENE_10_1_0, &*LUCENE_11_0_0];

    for version in versions {
        assert!(
            LATEST.on_or_after(version),
            "LATEST must always be on_or_after({})",
            version
        );
    }

    assert!(LUCENE_11_0_0.on_or_after(&Version::from_bits(9, 0, 0)?));
    assert!(LUCENE_11_0_0.on_or_after(&LUCENE_10_0_0));
    assert!(LUCENE_11_0_0.on_or_after(&LUCENE_10_1_0));
    Ok(())
}
#[test]
fn test_to_string() -> Result<(), TestError> {
    assert_eq!(Version::from_bits(9, 0, 0)?.to_string(), "9.0.0");
    assert_eq!(LUCENE_10_0_0.to_string(), "10.0.0");
    assert_eq!(LUCENE_10_1_0.to_string(), "10.1.0");
    assert_eq!(LUCENE_11_0_0.to_string(), "11.0.0");
    Ok(())
}

#[test]
fn test_parse_leniently() -> Result<(), TestError> {
    assert_eq!(Version::parse_leniently("11.0")?, *LUCENE_11_0_0);
    assert_eq!(Version::parse_leniently("11.0.0")?, *LUCENE_11_0_0);
    assert_eq!(Version::parse_leniently("LUCENE_11_0")?, *LUCENE_11_0_0);
    assert_eq!(Version::parse_leniently("LUCENE_11_0_0")?, *LUCENE_11_0_0);

    assert_eq!(Version::parse_leniently("10.0")?, *LUCENE_10_0_0);
    assert_eq!(Version::parse_leniently("10.0.0")?, *LUCENE_10_0_0);
    assert_eq!(Version::parse_leniently("LUCENE_10_0")?, *LUCENE_10_0_0);
    assert_eq!(Version::parse_leniently("LUCENE_10_0_0")?, *LUCENE_10_0_0);

    assert_eq!(Version::parse_leniently("LATEST")?, *LATEST);
    assert_eq!(Version::parse_leniently("latest")?, *LATEST);
    assert_eq!(Version::parse_leniently("LUCENE_CURRENT")?, *LATEST);
    assert_eq!(Version::parse_leniently("lucene_current")?, *LATEST);

    Ok(())
}
#[test]
fn test_parse_leniently_exceptions() {
    let result = Version::parse_leniently("LUCENE");
    assert!(result.is_err(), "Expected 'LUCENE' to return an error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("LUCENE"),
        "Expected error message to contain 'LUCENE', got: {}",
        error
    );

    let result = Version::parse_leniently("LUCENE_610");
    assert!(result.is_err(), "Expected 'LUCENE_610' to return an error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("LUCENE_610"),
        "Expected error message to contain 'LUCENE_610', got: {}",
        error
    );

    let result = Version::parse_leniently("LUCENE61");
    assert!(result.is_err(), "Expected 'LUCENE61' to return an error");
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("LUCENE61"),
        "Expected error message to contain 'LUCENE61', got: {}",
        error
    );

    let result = Version::parse_leniently("LUCENE_7.0.0");
    assert!(
        result.is_err(),
        "Expected 'LUCENE_7.0.0' to return an error"
    );
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("LUCENE_7.0.0"),
        "Expected error message to contain 'LUCENE_7.0.0', got: {}",
        error
    );
}
#[test]
fn test_parse_leniently_on_all_constants() -> Result<(), TestError> {
    let versions = vec![
        (&*LUCENE_10_0_0, "LUCENE_10_0_0"),
        (&*LUCENE_10_1_0, "LUCENE_10_1_0"),
        (&*LUCENE_11_0_0, "LUCENE_11_0_0"),
        (&*LATEST, "LATEST"),
        (&*LUCENE_CURRENT, "LUCENE_CURRENT"),
    ];

    let mut at_least_one = false;

    for (version, name) in versions {
        at_least_one = true;
        assert_eq!(
            *version,
            Version::parse_leniently(name)?,
            "parse_leniently failed for {}",
            name
        );
        assert_eq!(
            *version,
            Version::parse_leniently(&name.to_lowercase())?,
            "parse_leniently failed for {}",
            name.to_lowercase()
        );
        assert_eq!(
            *version,
            Version::parse_leniently(&version.to_string())?,
            "parse_leniently failed for {}",
            version
        );
    }

    assert!(at_least_one, "Expected at least one version to be tested");
    Ok(())
}
#[test]
fn test_parse() -> Result<(), TestError> {
    assert_eq!(Version::parse("10.0.0")?, *LUCENE_10_0_0);
    assert_eq!(Version::parse("11.0.0")?, *LUCENE_11_0_0);

    assert_eq!(Version::parse("1.0")?.major, 1);
    assert_eq!(Version::parse("7.0.0")?.major, 7);
    Ok(())
}

#[test]
fn test_forwards_compatibility() -> Result<(), TestError> {
    assert!(Version::parse("11.10.20")?.on_or_after(&LUCENE_11_0_0));
    assert!(Version::parse("10.10.20")?.on_or_after(&LUCENE_10_0_0));
    assert!(Version::parse("9.10.20")?.on_or_after(&Version::from_bits(9, 0, 0)?));
    Ok(())
}
#[test]
fn test_parse_exceptions() {
    let inputs = vec![
        "LUCENE_7_0_0",
        "7.256",
        "7.-1",
        "7.1.256",
        "7.1.-1",
        "7.1.1.3",
        "7.1.1.-1",
        "7.1.1.1",
        "7.1.1.2",
        "7.0.0.0",
        "7.0.0.1.42",
        "7..0.1",
    ];

    for input in inputs {
        check_parse_error(input);
    }
}
fn check_parse_error(input: &str) {
    let result = Version::parse(input);
    assert!(
        result.is_err(),
        "Expected '{}' to return an error, but it succeeded",
        input
    );
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains(input),
        "Expected error message to contain '{}', got: {}",
        input,
        error
    );
}
#[test]
fn test_non_floating_point_compliant_version_numbers() -> Result<(), TestError> {
    let version800 = Version::parse("8.0.0")?;
    assert!(
        Version::parse("8.10.0")?.on_or_after(&version800),
        "Expected 8.10.0 to be on or after 8.0.0"
    );
    assert!(
        Version::parse("8.10.0")?.on_or_after(&Version::parse("8.9.255")?),
        "Expected 8.10.0 to be on or after 8.9.255"
    );
    assert!(
        Version::parse("8.128.0")?.on_or_after(&version800),
        "Expected 8.128.0 to be on or after 8.0.0"
    );
    assert!(
        Version::parse("8.255.0")?.on_or_after(&version800),
        "Expected 8.255.0 to be on or after 8.0.0"
    );

    let version400 = Version::parse("4.0.0")?;
    assert!(
        version800.on_or_after(&version400),
        "Expected 8.0.0 to be on or after 4.0.0"
    );
    assert!(
        Version::parse("8.128.0")?.on_or_after(&version400),
        "Expected 8.128.0 to be on or after 4.0.0"
    );
    assert!(
        !version400.on_or_after(&version800),
        "Expected 4.0.0 not to be on or after 8.0.0"
    );

    Ok(())
}

#[test]
fn test_equals_hash_code() -> Result<(), TestError> {
    let mut random = my_random("test_equals_hash_code".to_string());

    let version = format!(
        "{}.{}.{}",
        4 + random.gen_range(0..1),
        random.gen_range(0..10),
        random.gen_range(0..10)
    );

    let v1 = Version::parse_leniently(&version)?;
    let v2 = Version::parse_leniently(&version)?;
    let mut hasher1 = DefaultHasher::new();
    let mut hasher2 = DefaultHasher::new();
    v1.hash(&mut hasher1);
    v2.hash(&mut hasher2);
    let v1_hash_value = hasher1.finish();
    assert_eq!(v1_hash_value, hasher2.finish());
    assert_eq!(v1, v2);
    let iterations = 10 + random.gen_range(0..20);
    for _ in 0..iterations {
        let v = format!(
            "{}.{}.{}",
            4 + random.gen_range(0..1),
            random.gen_range(0..10),
            random.gen_range(0..10)
        );

        if v == version {
            let version = Version::parse_leniently(&v)?;
            let mut hasher_3 = DefaultHasher::new();
            version.hash(&mut hasher_3);

            assert_eq!(
                hasher_3.finish(),
                v1_hash_value,
                "Expected hashCode of parsed '{}' to match",
                v
            );
            assert_eq!(
                Version::parse_leniently(&v)?,
                v1,
                "Expected parsed '{}' to equal v1",
                v
            );
        } else {
            assert_ne!(
                Version::parse_leniently(&v)?,
                v1,
                "Expected parsed '{}' not to equal v1",
                v
            );
        }
    }

    Ok(())
}
