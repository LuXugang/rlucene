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
use crate::core::codecs::{Codecs, codec};
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use rand::Rng;

/// Base test support for a norms format. NOTE: This test focuses on the norms implementation,
/// nothing else. The [stretch] goal is for this test to be so thorough in testing a new NormsFormat
/// that if this test passes, then all Lucene tests should also pass. Ie, if there is some bug in a
/// given NormsFormat that this test fails to catch then this test needs to be improved!
pub trait BaseIndexFileFormatTestCase {
  fn add_random_fields<R>(random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized;

  fn maybe_wrap_with_merging_reader<D>(&self, reader: D) -> Result<D> {
    Ok(reader)
  }

  /// Set the created version of the given [`Directory`] and return it.
  fn apply_created_version_major<D>(&self, d: D) -> Result<D>
  where
    D: Directory,
  {
    Ok(d)
  }
  fn test_merge_stability(&self) -> Result<()> {
    Ok(())
  }
  fn get_codec(&self) -> Result<Codecs> {
    Ok(codec::get_default())
  }
}
