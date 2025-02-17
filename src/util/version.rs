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
use crate::util::error::illegal_argument::IllegalArgumentError;
use crate::util::error::illegal_state::IllegalStateError;
use crate::util::error::parse::Parse;
use crate::util::strict_string_tokenizer::StrictStringTokenizer;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use thiserror::Error;

pub static LUCENE_10_0_0: Lazy<Version> = Lazy::new(|| Version::new(10, 0, 0).unwrap());

/// Match settings and bugs in Lucene's 10.1.0 release.
pub static LUCENE_10_1_0: Lazy<Version> = Lazy::new(|| Version::new(10, 1, 0).unwrap());

/// Match settings and bugs in Lucene's 11.0.0 release.
pub static LUCENE_11_0_0: Lazy<Version> = Lazy::new(|| Version::new(11, 0, 0).unwrap());

/// # Warning
/// If you use this setting, and then upgrade to a newer release of Lucene, sizable
/// changes may happen. If backwards compatibility is important, you should instead explicitly
/// specify an actual version.
///
/// If you use this constant, you may need to **re-index all of your documents** when
/// upgrading Lucene, as the way text is indexed may have changed. Additionally, you may need to
/// **re-test your entire application** to ensure it behaves as expected, as some defaults may
/// have changed and may break functionality in your application.
pub static LATEST: Lazy<Version> = Lazy::new(|| LUCENE_11_0_0.clone());
pub static LUCENE_CURRENT: Lazy<Version> = Lazy::new(|| LATEST.clone());
pub static MIN_SUPPORTED_MAJOR: Lazy<i32> = Lazy::new(|| LATEST.major - 1);
/// Used by certain classes to match version compatibility across releases of Lucene.
///
/// # Warning
/// When changing the version parameter that you supply to components in Lucene,
/// do not simply change the version at search-time, but instead also adjust your indexing code to
/// match, and re-index.
#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Debug, Hash)]
pub struct Version {
    /// Major version, the difference between stable and trunk.
    pub major: i32,
    /// Minor version, incremented within the stable branch.
    pub minor: i32,
    /// Bugfix number, incremented on release branches.
    pub bug_fix: i32,
    /// Prerelease version, currently 0 (alpha), 1 (beta), or 2 (final).
    pub(crate) prerelease: i32,
    encoded_value: i32,
}
impl Version {
    fn new(major: i32, minor: i32, bug_fix: i32) -> Result<Version, IllegalArgumentError> {
        Version::with_prerelease(major, minor, bug_fix, 0)
    }
    fn with_prerelease(
        major: i32,
        minor: i32,
        bug_fix: i32,
        prerelease: i32,
    ) -> Result<Version, IllegalArgumentError> {
        // NOTE: do not enforce major version so we remain future proof, except to
        // make sure it fits in the 8 bits we encode it into:
        if !(0..=255).contains(&major) {
            return Err(IllegalArgumentError::new(format!(
                "Illegal major version: {}",
                major
            )));
        }
        if !(0..=255).contains(&minor) {
            return Err(IllegalArgumentError::new(format!(
                "Illegal minor version: {}",
                minor
            )));
        }
        if !(0..=255).contains(&bug_fix) {
            return Err(IllegalArgumentError::new(format!(
                "Illegal bug fix version: {}",
                bug_fix
            )));
        }
        if !(0..=2).contains(&prerelease) {
            return Err(IllegalArgumentError::new(format!(
                "Illegal pre-release version: {}",
                prerelease
            )));
        }
        if prerelease != 0 && (minor != 0 || bug_fix != 0) {
            return Err(IllegalArgumentError::new(format!("Prerelease version only supported with major release (got prerelease: {}, minor: {}, bug_fix: {})", prerelease, minor, bug_fix)));
        }
        let encoded_value = (major << 18) | (minor << 10) | (bug_fix << 2) | prerelease;
        debug_assert!(Self::encoded_is_valid(
            major,
            minor,
            bug_fix,
            prerelease,
            encoded_value
        ));
        Ok(Version {
            major,
            minor,
            bug_fix,
            prerelease,
            encoded_value,
        })
    }
    /// Returns true if this version is the same or after the version from the argument.
    pub fn on_or_after(&self, other: &Version) -> bool {
        self.encoded_value >= other.encoded_value
    }
    /// Parses a version number of the form `"major.minor.bugfix.prerelease"`.
    ///
    /// The `.bugfix` and `.prerelease` parts are optional. Note that this is
    /// forwards compatible: the parsed version does not have to exist as a constant.
    ///
    /// # Note
    /// This is an internal API.
    pub fn parse(version: &str) -> Result<Version, VersionError> {
        let mut tokens = StrictStringTokenizer::new(version, '.');
        if !tokens.has_more_tokens() {
            return Err(VersionError::parse_error_with_pos(
                format!(
                    "Version is not in form major.minor.bugfix(.prerelease) (got: {})",
                    version
                ),
                0,
            ));
        }
        let mut token = tokens.next_token()?;
        let major = token.parse::<i32>();
        if major.is_err() {
            return Err(VersionError::parse_int_error(
                format!(
                    "Failed to parse major version from {} (got: {})",
                    token, version
                ),
                major.unwrap_err(),
            ));
        }
        token = tokens.next_token()?;
        let minor = token.parse::<i32>();
        if minor.is_err() {
            return Err(VersionError::parse_int_error(
                format!(
                    "Failed to parse minor version from {} (got: {})",
                    token, version
                ),
                minor.unwrap_err(),
            ));
        }
        let mut bug_fix_value: i32 = 0;
        if tokens.has_more_tokens() {
            token = tokens.next_token()?;
            let bug_fix = token.parse::<i32>();
            if bug_fix.is_err() {
                return Err(VersionError::parse_int_error(
                    format!(
                        "Failed to parse bug fix version from {} (got: {})",
                        token, version
                    ),
                    bug_fix.unwrap_err(),
                ));
            }
            bug_fix_value = bug_fix.unwrap();
        }
        let mut prerelease_value: i32 = 0;
        if tokens.has_more_tokens() {
            token = tokens.next_token()?;
            let prerelease = token.parse::<i32>();
            if prerelease.is_err() {
                return Err(VersionError::parse_int_error(
                    format!(
                        "Failed to parse pre-release version from {} (got: {})",
                        token, version
                    ),
                    prerelease.unwrap_err(),
                ));
            }
            prerelease_value = prerelease.unwrap();
            if prerelease_value == 0 {
                return Err(VersionError::parse_error_with_pos(
                    format!(
                        "Invalid value {}  for prerelease; should be 1 or 2 (got: {})",
                        prerelease_value, version
                    ),
                    0,
                ));
            }
            if tokens.has_more_tokens() {
                // too many tokens!
                return Err(VersionError::parse_error_with_pos(
                    format!(
                        "Version is not in form major.minor.bugfix(.prerelease) (got: {})",
                        version
                    ),
                    0,
                ));
            }
        }
        let result = Version::with_prerelease(
            major.unwrap(),
            minor.unwrap(),
            bug_fix_value,
            prerelease_value,
        );
        if result.is_err() {
            return Err(VersionError::parse_error_with_error(
                format!("failed to parse version string {}", version),
                result.unwrap_err(),
            ));
        }
        debug_assert!(result.is_ok());
        Ok(result?)
    }
    /// Parses the given version number as a constant or dot-based version.
    ///
    /// This method allows using `"LUCENE_X_Y"` constant names, or version numbers in the
    /// format `"x.y.z"`.
    ///
    /// # Note
    /// This is an internal API.
    pub fn parse_leniently(version: &str) -> Result<Version, VersionError> {
        let version_orig = version.to_string();
        let version_upper = version.to_uppercase();

        match version_upper.as_str() {
            "LATEST" | "LUCENE_CURRENT" => Ok(LATEST.clone()),
            _ => {
                let mut version = version_upper.clone();
                let patterns = [
                    (r"^LUCENE_(\d+)_(\d+)_(\d+)$", "$1.$2.$3"),
                    (r"^LUCENE_(\d+)_(\d+)$", "$1.$2.0"),
                    (r"^LUCENE_(\d)(\d)$", "$1.$2.0"),
                ];

                for (pattern, replacement) in patterns.iter() {
                    let re = Regex::new(pattern).unwrap();
                    version = re.replace_all(&version, *replacement).to_string();
                }

                // Try parsing the modified version string
                match Self::parse(&version) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(VersionError::parse_error_with_pos(
                        format!(
                            "failed to parse lenient version string {}: {}",
                            version_orig, e
                        ),
                        0,
                    )),
                }
            }
        }
    }
    /// Returns a new version based on raw numbers.
    ///
    /// # Note
    /// This is an internal API.
    pub fn from_bits(
        major: i32,
        minor: i32,
        bug_fix: i32,
    ) -> Result<Version, IllegalArgumentError> {
        Version::new(major, minor, bug_fix)
    }

    fn encoded_is_valid(
        major: i32,
        minor: i32,
        bug_fix: i32,
        prerelease: i32,
        encoded_value: i32,
    ) -> bool {
        debug_assert_eq!(major, ((encoded_value as u32 >> 18) & 0xFF) as i32);
        debug_assert_eq!(minor, ((encoded_value as u32 >> 10) & 0xFF) as i32);
        debug_assert_eq!(bug_fix, ((encoded_value as u32 >> 2) & 0xFF) as i32);
        debug_assert_eq!(prerelease, encoded_value & 0x03);
        true
    }
}
impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.prerelease == 0 {
            write!(f, "{}.{}.{}", self.major, self.minor, self.bug_fix)
        } else {
            write!(
                f,
                "{}.{}.{}.{}",
                self.major, self.minor, self.bug_fix, self.prerelease
            )
        }
    }
}

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("{0}")]
    IllegalState(#[from] IllegalStateError),

    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgumentError),

    #[error("{0}")]
    Parse(#[from] Parse),

    #[error("{message}: {source}")]
    ParseIntError {
        message: String,
        #[source]
        source: ParseIntError,
    },
}

