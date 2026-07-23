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
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::sorter::DocMap;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::mock_random_merge_policy::reverse;
use crate::test_framework::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, random,
};

#[allow(dead_code)] // for quick search
struct TestMockRandomMergePolicy;

#[test]
fn test_reverse_with_parents() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut config = IndexWriterConfig::new()?;
  config.set_parent_field("_parent");
  let w = IndexWriter::new(dir.clone(), config)?;
  let docs = vec![Document::new(); 5];
  w.add_documents(docs[0..2].to_vec())?;
  w.add_documents(docs[0..4].to_vec())?;
  w.add_documents(docs[0..3].to_vec())?;
  w.force_merge(1)?;
  w.close()?;
  let reader = directory_reader::open(dir.clone())?;
  let codec_reader = get_only_leaf_reader(&reader)?;
  let doc_map = reverse(&codec_reader)?;

  assert_eq!(7, doc_map.old_to_new(0)?);
  assert_eq!(8, doc_map.old_to_new(1)?);
  assert_eq!(3, doc_map.old_to_new(2)?);
  assert_eq!(4, doc_map.old_to_new(3)?);
  assert_eq!(5, doc_map.old_to_new(4)?);
  assert_eq!(6, doc_map.old_to_new(5)?);
  assert_eq!(0, doc_map.old_to_new(6)?);
  assert_eq!(1, doc_map.old_to_new(7)?);
  assert_eq!(2, doc_map.old_to_new(8)?);

  assert_eq!(6, doc_map.new_to_old(0)?);
  assert_eq!(7, doc_map.new_to_old(1)?);
  assert_eq!(8, doc_map.new_to_old(2)?);
  assert_eq!(2, doc_map.new_to_old(3)?);
  assert_eq!(3, doc_map.new_to_old(4)?);
  assert_eq!(4, doc_map.new_to_old(5)?);
  assert_eq!(5, doc_map.new_to_old(6)?);
  assert_eq!(0, doc_map.new_to_old(7)?);
  assert_eq!(1, doc_map.new_to_old(8)?);

  reader.close()?;
  dir.close()?;
  Ok(())
}
