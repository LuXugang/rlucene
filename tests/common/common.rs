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
use rand::{random, SeedableRng};

pub fn get_seed_from_env() -> u64 {
    std::env::var("TEST_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(random) // 默认种子
}

pub fn my_random(test_name: String) -> StdRng {
    let seed: u64 = get_seed_from_env();
    println!("Generated Seed in {}: {}", test_name, seed);
    StdRng::seed_from_u64(seed)
}

pub fn is_night_mode() -> bool {
    std::env::var("NIGHT_MODE").map_or(false, |v| v == "true")
}