impl VersionError {
    pub fn parse_error_with_pos(msg: impl Into<String>, position: i32) -> Self {
        VersionError::Parse(Parse::new(msg, position))
    }
    pub fn parse_error_with_error(msg: impl Into<String>, error: IllegalArgumentError) -> Self {
        VersionError::Parse(Parse::with_error(msg, Option::from(error)))
    }
    pub fn parse_int_error(input: impl Into<String>, source: ParseIntError) -> Self {
        VersionError::ParseIntError {
            message: input.into(),
            source,
        }
    }
}

#[cfg(feature = "not_required_in_rust_lucene")]
#[allow(unused)]
fn get_package_implementation_version() {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use crate::test::util::lucene_test_case::random;

    use crate::util::error::lucene_error::LuceneError;
    use crate::util::{
        Version, LATEST, LUCENE_10_0_0, LUCENE_10_1_0, LUCENE_11_0_0, LUCENE_CURRENT,
    };
    use rand::Rng;
    use std::hash::{DefaultHasher, Hash, Hasher};

    #[allow(dead_code)] // for quick search
    struct TestVersion;

    #[test]
    fn test_on_or_after() -> Result<(), LuceneError> {
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
    fn test_to_string() -> Result<(), LuceneError> {
        assert_eq!(Version::from_bits(9, 0, 0)?.to_string(), "9.0.0");
        assert_eq!(LUCENE_10_0_0.to_string(), "10.0.0");
        assert_eq!(LUCENE_10_1_0.to_string(), "10.1.0");
        assert_eq!(LUCENE_11_0_0.to_string(), "11.0.0");
        Ok(())
    }

    #[test]
    fn test_parse_leniently() -> Result<(), LuceneError> {
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
    fn test_parse_leniently_on_all_constants() -> Result<(), LuceneError> {
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
    fn test_parse() -> Result<(), LuceneError> {
        assert_eq!(Version::parse("10.0.0")?, *LUCENE_10_0_0);
        assert_eq!(Version::parse("11.0.0")?, *LUCENE_11_0_0);

        assert_eq!(Version::parse("1.0")?.major, 1);
        assert_eq!(Version::parse("7.0.0")?.major, 7);
        Ok(())
    }

    #[test]
    fn test_forwards_compatibility() -> Result<(), LuceneError> {
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
    fn test_non_floating_point_compliant_version_numbers() -> Result<(), LuceneError> {
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
    fn test_equals_hash_code() -> Result<(), LuceneError> {
        let mut random = random();

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
}
