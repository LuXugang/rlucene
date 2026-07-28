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
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::util::bits::Bits;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::index::asserting_directory_reader::AssertingDirectoryReader;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_string_field, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestAssertingLeafReader;

#[test]
fn test_assert_bits() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut config = new_index_writer_config(&mut random)?;
  config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), config)?;

  // Not deleted:
  writer.add_document(Document::new())?;

  // Does get deleted:
  let mut doc = Document::new();
  let mut field_types = HashMap::new();
  doc.add(new_string_field(
    &mut random,
    "id",
    "0",
    Store::No,
    &mut field_types,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  writer.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  writer.close()?;

  // Now we have index with 1 segment with 2 docs one of which is marked deleted.
  let reader = directory_reader::open(dir.clone())?;
  assert_eq!(1, reader.get_sequential_sub_readers().len());
  assert_eq!(2, reader.max_doc()?);
  assert_eq!(1, reader.num_docs()?);

  let reader = AssertingDirectoryReader::new(reader)?;
  let thread_result = std::thread::scope(|scope| {
    scope
      .spawn(|| -> Result<()> {
        for reader in reader.get_sequential_sub_readers() {
          reader
            .get_live_docs()?
            .expect("the deleted document must produce live docs")
            .get(0)?;
        }
        Ok(())
      })
      .join()
  });
  match thread_result {
    Ok(result) => result?,
    Err(payload) => std::panic::resume_unwind(payload),
  }

  IOUtils::use_or_suppress_result(reader.close(), dir.close())
}
