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
use crate::util::lucene_test_case::{new_directory, new_io_context};
use crate::util::test_error::TestError;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::codecs::{Codec, CodecUtil, CompoundFormat, LATEST_CODEC};
use rlucene::index::segment_info::SegmentInfo;
use rlucene::store::directory::Directory;
use rlucene::store::{DataInput, DataOutput, IOContext};
use rlucene::store::{IndexInput, IO_CONTEXT_DEFAULT};
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::{StringHelper, LATEST};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub trait BaseCompoundFormatTestCase {
    fn test_empty(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        assert_eq!(0, cfs.list_all()?.len());
        Ok(())
    }
    /// This test creates compound file based on a single file. Files of different sizes are tested: 0,
    /// 1, 10, 100 bytes.
    fn test_single_file(&self, random: &mut StdRng) -> Result<(), TestError> {
        let data = [0, 1, 10, 100];
        for (i, &size) in data.iter().enumerate() {
            let test_file = format!("_{}.test", i);
            let dir = Arc::new(Mutex::new(new_directory(random)?));
            let mut si = new_segment_info(random, dir.clone(), &format!("_{}", i))?;
            create_sequence_file(
                random,
                dir.clone(),
                &test_file,
                0,
                size,
                si.get_id().as_slice(),
                "suffix",
            )?;

            si.set_files(HashSet::from([test_file.clone()]));
            LATEST_CODEC
                .compound_format()
                .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;

            let cfs = LATEST_CODEC
                .compound_format()
                .get_compound_reader(dir.clone(), &si)?;

            let mut expected = dir
                .lock()
                .unwrap()
                .open_input(&test_file, &new_io_context(random)?)?;
            let mut actual = cfs.open_input(&test_file, &new_io_context(random)?)?;

            assert_same_streams(&test_file, &mut expected, &mut actual)?;
            assert_same_seek_behavior(&test_file, &mut expected, &mut actual)?;
        }
        Ok(())
    }
    /// This test creates compound file based on two files.
    fn test_two_files(&self, random: &mut StdRng) -> Result<(), TestError> {
        let files = ["_123.d1", "_123.d2"];
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut si = new_segment_info(random, dir.clone(), "_123")?;

        create_sequence_file(
            random,
            dir.clone(),
            files[0],
            0,
            15,
            si.get_id().as_slice(),
            "suffix",
        )?;
        create_sequence_file(
            random,
            dir.clone(),
            files[1],
            0,
            114,
            si.get_id().as_slice(),
            "suffix",
        )?;

        let files_set: HashSet<String> = files.iter().map(|&file| file.to_string()).collect();
        si.set_files(files_set);
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;

        for file in files.iter() {
            let mut expected = dir
                .lock()
                .unwrap()
                .open_input(file, &new_io_context(random)?)?;
            let mut actual = cfs.open_input(file, &new_io_context(random)?)?;
            assert_same_streams(file, &mut expected, &mut actual)?;
            assert_same_seek_behavior(file, &mut expected, &mut actual)?;
        }
        Ok(())
    }
    fn test_double_close(&self) -> Result<(), TestError> {
        // Rust Lucene not need close manually
        Ok(())
    }
    /// This test ensures that IOContext is passed correctly in contexts like NRTCachingDir.
    /// It checks that IOContext is properly propagated when interacting with the `Directory`.
    fn test_pass_io_context(&self) -> Result<(), TestError> {
        // TODO: FilterDirectory not implemented, so this test could not be implemented
        Ok(())
    }
    fn test_large_cfs(&self) -> Result<(), TestError> {
        // TODO: NRTCachingDirectory not implemented, so this test could not be implemented
        Ok(())
    }
    fn test_list_all(&self) -> Result<(), TestError> {
        // TODO: RandomIndexWriter not implemented, so this test could not be implemented
        Ok(())
    }
    /// Test that the compound file system (CFS) reader is read-only by attempting to create an output.
    fn test_create_output_disabled(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let mut cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let io_context = IOContext::default_io_context()?;
        let result = cfs.create_output("bogus", &io_context);
        assert!(matches!(result, Err(LuceneError::UnsupportedOperation(_))));
        Ok(())
    }
    /// Test that the CFS reader is read-only, and that `deleteFile` is disabled.
    fn test_delete_file_disabled(&self, random: &mut StdRng) -> Result<(), TestError> {
        let testfile = "_123.test";
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut out = dir
            .lock()
            .unwrap()
            .create_output(testfile, &IOContext::default_io_context()?)?;
        out.write_int(3)?;
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let mut cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let result = cfs.delete_file(testfile);
        assert!(matches!(result, Err(LuceneError::UnsupportedOperation(_))));
        Ok(())
    }
    /// Test that the CFS reader is read-only, and that `rename` is disabled.
    fn test_rename_file_disabled(&self, random: &mut StdRng) -> Result<(), TestError> {
        let testfile = "_123.test";
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut out = dir
            .lock()
            .unwrap()
            .create_output(testfile, &IOContext::default_io_context()?)?;
        out.write_int(3)?;
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let mut cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let result = cfs.rename(testfile, "bogus");
        assert!(matches!(result, Err(LuceneError::UnsupportedOperation(_))));
        Ok(())
    }
    /// Test that the CFS reader is read-only, and that `sync` is disabled.
    fn test_sync_disabled(&self, random: &mut StdRng) -> Result<(), TestError> {
        let testfile = "_123.test";
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut out = dir
            .lock()
            .unwrap()
            .create_output(testfile, &IOContext::default_io_context()?)?;
        out.write_int(3)?;
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let mut cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let result = cfs.sync(&[testfile]);
        assert!(matches!(result, Err(LuceneError::UnsupportedOperation(_))));
        Ok(())
    }

    /// Test that the CFS reader is read-only, and that obtaining locks is disabled.
    fn test_make_lock_disabled(&self, random: &mut StdRng) -> Result<(), TestError> {
        let testfile = "_123.test";
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut out = dir
            .lock()
            .unwrap()
            .create_output(testfile, &IOContext::default_io_context()?)?;
        out.write_int(3)?;
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let mut cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let result = cfs.obtain_lock("foobar");
        assert!(matches!(result, Err(LuceneError::UnsupportedOperation(_))));
        Ok(())
    }
    /// This test creates a compound file based on a large number of files of various length.
    /// The file content is generated randomly. The sizes range from 0 to 1Mb.
    /// Some of the sizes are selected to test the buffering logic in the file reading code.
    /// For this, the chunk variable is set to the length of the buffer used internally by the compound file logic.
    fn test_random_files(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let segment = "_123";
        let chunk = 1024; // internal buffer size used by the stream
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        let seg_id = si.get_id();
        create_random_file(random, &dir, &format!("{}.zero", segment), 0, &seg_id)?;
        create_random_file(random, &dir, &format!("{}.one", segment), 1, &seg_id)?;
        create_random_file(random, &dir, &format!("{}.ten", segment), 10, &seg_id)?;
        create_random_file(random, &dir, &format!("{}.hundred", segment), 100, &seg_id)?;
        create_random_file(random, &dir, &format!("{}.big1", segment), chunk, &seg_id)?;
        create_random_file(
            random,
            &dir,
            &format!("{}.big2", segment),
            chunk - 1,
            &seg_id,
        )?;
        create_random_file(
            random,
            &dir,
            &format!("{}.big3", segment),
            chunk + 1,
            &seg_id,
        )?;
        create_random_file(
            random,
            &dir,
            &format!("{}.big4", segment),
            3 * chunk,
            &seg_id,
        )?;
        create_random_file(
            random,
            &dir,
            &format!("{}.big5", segment),
            3 * chunk - 1,
            &seg_id,
        )?;
        create_random_file(
            random,
            &dir,
            &format!("{}.big6", segment),
            3 * chunk + 1,
            &seg_id,
        )?;
        create_random_file(
            random,
            &dir,
            &format!("{}.big7", segment),
            1000 * chunk,
            &seg_id,
        )?;
        let files: Vec<String> = dir
            .lock()
            .unwrap()
            .list_all()?
            .into_iter()
            .filter(|file| file.starts_with(segment))
            .collect();
        si.set_files(files.iter().cloned().collect());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;

        // Validate each file
        for file in files.iter() {
            let mut check = dir
                .lock()
                .unwrap()
                .open_input(file, &new_io_context(random)?)?;
            let mut test = cfs.open_input(file, &new_io_context(random)?)?;
            assert_same_streams(file, &mut check, &mut test)?;
            assert_same_seek_behavior(file, &mut check, &mut test)?;
        }

        Ok(())
    }

    fn test_many_sub_files(&self, random: &mut StdRng) -> Result<(), TestError> {
        // TODO: should enhance after implementing the newMockFSDirectory
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        const FILE_COUNT: usize = 500;
        let mut files = Vec::new();
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        for file_idx in 0..FILE_COUNT {
            let file = format!("_123.{}", file_idx);
            files.push(file.clone());
            let mut out = dir
                .lock()
                .unwrap()
                .create_output(&file, &new_io_context(random)?)?;
            CodecUtil::write_index_header(&mut out, "Foo", 0, &si.get_id(), "suffix")?;
            out.write_byte(file_idx as u8)?;
            CodecUtil::write_footer(&mut out)?;
        }
        let file_sets = files.iter().cloned().collect();
        si.set_files(file_sets);
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let mut ins = Vec::with_capacity(FILE_COUNT);
        // Open the files
        for file_idx in 0..FILE_COUNT {
            let file = format!("_123.{}", file_idx);
            let mut input = cfs.open_input(&file, &new_io_context(random)?)?;
            CodecUtil::check_index_header(&mut input, "Foo", 0, 0, &si.get_id(), "suffix")?;
            ins.push(input);
        }
        // assert_eq!(dir.lock().unwrap().get_file_handle_count(), 1);
        for (file_idx, input) in ins.iter_mut().enumerate() {
            assert_eq!(input.read_byte()?, file_idx as u8);
        }
        // Ensure only one file handle is used
        // assert_eq!(dir.lock().unwrap().get_file_handle_count(), 1);
        // for input in ins.iter_mut() {
        //     input.close()?;
        // }
        Ok(())
    }
}

