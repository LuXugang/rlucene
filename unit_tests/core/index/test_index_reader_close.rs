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
use crate::core::index::index_reader::{CacheHelper, CacheKey, IndexReader};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, random,
};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestIndexReaderClose;

#[test]
fn test_close_under_exception() -> Result<()> {
  // TODO IMPORTANT: FilterLeafReader currently has no reusable default delegation implementation,
  // so the Java anonymous wrapper's super.doClose() and exceptional close semantics cannot yet be
  // migrated faithfully.
  Ok(())
}

#[test]
fn test_core_listener_on_wrapper_with_different_cache_key() -> Result<()> {
  // TODO IMPORTANT: AssertingLeafReader has not been implemented, and parent-reader close
  // propagation is still missing, so the wrapper/cache-key close semantics cannot yet be tested
  // faithfully.
  Ok(())
}

#[test]
fn test_register_listener_on_closed_reader() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random)?)?;
  w.add_document(Document::new())?;
  let r = Arc::new(directory_reader::open_from_writer(&w)?);
  w.close()?;

  let context = r.clone().get_context()?;
  let leaf = context.leaves()?[0].reader().clone();

  // The reader is open, everything should work
  r.get_reader_cache_helper()?
    .unwrap()
    .add_closed_listener(Box::new(|_: &CacheKey| Ok(())))?;
  leaf
    .get_reader_cache_helper()?
    .unwrap()
    .add_closed_listener(Box::new(|_: &CacheKey| Ok(())))?;
  leaf
    .get_core_cache_helper()?
    .unwrap()
    .add_closed_listener(Box::new(|_: &CacheKey| Ok(())))?;

  // But now we close
  r.close()?;
  assert!(matches!(
    r.get_reader_cache_helper()?
      .unwrap()
      .add_closed_listener(Box::new(|_: &CacheKey| Ok(()))),
    Err(LuceneError::AlreadyClosed(_))
  ));
  assert!(matches!(
    leaf
      .get_reader_cache_helper()?
      .unwrap()
      .add_closed_listener(Box::new(|_: &CacheKey| Ok(()))),
    Err(LuceneError::AlreadyClosed(_))
  ));
  assert!(matches!(
    leaf
      .get_core_cache_helper()?
      .unwrap()
      .add_closed_listener(Box::new(|_: &CacheKey| Ok(()))),
    Err(LuceneError::AlreadyClosed(_))
  ));

  dir.close()?;
  Ok(())
}
