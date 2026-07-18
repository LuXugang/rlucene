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
use crate::core::document::document::Document;
use crate::core::index::merge_policy::OneMergeProgress;
use crate::core::index::merge_rate_limiter::MergeRateLimiter;
use crate::core::store::rate_limiter::RateLimiter;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestMergeRateLimiter;

#[test]
fn test_init_defaults() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;
  w.add_document(&mut random, Document::new())?;
  w.close(&mut random)?;

  let rate_limiter = MergeRateLimiter::new(Arc::new(OneMergeProgress::new()));
  assert!(rate_limiter.get_mb_per_sec().is_infinite());
  assert!(rate_limiter.get_min_pause_check_bytes() > 0);
  dir.close()?;
  Ok(())
}
