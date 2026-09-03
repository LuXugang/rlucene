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
use crate::core::store::IO_CONTEXT_DEFAULT;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory, new_directory_shared, random,
};
use std::collections::{HashMap, HashSet};

use std::sync::Arc;

use rand::RngExt;

use crate::core::codecs::segment_info_format::SegmentInfoFormat;
use crate::core::codecs::{Codec, CodecUtil, codec};
use crate::core::index::IndexFileNames;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::search::sort::Sort;
use crate::core::store::directory::Directory;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::store::{DataInput, DataOutput, IOContext, IndexInput};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::core::util::{LATEST, LUCENE_9_0_0, LUCENE_10_1_1, StringHelper};
use crate::test_framework::core::store::base_directory_test_case::EXTRA_FILE_NAME;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
struct TestSegmentInfos;
#[test]
fn test_illegal_created_version() -> Result<()> {
  // Test for an indexCreatedVersionMajor less than 6
  let result = SegmentInfos::<DummyDirectory>::new(5);
  assert!(result.is_err());
  if let Err(err) = result {
    assert!(
      err
        .to_string()
        .contains("indexCreatedVersionMajor must be >= 6")
    );
  }

  // Test for an indexCreatedVersionMajor greater than LATEST.major
  let future_version = LATEST.major + 1;
  let result = SegmentInfos::<DummyDirectory>::new(future_version);
  assert!(result.is_err());
  let expect = format!(
    "indexCreatedVersionMajor is in the future: {}",
    future_version
  );
  if let Err(err) = result {
    assert!(err.to_string().contains(&expect));
  }
  Ok(())
}
#[test]
fn test_versions_no_segments() -> Result<()> {
  let mut random = random();
  let directory = new_directory_shared(&mut random)?;
  let mut sis = SegmentInfos::new(LATEST.major)?;
  sis.commit(directory.as_ref())?;
  sis = SegmentInfos::read_latest_commit(directory.clone())?;
  assert!(sis.get_min_segment_lucene_version().is_none());
  let result = sis.get_commit_lucene_version();
  assert!(result.is_some());
  assert_eq!(*result.unwrap(), *LATEST);
  Ok(())
}
#[test]
fn test_versions_one_segment() -> Result<()> {
  let mut random = random();
  let dir = new_directory(&mut random)?;
  let directory = Arc::new(dir);
  let codec = codec::get_default();
  let io_context = IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?;
  let mut sis = SegmentInfos::new(LATEST.major)?;
  let mut info = SegmentInfo::new(
    directory.clone(),
    Some((*LUCENE_10_1_1).clone()),
    Some((*LUCENE_10_1_1).clone()),
    "_0",
    1,
    false,
    false,
    Some(codec.clone()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;
  info.set_files(HashSet::new())?;
  codec
    .segment_info_format()
    .write(directory.as_ref(), &mut info, &io_context)?;

  let commit_info = SegmentCommitInfo::new(info, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));

  sis.add(commit_info)?;
  sis.commit(directory.as_ref())?;

  sis = SegmentInfos::read_latest_commit(directory.clone())?;
  assert_eq!(
    *sis.get_min_segment_lucene_version().unwrap(),
    (*LUCENE_10_1_1).clone()
  );
  assert_eq!(*sis.get_commit_lucene_version().unwrap(), (*LATEST).clone());

  Ok(())
}

#[test]
fn test_versions_two_segments() -> Result<()> {
  let mut random = random();
  let dir = new_directory(&mut random)?;
  let directory = Arc::new(dir);
  let codec = codec::get_default();
  let mut sis = SegmentInfos::new(LATEST.major)?;
  let io_context = IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?;
  // First Segment
  let mut info_0 = SegmentInfo::new(
    directory.clone(),
    Some((*LUCENE_10_1_1).clone()),
    Some((*LUCENE_10_1_1).clone()),
    "_0",
    1,
    false,
    false,
    Some(codec.clone()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;
  info_0.set_files(HashSet::new())?;
  codec
    .segment_info_format()
    .write(directory.as_ref(), &mut info_0, &io_context)?;

  let commit_info_0 =
    SegmentCommitInfo::new(info_0, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));
  let _id_0 = commit_info_0.info.get_id_key().to_string();
  sis.add(commit_info_0)?;

  // Second Segment
  let mut info_1 = SegmentInfo::new(
    directory.clone(),
    Some((*LUCENE_10_1_1).clone()),
    Some((*LUCENE_10_1_1).clone()),
    "_1",
    1,
    false,
    false,
    Some(codec.clone()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;
  info_1.set_files(HashSet::new())?;
  codec
    .segment_info_format()
    .write(directory.as_ref(), &mut info_1, &io_context)?;

  let commit_info_1 =
    SegmentCommitInfo::new(info_1, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));
  let _id_1 = commit_info_1.info.get_id_key().to_string();
  sis.add(commit_info_1)?;
  sis.commit(directory.as_ref())?;

  let commit_info_id_0 = *sis.info(0).unwrap().get_id().unwrap();
  let commit_info_id_1 = *sis.info(1).unwrap().get_id().unwrap();

  // Read back the latest commit
  sis = SegmentInfos::read_latest_commit(directory.clone())?;

  // Verify results
  assert_eq!(
    *sis.get_min_segment_lucene_version().unwrap(),
    (*LUCENE_10_1_1).clone()
  );
  assert_eq!(*sis.get_commit_lucene_version().unwrap(), (*LATEST).clone());
  let actual1 = sis.info(0).unwrap().get_id();
  let actual2 = sis.info(1).unwrap().get_id();
  assert_eq!(
    StringHelper::id_to_string(Option::from(&commit_info_id_0)),
    StringHelper::id_to_string(Option::from(actual1.unwrap()))
  );
  assert_eq!(
    StringHelper::id_to_string(Option::from(&commit_info_id_1)),
    StringHelper::id_to_string(Option::from(actual2.unwrap()))
  );

  Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  // Diagnostics map
  let diagnostics: HashMap<String, String> = [
    ("key1".to_string(), "value1".to_string()),
    ("key2".to_string(), "value2".to_string()),
  ]
  .iter()
  .cloned()
  .collect();

  // Attributes map
  let attributes: HashMap<String, String> = [
    ("akey1".to_string(), "value1".to_string()),
    ("akey2".to_string(), "value2".to_string()),
  ]
  .iter()
  .cloned()
  .collect();

  // diagnostics X, attributes X
  let si = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "TEST",
    10000,
    false,
    false,
    Some(codec::get_default()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;
  assert_eq!(
    format!("TEST({}){}:[indexSort=<doc>]", *LATEST, ":C10000"),
    format!("{}", si)
  );

  // diagnostics O, attributes X
  let si = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "TEST",
    10000,
    false,
    false,
    Some(codec::get_default()),
    diagnostics.clone(),
    StringHelper::random_id(),
    HashMap::new(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;
  assert_eq!(
    format!(
      "TEST({}){}:[indexSort=<doc>]:[diagnostics={:?}]",
      *LATEST, ":C10000", diagnostics
    ),
    format!("{}", si)
  );

  // diagnostics X, attributes O
  let si = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "TEST",
    10000,
    false,
    false,
    Some(codec::get_default()),
    HashMap::new(),
    StringHelper::random_id(),
    attributes.clone(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;
  assert_eq!(
    format!(
      "TEST({}){}:[indexSort=<doc>]:[attributes={:?}]",
      *LATEST, ":C10000", attributes
    ),
    format!("{}", si)
  );

  // diagnostics O, attributes O
  let si = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "TEST",
    10000,
    false,
    false,
    Some(codec::get_default()),
    diagnostics.clone(),
    StringHelper::random_id(),
    attributes.clone(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;
  assert_eq!(
    format!(
      "TEST({}){}:[indexSort=<doc>]:[diagnostics={:?}]:[attributes={:?}]",
      *LATEST, ":C10000", diagnostics, attributes
    ),
    format!("{}", si)
  );
  Ok(())
}
#[test]
fn test_id_changes_on_advance() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let id = StringHelper::random_id();

  let info = SegmentInfo::new(
    dir.clone(),
    Some((*LUCENE_9_0_0).clone()),
    Some((*LUCENE_9_0_0).clone()),
    "_0",
    1,
    false,
    false,
    Some(codec::get_default()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;

  let mut commit_info = SegmentCommitInfo::new(info, 0, 0, -1, -1, -1, Some(id));
  assert_eq!(
    StringHelper::id_to_string(Some(&id)),
    StringHelper::id_to_string(commit_info.get_id())
  );

  commit_info.advance_del_gen();
  assert_ne!(
    StringHelper::id_to_string(Some(&id)),
    StringHelper::id_to_string(commit_info.get_id())
  );

  let new_id = *commit_info.get_id().unwrap();
  commit_info.advance_doc_values_gen();
  assert_ne!(
    StringHelper::id_to_string(Some(&new_id)),
    StringHelper::id_to_string(commit_info.get_id())
  );

  let new_id = *commit_info.get_id().unwrap();
  commit_info.advance_field_infos_gen();
  assert_ne!(
    StringHelper::id_to_string(Some(&new_id)),
    StringHelper::id_to_string(commit_info.get_id())
  );

  let clone = commit_info.clone();
  let current_id = *commit_info.get_id().unwrap();
  assert_eq!(
    StringHelper::id_to_string(Some(&current_id)),
    StringHelper::id_to_string(commit_info.get_id())
  );
  assert_eq!(
    StringHelper::id_to_string(Some(&current_id)),
    StringHelper::id_to_string(clone.get_id())
  );

  commit_info.advance_field_infos_gen();
  assert_ne!(
    StringHelper::id_to_string(Some(&current_id)),
    StringHelper::id_to_string(commit_info.get_id())
  );
  assert_eq!(
    StringHelper::id_to_string(Some(&current_id)),
    StringHelper::id_to_string(clone.get_id()),
    "clone changed but shouldn't"
  );

  Ok(())
}
#[test]
fn test_bit_flipped_triggers_corrupt_index_exception() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let codec = codec::get_default();
  let mut sis = SegmentInfos::new(LATEST.major)?;
  let io_context = IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?;
  let mut info_0 = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "_0",
    1,
    false,
    false,
    Some(codec.clone()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;
  info_0.set_files(HashSet::new())?;
  codec
    .segment_info_format()
    .write(dir.as_ref(), &mut info_0, &io_context)?;
  let commit_info_0 =
    SegmentCommitInfo::new(info_0, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));
  sis.add(commit_info_0)?;

  // Add second SegmentCommitInfo
  let mut info_1 = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "_1",
    1,
    false,
    false,
    Some(codec.clone()),
    HashMap::new(),
    StringHelper::random_id(),
    HashMap::new(),
    None,
  )?;
  info_1.set_files(HashSet::new())?;
  codec
    .segment_info_format()
    .write(dir.as_ref(), &mut info_1, &io_context)?;
  let commit_info_1 =
    SegmentCommitInfo::new(info_1, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));
  sis.add(commit_info_1)?;

  sis.commit(dir.as_ref())?;

  // Create a corrupt directory
  let corrupt_dir = new_directory_shared(&mut random)?;
  let mut corrupt = false;
  let io_context = IOContext::read_once_io_context()?;
  {
    let directory = dir.as_ref();
    for file in directory.list_all()? {
      if file.starts_with(IndexFileNames::SEGMENTS) {
        {
          let mut input = directory.open_input(&file, &io_context)?;
          let mut output = corrupt_dir.create_output(&file, &io_context)?;

          let copy_result = (|| -> Result<()> {
            let mut input_length = IndexInput::length(&input)?;
            let corrupt_index = TestUtil::next_usize(&mut random, 0, input_length - 1);
            output.copy_bytes(&mut input, corrupt_index)?;

            let byte = DataInput::read_byte(&mut input)?;
            let value = random.random_range(0x01..=0xff);
            let corrupt_byte = byte.wrapping_add(value);
            output.write_byte(corrupt_byte)?;
            input_length = IndexInput::length(&input)?;
            let file_pointer = input.get_file_pointer()?;
            output.copy_bytes(&mut input, input_length - file_pointer)?;
            Ok(())
          })();
          let close_result = IOUtils::use_or_suppress_result(output.close(), input.close());
          IOUtils::use_or_suppress_result(copy_result, close_result)?;
        }
        let input = corrupt_dir.open_input(&file, &io_context)?;
        let checksum_result = CodecUtil::checksum_entire_file(&input);
        let checksum_result = IOUtils::use_or_suppress_result(checksum_result, input.close());
        match checksum_result {
          Ok(_) => {
            if cfg!(feature = "test_log_verbose") {
              println!("TEST: Altering the file did not update the checksum, aborting...");
            }
            return Ok(());
          },
          Err(LuceneError::CorruptIndex(_)) => {
            // Corruption detected
          },
          Err(err) => return Err(err),
        }
        corrupt = true;
      } else if file != EXTRA_FILE_NAME {
        corrupt_dir.copy_from(directory, &file, &file, &io_context)?;
      }
    }
  }

  assert!(corrupt, "No segments file found");

  let result = SegmentInfos::read_latest_commit(corrupt_dir.clone());
  assert!(result.is_err());
  match result {
    Err(LuceneError::CorruptIndex(_))
    | Err(LuceneError::IndexFormatTooOld(_))
    | Err(LuceneError::IndexFormatTooNew(_)) => {},
    Err(error) => panic!("unexpected error: {error:?}"),
    Ok(_) => panic!("expected an error"),
  }

  Ok(())
}
#[test]
fn test_add_diagnostics() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  // Diagnostics map
  let diagnostics: HashMap<String, String> = [
    ("key1".to_string(), "value1".to_string()),
    ("key2".to_string(), "value2".to_string()),
  ]
  .iter()
  .cloned()
  .collect();

  // Test adding a new key-value pair
  let mut si = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "TEST",
    10000,
    false,
    false,
    Some(codec::get_default()),
    diagnostics.clone(),
    StringHelper::random_id(),
    HashMap::new(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;
  si.add_diagnostics(
    [("key3".to_string(), "value3".to_string())]
      .iter()
      .cloned()
      .collect(),
  );
  let expected_diagnostics: HashMap<String, String> = [
    ("key1".to_string(), "value1".to_string()),
    ("key2".to_string(), "value2".to_string()),
    ("key3".to_string(), "value3".to_string()),
  ]
  .iter()
  .cloned()
  .collect();
  assert_eq!(si.get_diagnostics(), &expected_diagnostics);

  // Test modifying an existing key-value pair
  let mut si = SegmentInfo::new(
    dir.clone(),
    Some((*LATEST).clone()),
    Some((*LATEST).clone()),
    "TEST",
    10000,
    false,
    false,
    Some(codec::get_default()),
    diagnostics.clone(),
    StringHelper::random_id(),
    HashMap::new(),
    Some(Arc::new(Sort::get_index_order()?)),
  )?;
  si.add_diagnostics(
    [("key2".to_string(), "foo".to_string())]
      .iter()
      .cloned()
      .collect(),
  );
  let expected_diagnostics: HashMap<String, String> = [
    ("key1".to_string(), "value1".to_string()),
    ("key2".to_string(), "foo".to_string()),
  ]
  .iter()
  .cloned()
  .collect();
  assert_eq!(si.get_diagnostics(), &expected_diagnostics);
  Ok(())
}
