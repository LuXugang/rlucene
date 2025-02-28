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
use crate::codecs::compound_directory::CompoundDirectory;
use crate::codecs::{Codec, CodecUtil, CompoundFormat, LATEST_CODEC};
use crate::index::segment_info::SegmentInfo;
use crate::store::directory::Directory;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::IndexOutput;
use crate::store::{DataInput, DataOutput, IOContext};
use crate::store::{IndexInput, IO_CONTEXT_DEFAULT};
use crate::test::util::lucene_test_case::{at_least, new_directory, new_io_context};

use crate::util::error::lucene_error::LuceneError;
use crate::util::{StringHelper, LATEST};
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub trait BaseCompoundFormatTestCase {
    fn test_empty(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_single_file(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_two_files(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_double_close(&self) -> Result<(), LuceneError> {
        // Rust Lucene not need close manually
        Ok(())
    }
    /// This test ensures that IOContext is passed correctly in contexts like NRTCachingDir.
    /// It checks that IOContext is properly propagated when interacting with the `Directory`.
    fn test_pass_io_context(&self) -> Result<(), LuceneError> {
        // TODO: FilterDirectory not implemented, so this test could not be implemented
        Ok(())
    }
    fn test_large_cfs(&self) -> Result<(), LuceneError> {
        // TODO: NRTCachingDirectory not implemented, so this test could not be implemented
        Ok(())
    }
    fn test_list_all(&self) -> Result<(), LuceneError> {
        // TODO: RandomIndexWriter not implemented, so this test could not be implemented
        Ok(())
    }
    /// Test that the compound file system (CFS) reader is read-only by attempting to create an output.
    fn test_create_output_disabled(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_delete_file_disabled(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_rename_file_disabled(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_sync_disabled(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_make_lock_disabled(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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
    fn test_random_files(&self, random: &mut StdRng) -> Result<(), LuceneError> {
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

    fn test_many_sub_files(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        // TODO: should enhance after implementing the newMockFSDirectory
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let file_count = at_least(random, 500) as usize;
        let mut files = Vec::new();
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        for file_idx in 0..file_count {
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
        let mut ins = Vec::with_capacity(file_count);
        // Open the files
        for file_idx in 0..file_count {
            let file = format!("_123.{}", file_idx);
            let mut input = cfs.open_input(&file, &new_io_context(random)?)?;
            CodecUtil::check_index_header(&mut input, "Foo", 0, 0, &si.get_id(), "suffix")?;
            ins.push(input);
        }
        // assert_eq!(dir.lock().unwrap().get_file_handle_count(), 1);
        for (file_idx, input) in ins.iter_mut().enumerate() {
            assert_eq!(DataInput::read_byte(input)?, file_idx as u8);
        }
        // Ensure only one file handle is used
        // assert_eq!(dir.lock().unwrap().get_file_handle_count(), 1);
        // for input in ins.iter_mut() {
        //     input.close()?;
        // }
        Ok(())
    }
    fn test_cloned_streams_closing(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let cr = create_large_cfs(random, dir.clone())?;

        let mut expected = dir
            .lock()
            .unwrap()
            .open_input("_123.f11", &new_io_context(random)?)?;
        let mut one = cr.open_input("_123.f11", &new_io_context(random)?)?;
        let mut two = one.clone();

        assert_same_streams("basic clone one", &mut expected, &mut one)?;
        expected.seek(0)?;
        assert_same_streams("basic clone two", &mut expected, &mut two)?;
        Ok(())
    }
    /// This test opens two files from a compound stream and verifies that their file positions are
    /// independent of each other.
    fn test_random_access(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let cr = create_large_cfs(random, dir.clone())?;

        // Open two files
        let mut e1 = dir
            .lock()
            .unwrap()
            .open_input("_123.f11", &new_io_context(random)?)?;
        let mut e2 = dir
            .lock()
            .unwrap()
            .open_input("_123.f3", &new_io_context(random)?)?;

        let mut a1 = cr.open_input("_123.f11", &new_io_context(random)?)?;
        let mut a2 = dir
            .lock()
            .unwrap()
            .open_input("_123.f3", &new_io_context(random)?)?;

        // Seek the first pair
        e1.seek(100)?;
        a1.seek(100)?;
        assert_eq!(100, e1.get_file_pointer());
        assert_eq!(100, a1.get_file_pointer());
        let be1 = DataInput::read_byte(&mut e1)?;
        let ba1 = DataInput::read_byte(&mut a1)?;
        assert_eq!(be1, ba1);

        // Now seek the second pair
        e2.seek(1027)?;
        a2.seek(1027)?;
        assert_eq!(1027, e2.get_file_pointer());
        assert_eq!(1027, a2.get_file_pointer());
        let be2 = DataInput::read_byte(&mut e2)?;
        let ba2 = DataInput::read_byte(&mut a2)?;
        assert_eq!(be2, ba2);

        // Now make sure the first one didn't move
        assert_eq!(101, e1.get_file_pointer());
        assert_eq!(101, a1.get_file_pointer());
        let be1 = DataInput::read_byte(&mut e1)?;
        let ba1 = DataInput::read_byte(&mut a1)?;
        assert_eq!(be1, ba1);

        // Now move the first one again, past the buffer length
        e1.seek(1910)?;
        a1.seek(1910)?;
        assert_eq!(1910, e1.get_file_pointer());
        assert_eq!(1910, a1.get_file_pointer());
        let be1 = DataInput::read_byte(&mut e1)?;
        let ba1 = DataInput::read_byte(&mut a1)?;
        assert_eq!(be1, ba1);

        // Now make sure the second set didn't move
        assert_eq!(1028, e2.get_file_pointer());
        assert_eq!(1028, a2.get_file_pointer());
        let be2 = DataInput::read_byte(&mut e2)?;
        let ba2 = DataInput::read_byte(&mut a2)?;
        assert_eq!(be2, ba2);

        // Move the second set back, again crossing the buffer size
        e2.seek(17)?;
        a2.seek(17)?;
        assert_eq!(17, e2.get_file_pointer());
        assert_eq!(17, a2.get_file_pointer());
        let be2 = DataInput::read_byte(&mut e2)?;
        let ba2 = DataInput::read_byte(&mut a2)?;
        assert_eq!(be2, ba2);

        // Finally, make sure the first set didn't move
        assert_eq!(1911, e1.get_file_pointer());
        assert_eq!(1911, a1.get_file_pointer());
        let be1 = DataInput::read_byte(&mut e1)?;
        let ba1 = DataInput::read_byte(&mut a1)?;
        assert_eq!(be1, ba1);
        Ok(())
    }
    /// This test opens two files from a compound stream and verifies that their file positions are
    /// independent of each other.
    fn test_random_access_clones(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let cr = create_large_cfs(random, dir.clone())?;

        // Open two files
        let mut e1 = cr.open_input("_123.f11", &new_io_context(random)?)?;
        let mut e2 = cr.open_input("_123.f3", &new_io_context(random)?)?;

        let mut a1 = e1.clone();
        let mut a2 = e2.clone();

        // Seek the first pair
        e1.seek(100)?;
        a1.seek(100)?;
        assert_eq!(100, e1.get_file_pointer());
        assert_eq!(100, a1.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e1)?,
            DataInput::read_byte(&mut a1)?
        );

        // Now seek the second pair
        e2.seek(1027)?;
        a2.seek(1027)?;
        assert_eq!(1027, e2.get_file_pointer());
        assert_eq!(1027, a2.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e2)?,
            DataInput::read_byte(&mut a2)?
        );

        // Now make sure the first one didn't move
        assert_eq!(101, e1.get_file_pointer());
        assert_eq!(101, a1.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e1)?,
            DataInput::read_byte(&mut a1)?
        );

        // Now move the first one again, past the buffer length
        e1.seek(1910)?;
        a1.seek(1910)?;
        assert_eq!(1910, e1.get_file_pointer());
        assert_eq!(1910, a1.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e1)?,
            DataInput::read_byte(&mut a1)?
        );

        // Now make sure the second set didn't move
        assert_eq!(1028, e2.get_file_pointer());
        assert_eq!(1028, a2.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e2)?,
            DataInput::read_byte(&mut a2)?
        );

        // Move the second set back, again crossing the buffer size
        e2.seek(17)?;
        a2.seek(17)?;
        assert_eq!(17, e2.get_file_pointer());
        assert_eq!(17, a2.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e2)?,
            DataInput::read_byte(&mut a2)?
        );

        // Finally, make sure the first set didn't move
        assert_eq!(1911, e1.get_file_pointer());
        assert_eq!(1911, a1.get_file_pointer());
        assert_eq!(
            DataInput::read_byte(&mut e1)?,
            DataInput::read_byte(&mut a1)?
        );

        Ok(())
    }
    fn test_file_not_found(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let cr = create_large_cfs(random, dir.clone())?;

        let result = cr.open_input("bogus", &new_io_context(random)?);
        assert!(matches!(result, Err(LuceneError::NotFound(_))));
        Ok(())
    }
    fn test_read_past_eof(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let cr = create_large_cfs(random, dir.clone())?;
        let mut is = cr.open_input("_123.f2", &new_io_context(random)?)?;
        is.seek(IndexInput::length(&is) - 10)?;
        let mut b = vec![0u8; 100];
        DataInput::read_bytes(&mut is, b.as_mut_slice(), 0, 10)?;
        let result = DataInput::read_byte(&mut is);
        assert!(matches!(result, Err(LuceneError::Eof(_))));
        is.seek(IndexInput::length(&is) - 10)?;
        let result = DataInput::read_bytes(&mut is, &mut b, 0, 50);
        assert!(matches!(result, Err(LuceneError::Eof(_))));
        Ok(())
    }
    fn test_resource_name_inside_compound_file(
        &self,
        random: &mut StdRng,
    ) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let sub_file = "_123.xyz";
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        create_sequence_file(
            random,
            dir.clone(),
            sub_file,
            0,
            10,
            si.get_id().as_slice(),
            "suffix",
        )?;
        let mut hash_set_file = HashSet::new();
        hash_set_file.insert(sub_file.to_string());
        si.set_files(hash_set_file);
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        let in_stream = cfs.open_input(sub_file, &new_io_context(random)?)?;
        let desc = in_stream.to_string();
        assert!(
            desc.contains(&format!("[slice={}]", sub_file)),
            "resource description hides that it's inside a compound file: {}",
            desc
        );
        Ok(())
    }
    fn test_missing_codec_headers_are_caught(
        &self,
        random: &mut StdRng,
    ) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let sub_file = "_123.xyz";

        // Missing codec header
        {
            let mut os = dir
                .lock()
                .unwrap()
                .create_output(sub_file, &new_io_context(random)?)?;
            for i in 0..1024 {
                os.write_byte(i as u8)?;
            }
        }

        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        let mut hash_set_file = HashSet::new();
        hash_set_file.insert(sub_file.to_string());
        si.set_files(hash_set_file);

        let result = LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT);
        assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));
        match result {
            Ok(_) => unreachable!(),
            Err(e) => {
                assert!(e.to_string().contains("codec header mismatch"));
                Ok(())
            }
        }
    }
    fn test_corrupt_files_are_caught(&self, random: &mut StdRng) -> Result<(), LuceneError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let sub_file = "_123.xyz";

        // wrong checksum
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        {
            let mut os = dir
                .lock()
                .unwrap()
                .create_output(sub_file, &new_io_context(random)?)?;
            CodecUtil::write_index_header(&mut os, "Foo", 0, &si.get_id(), "suffix")?;
            for i in 0..1024 {
                os.write_byte(i as u8)?;
            }

            // write footer with wrong checksum
            CodecUtil::write_be_int(&mut os, CodecUtil::FOOTER_MAGIC)?;
            CodecUtil::write_be_int(&mut os, 0)?;
            let checksum = os.get_checksum();
            assert!(checksum <= i64::MAX as u64);
            CodecUtil::write_be_long(&mut os, checksum as i64 + 1)?;
        }

        let mut hash_set_file = HashSet::new();
        hash_set_file.insert(sub_file.to_string());
        si.set_files(hash_set_file);

        let result = LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT);

        assert!(matches!(result, Err(LuceneError::CorruptIndex(_))));

        match result {
            Ok(_) => unreachable!(),
            Err(e) => {
                assert!(e
                    .to_string()
                    .contains("checksum failed (hardware problem?)"));
                Ok(())
            }
        }
    }
    fn test_check_integrity(&self, _random: &mut StdRng) -> Result<(), LuceneError> {
        // TODD: waiting for FileTrackingDirectoryWrapper implement
        Ok(())
    }
}

