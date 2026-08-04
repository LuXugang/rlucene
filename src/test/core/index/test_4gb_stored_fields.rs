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
use crate::core::document::stored_field::StoredField;
use crate::core::index::BytesRef;
use crate::core::index::concurrent_merge_scheduler::ConcurrentMergeScheduler;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicyEnum;
use crate::core::index::stored_fields::StoredFields;
use crate::core::store::directory::Directory;
use crate::core::store::mmap_directory::MMapDirectory;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::codecs::compressing::compressing_codec::CompressingCodec;
use crate::test_framework::core::store::mock_directory_wrapper::{
  MockDirectoryWrapper, Throttling,
};
use crate::test_framework::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_log_merge_policy_with_merge_factor_cfs, random,
};
use rand::{Rng, RngExt};
use std::sync::Arc;

/// This test creates an index with one segment that is a little larger than 4GB.
#[allow(dead_code)] // for quick search
struct Test4GBStoredFields;

#[cfg(feature = "monster")]
#[test]
#[ignore = "monster"]
fn test() -> Result<()> {
  let mut random = random();
  let mmap = MMapDirectory::new(create_temp_dir_with_prefix("4GBStoredFields")?.keep())?;
  let dir = Arc::new(MockDirectoryWrapper::new(&mut random, mmap));
  dir.set_throttling(Throttling::Never);

  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = IndexWriterConfig::with_analyzer(analyzer)?;
  let mut merge_policy = new_log_merge_policy_with_merge_factor_cfs(&mut random, false, 10)?;
  if let MergePolicyEnum::LogBytesSize(policy) = &mut merge_policy {
    // 1 petabyte:
    policy.set_max_merge_mb(1024.0 * 1024.0 * 1024.0);
  }
  iwc
    .set_max_buffered_docs(-1)
    .set_ram_buffer_size_mb(256.0)
    .set_merge_scheduler(ConcurrentMergeScheduler::new())
    .set_merge_policy(merge_policy)
    .set_open_mode(OpenMode::Create);

  if random.random_bool(0.5) {
    iwc.set_codec(CompressingCodec::reasonable_instance(&mut random)?);
  }
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let value_length = random.random_range((1 << 13)..=(1 << 20));
  let mut value = vec![0u8; value_length];
  // Random so that even compressing codecs can't compress it.
  random.fill_bytes(&mut value);

  let mut doc = Document::new();
  doc.add(StoredField::from_binary("fld", value.clone())?);

  let num_docs = (((1u64 << 32) / value_length as u64) + 100) as i32;
  for i in 0..num_docs {
    // Java passes the reusable document by reference. Rust's IndexWriter consumes the document,
    // so retain the reusable instance and pass a clone containing the same stored value.
    writer.add_document(doc.clone())?;
    if cfg!(feature = "test_log_verbose") && i % (num_docs / 10) == 0 {
      println!("{i} of {num_docs}...");
    }
  }
  writer.force_merge(1)?;
  writer.close()?;

  if cfg!(feature = "test_log_verbose") {
    let mut found = false;
    for file in dir.list_all()? {
      if file.ends_with(".fdt") {
        let file_length = dir.file_length(&file)?;
        if file_length as u64 >= 1u64 << 32 {
          found = true;
        }
        println!("File length of {file} : {file_length}");
      }
    }
    if !found {
      println!("No .fdt file larger than 4GB, test bug?");
    }
  }

  let reader = directory_reader::open(dir.clone())?;
  let mut stored_fields = reader.stored_fields()?;
  let stored_doc = stored_fields.document(num_docs - 1)?;
  assert_eq!(1, stored_doc.get_fields().len());
  let value_ref = stored_doc
    .get_binary_value("fld")?
    .expect("fld must have a stored binary value");
  assert_eq!(&BytesRef::from_bytes(value), value_ref.as_ref());
  reader.close()?;

  dir.as_ref().close()?;
  Ok(())
}
