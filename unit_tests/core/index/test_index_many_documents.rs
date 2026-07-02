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
use crate::core::document::field::Store::No;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, create_temp_dir, new_fs_directory, new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestIndexManyDocuments;
#[test]
fn test_threaded_indexing() -> Result<()> {
  let mut random = random();

  let dir = new_fs_directory(&mut random, create_temp_dir()?)?;

  let mut iwc = IndexWriterConfig::new()?;
  let max_buffered_docs = TestUtil::next_int(&mut random, 100, 2000);
  iwc.set_max_buffered_docs(max_buffered_docs);

  let num_docs = at_least(&mut random, 10000);

  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let counter = AtomicI32::new(0);
  let random = Mutex::new(random);
  let shared_field_types = Mutex::new(HashMap::new());
  let threads = 5;
  thread::scope(|scope| {
    for _ in 0..threads {
      let writer = &writer;
      let counter = &counter;
      let random = &random;
      let field_types = &shared_field_types;

      scope.spawn(move || {
        loop {
          let curr = counter.fetch_add(1, Ordering::SeqCst);
          if curr >= num_docs {
            break;
          }

          let mut doc = Document::new();
          doc.add(
            new_text_field(
              &mut random.lock(),
              "field",
              "text",
              No,
              &mut field_types.lock(),
            )
            .unwrap(),
          );

          if let Err(e) = writer.add_document(doc) {
            unreachable!("thread indexing failed: {:?}", e);
          }
        }
      });
    }
  });

  let stats = writer.get_doc_stats()?;
  assert_eq!(num_docs, stats.max_doc,);

  writer.close()?;

  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(num_docs, reader.max_doc()?);

  Ok(())
}
