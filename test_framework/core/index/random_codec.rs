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
use crate::core::codecs::Codecs;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashSet;

/// Codec that assigns per-field random postings formats.
///
/// TODO IMPORTANT: Implement the Java `RandomCodec` format selection and per-field mappings. The
/// enum and constructors are defined now so callers will not need to change when that logic is
/// migrated.
#[derive(Clone)]
pub enum RandomCodec {
  /// Until the randomized formats are migrated, delegate to the current default codec.
  Default(Codecs),
}

impl RandomCodec {
  pub fn new<R>(random: &mut R) -> Self
  where
    R: Rng + ?Sized,
  {
    Self::with_avoid_codecs(random, &HashSet::new())
  }

  pub fn with_avoid_codecs<R>(_random: &mut R, _avoid_codecs: &HashSet<String>) -> Self
  where
    R: Rng + ?Sized,
  {
    Self::Default(TestUtil::get_default_codec().into())
  }
}

impl From<RandomCodec> for Codecs {
  fn from(codec: RandomCodec) -> Self {
    match codec {
      RandomCodec::Default(codec) => codec,
    }
  }
}
