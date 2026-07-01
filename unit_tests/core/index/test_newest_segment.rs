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
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};

#[allow(dead_code)] // for quick search
pub struct TestNewestSegment;

#[test]
fn test_newest_segment() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    directory.clone(),
    new_index_writer_config_with_analyzer(&mut random, mock)?,
  )?;
  writer.new_segment_name(None);
  writer.close()?;
  Ok(())
}
