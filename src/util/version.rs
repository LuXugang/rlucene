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
use crate::util::StrictStringTokenizer;
use lazy_static::lazy_static;
use regex::Regex;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use thiserror::Error;

lazy_static! {
pub static ref LUCENE_10_0_0:Version = Version::new(10, 0, 0).unwrap();
/**
* Match settings and bugs in Lucene's 0.2.0 release.
*
*/
pub static ref LUCENE_10_1_0:Version = Version::new(10,1,0).unwrap();
 /**
 * Match settings and bugs in Lucene's 1.0.0 release.
 *
 */
pub static ref LUCENE_11_0_0:Version = Version::new(11,0,0).unwrap();
/**
* WARNING: if you use this setting, and then upgrade to a newer release of Lucene, sizable
* changes may happen. If backwards compatibility is important then you should instead explicitly
* specify an actual version.
*
* If you use this constant then you may need to re-index all of your documents when
* upgrading Lucene, as the way text is indexed may have changed. Additionally, you may need to
* re-test your entire application to ensure it behaves as expected, as some defaults may
* have changed and may break functionality in your application.
*/
pub static ref LATEST:Version = LUCENE_11_0_0.clone();
pub static ref LUCENE_CURRENT:Version =  LATEST.clone();
pub static ref MIN_SUPPORTED_MAJOR:u32= LATEST.major - 1;
}

/**
 * Constant for the minimal supported major version of an index. This version is defined by the
 * version that initially created the index.
 */

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, Debug, Hash)]
pub struct Version {
    pub major: u32,
    minor: u32,
    bug_fix: u32,
    prerelease: u32,
    encoded_value: u32,
}
impl Version {
    fn new(major: u32, minor: u32, bug_fix: u32) -> Result<Version, IllegalArgumentError> {
        Version::new_with_prerelease(major, minor, bug_fix, 0)
    }
    fn new_with_prerelease(
        major: u32,
        minor: u32,
        bug_fix: u32,
        prerelease: u32,
    ) -> Result<Version, IllegalArgumentError> {
        // NOTE: do not enforce major version so we remain future proof, except to
        // make sure it fits in the 8 bits we encode it into:
        if major > 255 {
            return Err(IllegalArgumentError::new(format!(
                "Illegal major version: {}",
                major
            )));
        }
        if minor > 255 {
            return Err(IllegalArgumentError::new(format!(
                "Illegal minor version: {}",
                minor
            )));
        }
        if bug_fix > 255 {
            return Err(IllegalArgumentError::new(format!(
                "Illegal bug fix version: {}",
                bug_fix
            )));
        }
        if prerelease > 2 {
            return Err(IllegalArgumentError::new(format!(
                "Illegal pre-release version: {}",
                prerelease
            )));
        }
        if prerelease != 0 && (minor != 0 || bug_fix != 0) {
            return Err(IllegalArgumentError::new(format!("Prerelease version only supported with major release (got prerelease: {}, minor: {}, bug_fix: {})", prerelease, minor, bug_fix)));
        }
        let encoded_value = (major << 18) | (minor << 10) | (bug_fix << 2) | prerelease;
        debug_assert!(encoded_is_valid(
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
    pub fn on_or_after(&self, other: Version) -> bool {
        self.encoded_value >= other.encoded_value
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
/**
 * Parse a version number of the form `"major.minor.bugfix.prerelease"`.
 *
 * Part `".bugfix"` and part `".prerelease"` are optional. Note that this is
 * forwards compatible: the parsed version does not have to exist as a constant.
 */
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
    let major = token.parse::<u32>();
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
    let minor = token.parse::<u32>();
    if minor.is_err() {
        return Err(VersionError::parse_int_error(
            format!(
                "Failed to parse minor version from {} (got: {})",
                token, version
            ),
            minor.unwrap_err(),
        ));
    }
    let mut bug_fix_value: u32 = 0;
    if tokens.has_more_tokens() {
        token = tokens.next_token()?;
        let bug_fix = token.parse::<u32>();
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
    let mut prerelease_value: u32 = 0;
    if tokens.has_more_tokens() {
        token = tokens.next_token()?;
        let prerelease = token.parse::<u32>();
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
    let result = Version::new_with_prerelease(
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
            match parse(&version) {
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
/**
 * Returns a new version based on raw numbers
 */
pub fn from_bits(major: u32, minor: u32, bug_fix: u32) -> Result<Version, IllegalArgumentError> {
    Version::new(major, minor, bug_fix)
}

fn encoded_is_valid(
    major: u32,
    minor: u32,
    bug_fix: u32,
    prerelease: u32,
    encoded_value: u32,
) -> bool {
    debug_assert_eq!(major, (encoded_value >> 18) & 0xFF);
    debug_assert_eq!(minor, (encoded_value >> 10) & 0xFF);
    debug_assert_eq!(bug_fix, (encoded_value >> 2) & 0xFF);
    debug_assert_eq!(prerelease, encoded_value & 0x03);
    true
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
    pub fn parse_error_with_pos(msg: impl Into<String>, position: u32) -> Self {
        VersionError::Parse(Parse::new(msg, position))
    }
    pub fn parse_error_with_error(msg: impl Into<String>, error: IllegalArgumentError) -> Self {
        VersionError::Parse(Parse::new_with_error(msg, Option::from(error)))
    }
    pub fn parse_int_error(input: impl Into<String>, source: ParseIntError) -> Self {
        VersionError::ParseIntError {
            message: input.into(),
            source,
        }
    }
}

#[cfg(feature = "not_required_in_rlucene")]
fn get_package_implementation_version() {
    unimplemented!()
}