pub(crate) fn new_segment_info<D: Directory>(
    random: &mut StdRng,
    dir: Arc<Mutex<D>>,
    name: &str,
) -> Result<SegmentInfo<D>, LuceneError> {
    let min_version = if random.random_bool(0.5) {
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
pub(crate) fn create_random_file<D: Directory>(
    random: &mut StdRng,
    dir: &Arc<Mutex<D>>,
    name: &str,
    size: i32,
    seg_id: &[u8],
) -> Result<(), LuceneError> {
    let mut os = dir
        .lock()
        .unwrap()
        .create_output(name, &new_io_context(random)?)?;
    CodecUtil::write_index_header(&mut os, "Foo", 0, seg_id, "suffix")?;

    for _ in 0..size {
        let b = random.random_range(0..256) as u8;
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
) -> Result<(), LuceneError> {
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
) -> Result<(), LuceneError> {
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
) -> Result<(), LuceneError> {
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
) -> Result<(), LuceneError> {
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
/// Creates a large compound file with 20 sequential files, each of which is 1000 bytes.
fn create_large_cfs<D>(
    random: &mut StdRng,
    dir: Arc<Mutex<D>>,
) -> Result<CompoundDirectory<D>, LuceneError>
where
    D: Directory,
    D::IndexInputType: IndexInput<Slice = D::IndexInputType> + RandomAccessInput,
{
    let mut files = HashSet::new();
    let mut si = new_segment_info(random, dir.clone(), "_123")?;

    // Create 20 sequential files
    for i in 0..20 {
        let file_name = format!("_123.f{}", i);
        create_sequence_file(
            random,
            dir.clone(),
            &file_name,
            0,
            2000,
            &si.get_id(),
            "suffix",
        )?;
        files.insert(file_name);
    }
    si.set_files(files);
    LATEST_CODEC
        .compound_format()
        .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
    let cfs = LATEST_CODEC
        .compound_format()
        .get_compound_reader(dir.clone(), &si)?;
    Ok(cfs)
}
