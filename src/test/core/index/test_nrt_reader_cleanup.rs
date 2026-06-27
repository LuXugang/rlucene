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
use crate::core::document::field::Store;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_index_writer_config_with_analyzer, new_mock_directory, new_text_field, random,
};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestNRTReaderCleanup;

#[test]
fn test_closing_nrt_reader_does_not_corrupt_your_index() -> Result<()> {
  if cfg!(windows) {
    return Ok(());
  }

  let mut random = random();
  let dir = new_mock_directory(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  let mut lmp = LogMergePolicy::log_doc();
  lmp.set_merge_factor(2)?;
  iwc.set_merge_policy(lmp);

  let w = RandomIndexWriter::with_config(&mut random, Arc::new(dir.clone()), iwc);
  let mut field_types = HashMap::new();

  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "a",
    "foo",
    Store::No,
    &mut field_types,
  )?);

  w.add_document(&mut random, doc.clone())?;
  w.commit(&mut random)?;
  w.add_document(&mut random, doc.clone())?;

  let r = w.get_reader(&mut random)?;
  w.close(&mut random)?;
  drop(w);

  for name in dir.list_all()? {
    dir.delete_file(&name)?;
  }
  let w = RandomIndexWriter::new(&mut random, Arc::new(dir.clone()))?;
  w.add_document(&mut random, doc)?;
  w.close(&mut random)?;
  r.close()?;
  Ok(())
}