fn new_segment_info<D: Directory>(
    random: &mut StdRng,
    dir: Arc<Mutex<D>>,
    name: &str,
) -> Result<SegmentInfo<D>, TestError> {
    let min_version = if random.gen_bool(0.5) {
        None
    } else {
        Some((*LATEST).clone())
    };
    let id = StringHelper::random_id();
    let value = SegmentInfo::new(
        dir,
        Some((*LATEST).clone()),
        min_version,
        name.to_string(),
        Option::from(10_000),
        false,
        false,
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    Ok(value)
}
/// Creates a file of the specified size with random data.
fn create_random_file<D: Directory>(
    random: &mut StdRng,
    dir: &Arc<Mutex<D>>,
    name: &str,
    size: i32,
    seg_id: &[u8],
) -> Result<(), TestError> {
    let mut os = dir
        .lock()
        .unwrap()
        .create_output(name, &new_io_context(random)?)?;
    CodecUtil::write_index_header(&mut os, "Foo", 0, seg_id, "suffix")?;

    for _ in 0..size {
        let b = random.gen_range(0..256) as u8;
        os.write_byte(b)?;
    }
    CodecUtil::write_footer(&mut os)?;
    Ok(())
}

/// Creates a file of the specified size with sequential data. The first byte is written as the
/// start byte provided. All subsequent bytes are computed as start + offset where offset is the
/// number of the byte.
fn create_sequence_file<D: Directory>(
    random: &mut StdRng,
    dir: Arc<Mutex<D>>,
    name: &str,
    mut start: u8,
    size: i32,
    seg_id: &[u8],
    seg_suffix: &str,
) -> Result<(), TestError> {
    let mut os = dir
        .lock()
        .unwrap()
        .create_output(name, &new_io_context(random)?)?;
    CodecUtil::write_index_header(&mut os, "Foo", 0, seg_id, seg_suffix)?;
    for _ in 0..size {
        os.write_byte(start)?;
        start += 1;
    }
    CodecUtil::write_footer(&mut os)?;

    Ok(())
}

fn assert_same_streams<D: IndexInput>(
    msg: &str,
    expected: &mut D,
    test: &mut D,
) -> Result<(), TestError> {
    assert_eq!(expected.length(), test.length(), "{} length", msg);
    assert_eq!(
        expected.get_file_pointer(),
        test.get_file_pointer(),
        "{} position",
        msg
    );

    let mut expected_buffer = vec![0u8; 512];
    let expected_len = expected.length();
    let mut test_buffer = vec![0u8; expected_len as usize];

    let mut remainder = expected.length() - expected.get_file_pointer();
    while remainder > 0 {
        let read_len = remainder.min(expected_buffer.len() as i64) as usize;
        expected.read_bytes(&mut expected_buffer[..read_len], 0, read_len as i32)?;
        test.read_bytes(&mut test_buffer[..read_len], 0, read_len as i32)?;
        assert_equal_arrays(msg, &expected_buffer, &test_buffer, 0, read_len);
        remainder -= read_len as i64;
    }
    Ok(())
}
fn assert_same_streams_seek_with_seek<D: IndexInput>(
    msg: &str,
    expected: &mut D,
    actual: &mut D,
    seek_to: i64,
) -> Result<(), TestError> {
    if seek_to >= 0 && seek_to < expected.length() {
        expected.seek(seek_to)?;
        actual.seek(seek_to)?;
        assert_same_streams(msg, expected, actual)?;
    }
    Ok(())
}

fn assert_same_seek_behavior<D: IndexInput>(
    msg: &str,
    expected: &mut D,
    actual: &mut D,
) -> Result<(), TestError> {
    // Seek to 0
    let point = 0;
    assert_same_streams_seek_with_seek(msg, expected, actual, point)?;

    // Seek to middle
    let point = expected.length() / 2;
    assert_same_streams_seek_with_seek(msg, expected, actual, point)?;

    // Seek to end - 2
    let point = expected.length() - 2;
    assert_same_streams_seek_with_seek(msg, expected, actual, point)?;

    // Seek to end - 1
    let point = expected.length() - 1;
    assert_same_streams_seek_with_seek(msg, expected, actual, point)?;

    // Seek to the end
    let point = expected.length();
    assert_same_streams_seek_with_seek(msg, expected, actual, point)?;

    // Seek past the end
    let point = expected.length() + 1;
    assert_same_streams_seek_with_seek(msg, expected, actual, point)?;

    Ok(())
}

fn assert_equal_arrays(msg: &str, expected: &[u8], test: &[u8], start: usize, len: usize) {
    assert!(!expected.is_empty(), "{} null expected", msg);
    assert!(!test.is_empty(), "{} null test", msg);

    for i in start..len {
        assert_eq!(expected[i], test[i], "{} {}", msg, i);
    }
}
