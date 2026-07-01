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
use crate::core::index::BytesRef;
use crate::test::support::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};

pub fn random_vector_dim<R>(random: &mut R, dim: usize) -> Vec<f32>
where
  R: Rng + ?Sized,
{
  (0..dim).map(|_| random.random::<f32>()).collect()
}

pub fn random_vector_bytes_dim<R>(random: &mut R, dim: usize) -> Vec<u8>
where
  R: Rng + ?Sized,
{
  let mut v: BytesRef<Vec<u8>> = TestUtil::random_binary_term_with_len(random, dim);
  for i in v.offset..(v.offset + v.length) {
    if v.bytes[i] == i8::MIN as u8 {
      v.bytes[i] = (-127i8) as u8;
    }
  }
  v.bytes
}
