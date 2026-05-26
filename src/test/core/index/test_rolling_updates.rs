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
use crate::core::document::field_type::FieldType;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_string_field, random,
  random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Mutex;

#[allow(dead_code)] // for quick search
struct TestRollingUpdates;

#[test]
fn test_rolling_updates() -> Result<()> {
  // TODO IMPORTANT tryDeleteDocument未实现
  Ok(())
}

// TODO IMPORTANT 多线程索引 BUG
fn test_update_same_doc() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let field_to_type: Mutex<HashMap<String, FieldType>> = Mutex::new(HashMap::new());

  for _ in 0..3 {
    let analyzer = MockAnalyzer::new(&mut random);
    let mut config = new_index_writer_config_with_analyzer(&mut random, analyzer);
    config.set_max_buffered_docs(2);
    let writer = IndexWriter::new(dir.clone(), config)?;
    let num_updates = at_least(&mut random, 20);
    let num_threads = TestUtil::next_int(&mut random, 2, 6);
    let seed = random.random::<u64>();

    std::thread::scope(|scope| {
      let mut threads = Vec::new();
      for _ in 0..num_threads {
        threads.push(scope.spawn(|| indexing_thread(seed, &writer, num_updates, &field_to_type)));
      }

      for thread in threads {
        thread.join().expect("indexing thread panicked")?;
      }
      Ok::<(), crate::core::util::error::lucene_error::LuceneError>(())
    })?;

    writer.close()?;
  }

  let open = directory_reader::open(dir)?;
  assert_eq!(1, open.num_docs()?);
  open.close()?;
  Ok(())
}

fn indexing_thread<D>(
  seed: u64,
  writer: &IndexWriter<D>,
  num: i32,
  field_to_type: &Mutex<HashMap<String, FieldType>>,
) -> Result<()>
where
  D: Directory,
{
  let mut random = random_from_seed(seed);
  let mut open: Option<StandardDirectoryReaderType<D>> = None;

  for i in 0..num {
    let mut doc = Document::new();
    let id_field = {
      let mut field_to_type = field_to_type.lock().unwrap();
      new_string_field(&mut random, "id", "test", Store::No, &mut field_to_type)?
    };
    doc.add(id_field);
    writer.update_document_with_term(Term::from_text("id", "test"), doc)?;

    if TestUtil::next_int(&mut random, 0, 2) == 0 {
      if open.is_none() {
        // TODO IMPORTANT: openIfChanged 未实现
        open = Some(directory_reader::open_from_writer(writer)?);
      }

      let open_ref = open.as_ref().unwrap();
      assert_eq!(
        1,
        open_ref.num_docs()?,
        "iter: {} numDocs: {} del: {} max: {}",
        i,
        open_ref.num_docs()?,
        open_ref.num_deleted_docs()?,
        open_ref.max_doc()?
      );
    }
  }

  if let Some(open) = open {
    open.close()?;
  }
  Ok(())
}
