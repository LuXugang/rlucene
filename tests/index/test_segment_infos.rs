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
use crate::common::my_random;
use crate::util::lucene_test_case::new_directory;
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::codecs::segment_info_format::SegmentInfoFormat;
use rlucene::codecs::{get_default_code, Codec, CodecUtil};
use rlucene::index::segment_commit_info::SegmentCommitInfo;
use rlucene::index::segment_info::SegmentInfo;
use rlucene::index::segment_infos::SegmentInfos;
use rlucene::index::sort::Sort;
use rlucene::index::IndexFileNames;
use rlucene::search::field_comparator_source::DummyFieldComparatorSource;
use rlucene::store::directory::Directory;
use rlucene::store::nio_fs_directory::NIOFSDirectory;
use rlucene::store::nio_fs_index_input::NIOFSIndexInput;
use rlucene::store::{DataInput, DataOutput};
use rlucene::store::{FSDirectory, IOContext, IndexInput, NativeFSLockFactory};
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::{StringHelper, LATEST, LUCENE_10_0_0, LUCENE_11_0_0};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // for quick search
pub struct TestSegmentInfos;
#[test]
fn test_illegal_created_version() -> Result<(), TestError> {
    // Test for an indexCreatedVersionMajor less than 6
    let result = SegmentInfos::<
        FSDirectory<NativeFSLockFactory, NIOFSDirectory, NIOFSIndexInput>,
        DummyFieldComparatorSource,
    >::new(5);
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err
            .to_string()
            .contains("indexCreatedVersionMajor must be >= 6"));
    }

    // Test for an indexCreatedVersionMajor greater than LATEST.major
    let future_version = LATEST.major + 1;
    let result = SegmentInfos::<
        FSDirectory<NativeFSLockFactory, NIOFSDirectory, NIOFSIndexInput>,
        DummyFieldComparatorSource,
    >::new(future_version);
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
fn test_versions_no_segments() -> Result<(), TestError> {
    let mut random = my_random("test_versions_no_segments".to_string());
    let directory = Arc::new(Mutex::new(new_directory(&mut random)?));
    let mut sis = SegmentInfos::<_, DummyFieldComparatorSource>::new(LATEST.major)?;
    sis.commit(directory.clone())?;
    let result = SegmentInfos::read_latest_commit(directory.clone())?.into_segment_infos();
    assert!(result.is_some());
    sis = result.unwrap();
    assert!(sis.get_min_segment_lucene_version().is_none());
    let result = sis.get_commit_lucene_version();
    assert!(result.is_some());
    assert_eq!(*result.unwrap(), *LATEST);
    Ok(())
}
#[test]
fn test_versions_one_segment() -> Result<(), TestError> {
    let mut random = my_random("test_versions_one_segment".to_string());
    let dir = new_directory(&mut random)?;
    let directory = Arc::new(Mutex::new(dir));
    let id = StringHelper::random_id();
    let codec = get_default_code();
    let mut sis = SegmentInfos::<_, DummyFieldComparatorSource>::new(LATEST.major)?;
    let mut info = SegmentInfo::new(
        directory.clone(),
        Some((*LUCENE_11_0_0).clone()),
        Some((*LUCENE_11_0_0).clone()),
        "_0".to_string(),
        Some(1),
        false,
        false,
        Some(get_default_code()),
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    info.set_files(HashSet::new());
    codec.segment_info_format().write(
        directory.clone(),
        &mut info,
        IOContext::default_io_context()?,
    )?;

    let commit_info = SegmentCommitInfo::new(
        info,
        0,
        0,
        -1,
        -1,
        -1,
        Some(Vec::from(StringHelper::random_id())),
    )?;

    sis.add(commit_info)?;
    sis.commit(directory.clone())?;

    let result = SegmentInfos::read_latest_commit(directory.clone())?.into_segment_infos();
    assert!(result.is_some());
    sis = result.unwrap();
    assert_eq!(
        *sis.get_min_segment_lucene_version().unwrap(),
        (*LUCENE_11_0_0).clone()
    );
    assert_eq!(*sis.get_commit_lucene_version().unwrap(), (*LATEST).clone());

    Ok(())
}

#[test]
fn test_versions_two_segments() -> Result<(), TestError> {
    let mut random = my_random("test_versions_two_segments".to_string());
    let dir = new_directory(&mut random)?;
    let directory = Arc::new(Mutex::new(dir));
    let id = StringHelper::random_id();
    let codec = get_default_code();
    let mut sis = SegmentInfos::<_, DummyFieldComparatorSource>::new(LATEST.major)?;
    // First Segment
    let mut info_0 = SegmentInfo::new(
        directory.clone(),
        Some((*LUCENE_11_0_0).clone()),
        Some((*LUCENE_11_0_0).clone()),
        "_0".to_string(),
        Some(1),
        false,
        false,
        Some(get_default_code()),
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    info_0.set_files(HashSet::new());
    codec.segment_info_format().write(
        directory.clone(),
        &mut info_0,
        IOContext::default_io_context()?,
    )?;

    let commit_info_0 = SegmentCommitInfo::new(
        info_0,
        0,
        0,
        -1,
        -1,
        -1,
        Some(Vec::from(StringHelper::random_id())),
    )?;
    sis.add(commit_info_0)?;

    // Second Segment
    let mut info_1 = SegmentInfo::new(
        directory.clone(),
        Some((*LUCENE_11_0_0).clone()),
        Some((*LUCENE_11_0_0).clone()),
        "_1".to_string(),
        Some(1),
        false,
        false,
        Some(get_default_code()),
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    info_1.set_files(HashSet::new());
    codec.segment_info_format().write(
        directory.clone(),
        &mut info_1,
        IOContext::default_io_context()?,
    )?;

    let commit_info_1 = SegmentCommitInfo::new(
        info_1,
        0,
        0,
        -1,
        -1,
        -1,
        Some(Vec::from(StringHelper::random_id())),
    )?;
    sis.add(commit_info_1)?;
    sis.commit(directory.clone())?;

    let commit_info_id_0 = sis.info(0).unwrap().get_id().clone();
    let commit_info_id_1 = sis.info(1).unwrap().get_id().clone();

    // Read back the latest commit
    let result = SegmentInfos::read_latest_commit(directory.clone())?.into_segment_infos();
    assert!(result.is_some());
    sis = result.unwrap();

    // Verify results
    assert_eq!(
        *sis.get_min_segment_lucene_version().unwrap(),
        (*LUCENE_11_0_0).clone()
    );
    assert_eq!(*sis.get_commit_lucene_version().unwrap(), (*LATEST).clone());
    let actual1 = sis.info(0).unwrap().get_id();
    let actual2 = sis.info(1).unwrap().get_id();
    assert_eq!(
        StringHelper::id_to_string(Option::from(commit_info_id_0.as_ref().unwrap().as_slice())),
        StringHelper::id_to_string(Option::from(actual1.as_ref().unwrap().as_slice()))
    );
    assert_eq!(
        StringHelper::id_to_string(Option::from(commit_info_id_1.as_ref().unwrap().as_slice())),
        StringHelper::id_to_string(Option::from(actual2.as_ref().unwrap().as_slice()))
    );

    Ok(())
}
#[test]
fn test_to_string() -> Result<(), TestError> {
    let mut random = my_random("test_to_string".to_string());
    let dir = Arc::new(Mutex::new(new_directory(&mut random)?));
    let codec = get_default_code();

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
        "TEST".to_string(),
        Some(10000),
        false,
        false,
        Some(codec.clone()),
        HashMap::new(),
        Vec::from(StringHelper::random_id()),
        HashMap::new(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
    )?;
    assert_eq!(
        format!("TEST({}){}:[indexSort=<doc>]", LATEST, ":C10000"),
        format!("{}", si)
    );

    // diagnostics O, attributes X
    let si = SegmentInfo::new(
        dir.clone(),
        Some((*LATEST).clone()),
        Some((*LATEST).clone()),
        "TEST".to_string(),
        Some(10000),
        false,
        false,
        Some(codec.clone()),
        diagnostics.clone(),
        Vec::from(StringHelper::random_id()),
        HashMap::new(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
    )?;
    assert_eq!(
        format!(
            "TEST({}){}:[indexSort=<doc>]:[diagnostics={:?}]",
            LATEST, ":C10000", diagnostics
        ),
        format!("{}", si)
    );

    // diagnostics X, attributes O
    let si = SegmentInfo::new(
        dir.clone(),
        Some((*LATEST).clone()),
        Some((*LATEST).clone()),
        "TEST".to_string(),
        Some(10000),
        false,
        false,
        Some(codec.clone()),
        HashMap::new(),
        Vec::from(StringHelper::random_id()),
        attributes.clone(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
    )?;
    assert_eq!(
        format!(
            "TEST({}){}:[indexSort=<doc>]:[attributes={:?}]",
            LATEST, ":C10000", attributes
        ),
        format!("{}", si)
    );

    // diagnostics O, attributes O
    let si = SegmentInfo::new(
        dir.clone(),
        Some((*LATEST).clone()),
        Some((*LATEST).clone()),
        "TEST".to_string(),
        Some(10000),
        false,
        false,
        Some(codec),
        diagnostics.clone(),
        Vec::from(StringHelper::random_id()),
        attributes.clone(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
    )?;
    assert_eq!(
        format!(
            "TEST({}){}:[indexSort=<doc>]:[diagnostics={:?}]:[attributes={:?}]",
            LATEST, ":C10000", diagnostics, attributes
        ),
        format!("{}", si)
    );
    Ok(())
}
#[test]
fn test_id_changes_on_advance() -> Result<(), TestError> {
    let mut random = my_random("test_id_changes_on_advance".to_string());
    let dir = Arc::new(Mutex::new(new_directory(&mut random)?));
    let id = StringHelper::random_id();

    let info = SegmentInfo::new(
        dir.clone(),
        Some((*LUCENE_10_0_0).clone()),
        Some((*LUCENE_10_0_0).clone()),
        "_0".to_string(),
        Some(1),
        false,
        false,
        Some(get_default_code()),
        HashMap::new(),
        Vec::from(StringHelper::random_id()),
        HashMap::new(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
    )?;

    let mut commit_info = SegmentCommitInfo::new(info, 0, 0, -1, -1, -1, Some(Vec::from(id)))?;
    assert_eq!(
        StringHelper::id_to_string(Some(id.as_slice())),
        StringHelper::id_to_string(commit_info.get_id().as_deref())
    );

    commit_info.advance_del_gen();
    assert_ne!(
        StringHelper::id_to_string(Some(id.as_slice())),
        StringHelper::id_to_string(commit_info.get_id().as_deref())
    );

    let new_id = commit_info.get_id().clone();
    commit_info.advance_doc_values_gen();
    assert_ne!(
        StringHelper::id_to_string(new_id.as_deref()),
        StringHelper::id_to_string(commit_info.get_id().as_deref())
    );

    let new_id = commit_info.get_id().clone();
    commit_info.advance_field_infos_gen();
    assert_ne!(
        StringHelper::id_to_string(new_id.as_deref()),
        StringHelper::id_to_string(commit_info.get_id().as_deref())
    );

    let clone = commit_info.clone();
    let current_id = commit_info.get_id().clone();
    assert_eq!(
        StringHelper::id_to_string(current_id.as_deref()),
        StringHelper::id_to_string(commit_info.get_id().as_deref())
    );
    assert_eq!(
        StringHelper::id_to_string(current_id.as_deref()),
        StringHelper::id_to_string(clone.get_id().as_deref())
    );

    commit_info.advance_field_infos_gen();
    assert_ne!(
        StringHelper::id_to_string(current_id.as_deref()),
        StringHelper::id_to_string(commit_info.get_id().as_deref())
    );
    assert_eq!(
        StringHelper::id_to_string(current_id.as_deref()),
        StringHelper::id_to_string(clone.get_id().as_deref()),
        "clone changed but shouldn't"
    );

    Ok(())
}
#[test]
fn test_bit_flipped_triggers_corrupt_index_exception() -> Result<(), TestError> {
    let mut random = my_random("test_bit_flipped_triggers_corrupt_index_exception".to_string());
    let dir = Arc::new(Mutex::new(new_directory(&mut random)?));
    let id = StringHelper::random_id();
    let codec = get_default_code();
    let mut sis = SegmentInfos::<_, DummyFieldComparatorSource>::new(LATEST.major)?;
    let mut info_0 = SegmentInfo::new(
        dir.clone(),
        Some((*LATEST).clone()),
        Some((*LATEST).clone()),
        "_0".to_string(),
        Some(1),
        false,
        false,
        Some(codec.clone()),
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    info_0.set_files(HashSet::new());
    codec.segment_info_format().write(
        dir.clone(),
        &mut info_0,
        IOContext::default_io_context()?,
    )?;
    let commit_info_0 = SegmentCommitInfo::new(
        info_0,
        0,
        0,
        -1,
        -1,
        -1,
        Some(Vec::from(StringHelper::random_id())),
    )?;
    sis.add(commit_info_0)?;

    // Add second SegmentCommitInfo
    let mut info_1 = SegmentInfo::new(
        dir.clone(),
        Some((*LATEST).clone()),
        Some((*LATEST).clone()),
        "_1".to_string(),
        Some(1),
        false,
        false,
        Some(codec.clone()),
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    info_1.set_files(HashSet::new());
    codec.segment_info_format().write(
        dir.clone(),
        &mut info_1,
        IOContext::default_io_context()?,
    )?;
    let commit_info_1 = SegmentCommitInfo::new(
        info_1,
        0,
        0,
        -1,
        -1,
        -1,
        Some(Vec::from(StringHelper::random_id())),
    )?;
    sis.add(commit_info_1)?;

    sis.commit(dir.clone())?;

    // Create a corrupt directory
    let corrupt_dir = Arc::new(Mutex::new(new_directory(&mut random)?));
    let mut corrupt = false;
    {
        let mut corrupt_directory = corrupt_dir.lock().unwrap();
        let directory = dir.lock().unwrap();
        for file in directory.list_all()? {
            if file.starts_with(IndexFileNames::SEGMENTS) {
                {
                    let mut input =
                        directory.open_input(&file, IOContext::read_once_io_context()?)?;
                    let mut output =
                        corrupt_directory.create_output(&file, IOContext::default_io_context()?)?;

                    let mut input_length = IndexInput::length(&input);
                    let corrupt_index = random.gen_range(0..input_length - 1);
                    output.copy_bytes(&mut input, corrupt_index)?;

                    let byte = DataInput::read_byte(&mut input)?;
                    let value = random.gen_range(0x01..0xff);
                    let corrupt_byte = byte.wrapping_add(value);
                    output.write_byte(corrupt_byte)?;
                    input_length = IndexInput::length(&input);
                    let file_pointer = input.get_file_pointer();
                    output.copy_bytes(&mut input, input_length - file_pointer)?;
                }
                let mut input =
                    corrupt_directory.open_input(&file, IOContext::read_once_io_context()?)?;
                match CodecUtil::checksum_entire_file(&mut input) {
                    Ok(_) => {
                        if cfg!(feature = "verbose") {
                            println!(
                                "TEST: Altering the file did not update the checksum, aborting..."
                            );
                        }
                        return Ok(());
                    }
                    Err(LuceneError::CorruptIndex(_)) => {
                        // Corruption detected
                    }
                    Err(err) => return Err(err.into()),
                }
                corrupt = true;
            } else if file.eq("extra0") {
                corrupt_directory.copy_from(
                    dir.clone(),
                    &file,
                    &file,
                    IOContext::default_io_context()?,
                )?;
            }
        }
    }

    assert!(corrupt, "No segments file found");

    let result =
        SegmentInfos::<_, DummyFieldComparatorSource>::read_latest_commit(corrupt_dir.clone());
    assert!(result.is_err());
    match result {
        Err(LuceneError::CorruptIndex(_))
        | Err(LuceneError::IndexFormatTooOld(_))
        | Err(LuceneError::IndexFormatTooNew(_)) => {}
        _ => {
            unreachable!()
        }
    }

    Ok(())
}
#[test]
fn test_add_diagnostics() -> Result<(), TestError> {
    let mut random = my_random("test_add_diagnostics".to_string());
    let dir = Arc::new(Mutex::new(new_directory(&mut random)?));
    let codec = get_default_code();

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
        "TEST".to_string(),
        Some(10000),
        false,
        false,
        Some(codec.clone()),
        diagnostics.clone(),
        Vec::from(StringHelper::random_id()),
        HashMap::new(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
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
        "TEST".to_string(),
        Some(10000),
        false,
        false,
        Some(codec.clone()),
        diagnostics.clone(),
        Vec::from(StringHelper::random_id()),
        HashMap::new(),
        Some(Sort::<DummyFieldComparatorSource>::get_index_order()?),
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
