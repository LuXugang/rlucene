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
pub fn is_night_mode() -> bool {
    std::env::var("NIGHT_MODE").is_ok_and(|v| v == "true")
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
        assert_eq!(expected.len(), actual.len(),);
    }

    for (exp, act) in expected.iter().zip(actual.iter()) {
        if exp != act {
            assert_eq!(exp, act,);
        }
    }
}
