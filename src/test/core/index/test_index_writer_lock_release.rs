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
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_fs_directory, new_index_writer_config_with_analyzer, random,
};

#[allow(dead_code)] // for quick search
struct TestIndexWriterLockRelease;

#[test]
fn test_index_writer_lock_release() -> Result<()> {
  let mut random = random();
  let tmp = create_temp_dir_with_prefix("testLockRelease")?;
  let dir = new_fs_directory(&mut random, tmp)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_open_mode(OpenMode::Append);

  if IndexWriter::new(dir.clone(), iwc).is_err() {
    let mock = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
    iwc.set_open_mode(OpenMode::Append);
    let _ = IndexWriter::new(dir.clone(), iwc);
  }
  Ok(())
}
