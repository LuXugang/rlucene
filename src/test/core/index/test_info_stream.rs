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
use crate::core::index::index_writer::{IndexWriter, IndexWriterHooks, IndexWriterHooksEnum};
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStream, InfoStreamEnum};
use crate::test::core::util::lucene_test_case::{new_directory_shared, random};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/** Tests indexwriter's infostream */
#[allow(dead_code)] // for quick
struct TestInfoStream;

/** we shouldn't have test points unless we ask */
#[test]
fn test_test_points_off() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = IndexWriterConfig::new();
  iwc.set_info_stream(InfoStreamEnum::Custom(Box::new(NoTestPointsInfoStream)));
  let iw = IndexWriter::new(dir, iwc)?;
  iw.add_document(Document::new())?;
  iw.close()?;
  Ok(())
}

/** but they should work when we need */
#[test]
fn test_test_points_on() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut iwc = IndexWriterConfig::new();
  let seen_test_point = Arc::new(AtomicBool::new(false));
  iwc.set_info_stream(InfoStreamEnum::Custom(Box::new(TestPointsInfoStream {
    seen_test_point: seen_test_point.clone(),
  })));
  let iw = IndexWriter::with_hooks(
    dir,
    iwc,
    Some(IndexWriterHooksEnum::custom(TestPointsIndexWriter)),
  )?;
  iw.add_document(Document::new())?;
  iw.close()?;
  assert!(seen_test_point.load(Ordering::SeqCst));
  Ok(())
}

struct NoTestPointsInfoStream;

impl InfoStream for NoTestPointsInfoStream {
  fn close(&self) -> Result<()> {
    Ok(())
  }

  fn message(&self, component: &str, _message: &str) -> Result<()> {
    assert_ne!("TP", component);
    Ok(())
  }

  fn is_enabled(&self, component: &str) -> bool {
    assert_ne!("TP", component);
    true
  }
}

struct TestPointsInfoStream {
  seen_test_point: Arc<AtomicBool>,
}

impl InfoStream for TestPointsInfoStream {
  fn close(&self) -> Result<()> {
    Ok(())
  }

  fn message(&self, component: &str, _message: &str) -> Result<()> {
    if component == "TP" {
      self.seen_test_point.store(true, Ordering::SeqCst);
    }
    Ok(())
  }

  fn is_enabled(&self, _component: &str) -> bool {
    true
  }
}

struct TestPointsIndexWriter;

impl IndexWriterHooks for TestPointsIndexWriter {
  fn is_enable_test_points(&self) -> bool {
    true
  }
}
