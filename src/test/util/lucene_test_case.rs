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
use crate::store::directory::Directory;
use crate::store::flush_info::FlushInfo;
use crate::store::merge_info::MergeInfo;
use crate::store::nio_fs_directory::NIOFSDirectory;
use crate::store::{
    FSDirectory, IOContext, NativeFSLockFactory, IO_CONTEXT_DEFAULT, IO_CONTEXT_READ_ONCE,
};
use crate::test::util::lucene_test_case::EnvConfig::{Multiplier, NightMode, TestSeed};

use crate::test::util::test_util::TestUtil;
use crate::util::error::lucene_error::LuceneError;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fmt;
use tempfile::TempDir;

#[allow(dead_code)] // for quick search
pub struct LuceneTestCase;
/// Describes the currently supported environment variables used to control Lucene tests.
///
/// Each variant corresponds to an environment variable that configures specific behaviors of the tests.
/// For example, environment variables can be used to control the test mode, random number generator seed, etc.
#[derive(Debug, Clone, Copy)]
pub enum EnvConfig {
    NightMode,
    Multiplier,
    TestSeed,
}

impl fmt::Display for EnvConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self {
            NightMode => "tests.nightly",
            Multiplier => "tests.multiplier",
            TestSeed => "tests.seed",
        };
        write!(f, "{}", key)
    }
}

pub(crate) fn random_multiplier() -> i32 {
    let multiplier = std::env::var(Multiplier.to_string()).ok();

    multiplier
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(default_random_multiplier())
}

fn default_random_multiplier() -> i32 {
    if is_night_mode() {
        2
    } else {
        1
    }
}
/// Returns a number of at least `i`
///
/// The actual number returned will be influenced by whether `TEST_NIGHTLY` is active and
/// `RANDOM_MULTIPLIER`, but also with some random fudge.
pub(crate) fn at_least(random: &mut StdRng, i: i32) -> i32 {
    let min = i * random_multiplier();
    let max = min + (min / 2);
    TestUtil::next_int(random, min, max)
}

pub(crate) fn rarely(random: &mut StdRng) -> bool {
    let mut p = if is_night_mode() { 5 } else { 1 };
    p += (p as f64 * (random_multiplier() as f64).ln()).round() as i32;
    let min = 100 - p.min(20); // Never more than 20% chance
    random.gen_range(0..100) >= min
}

// TODO: When we have implemented multiple directories, we need to select one randomly. Currently, we choose NIOFSDirectory.
pub(crate) fn new_directory(
    _random: &mut StdRng,
) -> Result<FSDirectory<NativeFSLockFactory, NIOFSDirectory>, LuceneError> {
    let temp_dir = TempDir::new()?;
    let sub_directory = NIOFSDirectory::new();
    FSDirectory::new(temp_dir.into_path(), sub_directory)
}

pub(crate) fn new_io_context(random: &mut StdRng) -> Result<IOContext, LuceneError> {
    new_io_context_with_default(random, &IO_CONTEXT_DEFAULT)
}

