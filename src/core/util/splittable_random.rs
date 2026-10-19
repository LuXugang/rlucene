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

/// The seeded `nextDouble` path of Java's `java.util.SplittableRandom`.
///
/// Only the operations used by HNSW level generation are implemented. Seeds
/// preserve the bit pattern of Java's signed `long` values.
pub(crate) struct SplittableRandom {
  seed: u64,
}

impl SplittableRandom {
  pub(crate) fn new(seed: u64) -> Self {
    Self { seed }
  }

  pub(crate) fn next_double(&mut self) -> f64 {
    self.seed = self.seed.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = self.seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^= z >> 31;
    (z >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
  }
}
