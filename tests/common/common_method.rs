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
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};

/// Retrieves the seed from the environment variable "TEST_SEED".
/// If the environment variable is not set or cannot be parsed as a `u64`,
/// it generates a random seed and logs the result.
///
/// # Returns
/// A valid `u64` seed.
pub fn get_seed_from_env(test_name: String) -> u64 {
    if let Ok(seed_str) = std::env::var("TEST_SEED") {
        if let Ok(seed) = seed_str.parse::<u64>() {
            println!("Using Global Seed from environment: {}", seed);
            return seed;
        } else {
            println!("Environment variable TEST_SEED is invalid: {}", seed_str);
        }
    }

    let seed = rand::thread_rng().gen_range(0..u64::MAX);
    println!("Generated random seed in {}: {}", test_name,seed);
    seed
}

pub fn my_random(test_name: String) -> StdRng {
    let seed: u64 = get_seed_from_env(test_name);
    StdRng::seed_from_u64(seed)
}

pub fn my_random_with_seed(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

pub fn is_night_mode() -> bool {
    std::env::var("NIGHT_MODE").map_or(false, |v| v == "true")
}

pub fn rarely(random_value: i32) -> bool {
    let mut p = if is_night_mode() { 5 } else { 1 }; // Probability factor for nightly testing
    p += (p as f64 * (get_random_multiplier() as f64).ln()).round() as i32; // Adjust by random multiplier
    let min = 100 - p.min(20); // Never more than 20% chance
    random_value >= min
}

pub fn get_random_multiplier() -> i32 {
    let multiplier = std::env::var("TESTS_MULTIPLIER").ok();

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
pub fn assert_vecs_equal<T: PartialEq + std::fmt::Debug>(expected: &[T], actual: &[T]) {
    if expected.len() != actual.len() {
        panic!(
            "Expected and actual arrays have different lengths: expected {}, actual {}",
            expected.len(),
            actual.len()
        );
    }

    for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
        if exp != act {
            panic!(
                "Mismatch at index {}: Expected {:?}, Actual {:?}",
                i, exp, act
            );
        }
    }
}
