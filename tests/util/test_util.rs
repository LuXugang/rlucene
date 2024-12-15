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
use rand::rngs::StdRng;
use rand::Rng;

pub struct TestUtil;

impl TestUtil {
    pub fn random_simple_string_with_length(
        random: &mut StdRng,
        min_length: usize,
        max_length: usize,
    ) -> String {
        let end = random.gen_range(min_length..=max_length);
        if end == 0 {
            // Allow 0 length
            return String::new();
        }
        (0..end)
            .map(|_| random.gen_range(b'a'..=b'z') as char)
            .collect()
    }

    pub fn random_simple_string(random: &mut StdRng) -> String {
        Self::random_simple_string_with_length(random, 0, 10)
    }
}
