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
use crate::index::base_compound_format_test_case::{
    create_random_file, new_segment_info, BaseCompoundFormatTestCase,
};
use crate::util::lucene_test_case::new_directory;
use crate::util::test_error::TestError;
use rand::prelude::SliceRandom;
use rand::Rng;
use rlucene::codecs::{Codec, CodecUtil, CompoundFormat, Lucene90CompoundFormat, LATEST_CODEC};
use rlucene::index::IndexFileNames;
use rlucene::store::directory::Directory;
use rlucene::store::{DataInput, IO_CONTEXT_DEFAULT};
use rlucene::util::error::lucene_error::LuceneError;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct TestLucene90CompoundFormat;
impl BaseCompoundFormatTestCase for TestLucene90CompoundFormat {}
#[test]
fn test_empty() -> Result<(), TestError> {
    let mut random = my_random("test_empty".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_empty(&mut random)
}
#[test]
fn test_single_file() -> Result<(), TestError> {
    let mut random = my_random("test_single_file".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_single_file(&mut random)
}
#[test]
fn test_two_files() -> Result<(), TestError> {
    let mut random = my_random("test_two_files".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_two_files(&mut random)
}
#[test]
fn test_double_close() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_double_close()
}
#[test]
fn test_pass_io_context() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_pass_io_context()
}
#[test]
fn test_large_cfs() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_large_cfs()
}
#[test]
fn test_list_all() -> Result<(), TestError> {
    let case = TestLucene90CompoundFormat;
    case.test_list_all()
}
#[test]
fn test_create_output_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_create_output_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_create_output_disabled(&mut random)
}
#[test]
fn test_delete_file_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_delete_file_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_delete_file_disabled(&mut random)
}
#[test]
fn test_rename_file_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_rename_file_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_rename_file_disabled(&mut random)
}
#[test]
fn test_sync_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_sync_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_sync_disabled(&mut random)
}
#[test]
fn test_make_lock_disabled() -> Result<(), TestError> {
    let mut random = my_random("test_make_lock_disabled".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_make_lock_disabled(&mut random)
}
#[test]
fn test_random_files() -> Result<(), TestError> {
    let mut random = my_random("test_random_files".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_random_files(&mut random)
}
#[test]
fn test_many_sub_files() -> Result<(), TestError> {
    let mut random = my_random("test_many_sub_files".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_many_sub_files(&mut random)
}
#[test]
fn test_cloned_streams_closing() -> Result<(), TestError> {
    let mut random = my_random("test_cloned_streams_closing".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_cloned_streams_closing(&mut random)
}
#[test]
fn test_random_access() -> Result<(), TestError> {
    let mut random = my_random("test_random_access".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_random_access(&mut random)
}
#[test]
fn test_random_access_clones() -> Result<(), TestError> {
    let mut random = my_random("test_random_access_clones".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_random_access_clones(&mut random)
}
#[test]
fn test_file_not_found() -> Result<(), TestError> {
    let mut random = my_random("test_file_not_found".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_file_not_found(&mut random)
}
#[test]
fn test_read_past_eof() -> Result<(), TestError> {
    let mut random = my_random("test_read_past_eof".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_read_past_eof(&mut random)
}
#[test]
fn test_resource_name_inside_compound_file() -> Result<(), TestError> {
    let mut random = my_random("test_resource_name_inside_compound_file".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_resource_name_inside_compound_file(&mut random)
}
#[test]
fn test_missing_codec_headers_are_caught() -> Result<(), TestError> {
    let mut random = my_random("test_missing_codec_headers_are_caught".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_missing_codec_headers_are_caught(&mut random)
}
#[test]
fn test_corrupt_files_are_caught() -> Result<(), TestError> {
    let mut random = my_random("test_corrupt_files_are_caught".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_corrupt_files_are_caught(&mut random)
}
#[test]
fn test_check_integrity() -> Result<(), TestError> {
    let mut random = my_random("test_check_integrity".to_string());
    let case = TestLucene90CompoundFormat;
    case.test_check_integrity(&mut random)
}

#[test]
fn test_file_length_ordering() -> Result<(), TestError> {
    let mut random = my_random("test_file_length_ordering".to_string());
    let dir = Arc::new(Mutex::new(new_directory(&mut random)?));
    let segment = "_123";
    let chunk = 1024; // internal buffer size used by the stream
    let mut si = new_segment_info(&mut random, dir.clone(), segment)?;

    let seg_id = si.get_id();
    let mut ordered_files = Vec::new();
    let mut random_file_size = random.gen_range(0..chunk);

    for i in 0..10 {
        let filename = format!("{}.{}", segment, i);
        create_random_file(&mut random, &dir, &filename, random_file_size, &seg_id)?;
        random_file_size += random.gen_range(1..100);
        ordered_files.push(filename);
    }

    let mut shuffled_files = ordered_files.clone();
    shuffled_files.shuffle(&mut random);
    let files = shuffled_files.into_iter().collect();
    si.set_files(files);

    LATEST_CODEC
        .compound_format()
        .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;

    // Entries file should contain files ordered by their size
    let entries_file_name =
        IndexFileNames::segment_file_name(&si.name, "", Lucene90CompoundFormat::ENTRIES_EXTENSION);
    let mut entries_stream = dir
        .lock()
        .unwrap()
        .open_checksum_input(&entries_file_name)?;

    let mut prior_e = None;
    let result: Result<(), LuceneError> = (|| {
        CodecUtil::check_index_header(
            &mut entries_stream,
            Lucene90CompoundFormat::ENTRY_CODEC,
            Lucene90CompoundFormat::VERSION_START,
            Lucene90CompoundFormat::VERSION_CURRENT,
            &si.get_id(),
            "",
        )?;

        let num_entries = entries_stream.read_vint()?;
        let mut last_offset = 0;
        let mut last_length = 0;
        for i in 0..num_entries {
            let id = entries_stream.read_string()?;
            assert_eq!(ordered_files[i as usize], format!("{}{}", segment, id));
            let offset = entries_stream.read_long()?;
            assert!(offset > last_offset);
            last_offset = offset;
            let length = entries_stream.read_long()?;
            assert!(length >= last_length);
            last_length = length;
        }
        Ok(())
    })();
    if let Err(e) = result {
        prior_e = Some(e);
    }

    if prior_e.is_some() {
        CodecUtil::check_footer_with_error(&mut entries_stream, &mut prior_e.unwrap())?;
    } else {
        CodecUtil::check_footer(&mut entries_stream)?;
    }
    Ok(())
}