pub(crate) fn new_io_context_with_default(
    random: &mut StdRng,
    old_context: &IOContext,
) -> Result<IOContext, LuceneError> {
    if *old_context == *IO_CONTEXT_READ_ONCE {
        // Don't modify the READONCE singleton
        return Ok(old_context.clone());
    }

    // Generate random parameters
    let random_num_docs: i32 = random.gen_range(0..4192);
    let size = random.gen_range(0..512) * random_num_docs as i64;

    if let Some(flush_info) = &old_context.flush_info {
        // Always return at least the estimatedSegmentSize of the incoming IOContext
        Ok(IOContext::with_flush(FlushInfo::new(
            random_num_docs,
            size.max(flush_info.get_estimated_segment_size()),
        ))?)
    } else if let Some(merge_info) = &old_context.merge_info {
        // Always return at least the estimatedMergeBytes of the incoming IOContext
        return IOContext::with_merge(MergeInfo::new(
            random_num_docs,
            size.max(merge_info.get_estimated_merge_bytes()),
            random.gen_bool(0.5), // Randomly decide if it's an external merge
            random.gen_range(1..=100),
        ));
    } else {
        // Make a totally random IOContext, except READONCE which has semantic implications
        let context_type = random.gen_range(0..3);
        match context_type {
            0 => Ok(IOContext::default_io_context()?),
            1 => Ok(IOContext::with_merge(MergeInfo::new(
                random_num_docs,
                size,
                true,
                -1,
            ))?),
            2 => Ok(IOContext::with_flush(FlushInfo::new(
                random_num_docs,
                size,
            ))?),
            _ => Ok(IOContext::default_io_context()?),
        }
    }
}
pub(crate) fn slow_file_exists(dir: &impl Directory, name: &str) -> Result<bool, LuceneError> {
    let result = dir.open_input(name, &IOContext::default_io_context()?);
    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
/// Creates a `BytesRef` holding UTF-8 bytes for the incoming string,
/// that sometimes uses a non-zero offset and non-zero end-padding to
/// tickle latent bugs that fail to look at `BytesRef.offset`.
pub(crate) fn new_bytes_ref_from_string(
    random: &mut StdRng,
    s: &str,
) -> Result<BytesRef, LuceneError> {
    let bytes = s.as_bytes();
    new_bytes_ref(random, bytes, 0, bytes.len() as i32)
}

/// Creates a copy of the incoming `BytesRef` that sometimes uses a non-zero offset,
/// and non-zero end-padding, to tickle latent bugs that fail to look at `BytesRef.offset`.
#[allow(unused)]
pub(crate) fn new_bytes_ref_from_bytes_ref(
    random: &mut StdRng,
    b: &BytesRef,
) -> Result<BytesRef, LuceneError> {
    assert!(b.is_valid()?);
    new_bytes_ref(random, &b.bytes, b.offset, b.length)
}

/// Creates a random `BytesRef` from the incoming bytes, sometimes using a non-zero offset,
/// and non-zero end-padding, to tickle latent bugs that fail to look at `BytesRef.offset`.
#[allow(unused)]
pub(crate) fn new_bytes_ref_from_bytes(
    random: &mut StdRng,
    bytes_in: &[u8],
) -> Result<BytesRef, LuceneError> {
    new_bytes_ref(random, bytes_in, 0, bytes_in.len() as i32)
}

/// Creates a random empty `BytesRef` that sometimes uses a non-zero offset, and non-zero
/// end-padding, to tickle latent bugs that fail to look at `BytesRef.offset`.
#[allow(unused)]
pub(crate) fn new_bytes_ref_empty(random: &mut StdRng) -> Result<BytesRef, LuceneError> {
    new_bytes_ref(random, &[], 0, 0) // Calling the existing `new_bytes_ref` function
}

/// Creates a random empty `BytesRef`, with at least the requested length of bytes free,
/// that sometimes uses a non-zero offset and non-zero end-padding to tickle latent bugs
/// that fail to look at `BytesRef.offset`.
#[allow(unused)]
pub(crate) fn new_bytes_ref_with_length(
    byte_length: i32,
    random: &mut StdRng,
) -> Result<BytesRef, LuceneError> {
    let bytes_in = vec![0u8; byte_length as usize];
    new_bytes_ref(random, &bytes_in, 0, byte_length)
}

/// Creates a copy of the incoming bytes slice that sometimes uses a non-zero {@code offset}, and
/// non-zero end-padding, to tickle latent bugs that fail to look at {@code BytesRef.offset}.
pub(crate) fn new_bytes_ref(
    random: &mut StdRng,
    bytes_in: &[u8],
    offset: i32,
    length: i32,
) -> Result<BytesRef, LuceneError> {
    assert!(
        bytes_in.len() >= (offset + length) as usize,
        "got offset={} length={} bytesIn.length={}",
        offset,
        length,
        bytes_in.len()
    );
    // Randomly set a non-zero offset
    let start_offset = if random.gen_bool(0.5) {
        random.gen_range(1..=20)
    } else {
        0
    };

    // Randomly set an end padding (between 1 and 20)
    let end_padding = if random.gen_bool(0.5) {
        random.gen_range(1..=20)
    } else {
        0
    };

    let mut bytes = vec![0u8; (start_offset + length + end_padding) as usize];

    bytes[start_offset as usize..(start_offset + length) as usize]
        .copy_from_slice(&bytes_in[offset as usize..(offset + length) as usize]);
    // Create a BytesRef and return it
    let it = BytesRef {
        bytes,
        offset: start_offset,
        length,
    };
    assert!(it.is_valid()?);

    if random.gen_range(1..=17) == 7 {
        return new_bytes_ref(random, &it.bytes, it.offset, it.length);
    };
    Ok(it)
}

/// Retrieves the seed from the environment variable "tests.seed".
/// If the environment variable is not set or cannot be parsed as a `u64`,
/// it generates a random seed and logs the result.
///
/// # Returns
/// A valid `u64` seed.
pub(crate) fn get_seed_from_env() -> u64 {
    if let Ok(seed_str) = std::env::var(TestSeed.to_string()) {
        if let Ok(seed) = seed_str.parse::<u64>() {
            println!("Using Global Seed from environment: '{}'", seed);
            return seed;
        } else {
            println!("Environment variable tests.seed is invalid: '{}'", seed_str);
        }
    }

    let seed = rand::thread_rng().gen_range(0..u64::MAX);
    println!("Generated random seed : {}", seed);
    seed
}

pub(crate) fn random() -> StdRng {
    StdRng::seed_from_u64(get_seed_from_env())
}

pub(crate) fn random_from_seed(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

pub fn is_night_mode() -> bool {
    std::env::var(NightMode.to_string()).is_ok_and(|v| v == "true")
}
