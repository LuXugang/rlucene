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
use rlucene::store::DataOutput;
use rlucene::store::DataInput;
use crate::util::test_error::TestError;
use rlucene::store::directory::Directory;
use std::path::{Path, PathBuf};
use rand::Rng;
use rand::rngs::StdRng;
use tempfile::{Builder, TempDir};
use crate::util::lucene_test_case::{new_directory, new_io_context, slow_file_exists};

pub trait BaseDirectoryTestCase {
    fn get_directory(&self, path: PathBuf) -> Result<impl Directory,TestError>;

    fn test_copy_from(&self, random: &mut StdRng) -> Result<(), TestError> {
        let mut temp_dir =Builder::new()
            .prefix("testCopy")
            .tempdir()?; 
        let mut source = self.get_directory(temp_dir.into_path())?;
        let mut dest = new_directory(random)?;
        Self::run_copy_from(&mut source, &mut dest, random)?;
        
        let mut source = new_directory(random)?;
        temp_dir =Builder::new()
            .prefix("testCopyDestination")
            .tempdir()?; 
        let mut dest = self.get_directory(temp_dir.into_path())?;
        Self::run_copy_from(&mut source, &mut dest, random)?;
        Ok(())
    }

    fn run_copy_from(
        source: &mut impl Directory,
        dest: &mut impl Directory,
        random: &mut StdRng,
    ) -> Result<(), TestError> {
        let mut bytes = vec![0u8; 20000];
        random.fill(&mut bytes[..]);
        {
            let mut output = source
                .create_output("foobar", new_io_context(random)?)?;

            output.write_bytes_with_len(&bytes, bytes.len() as u32)?;
        }
        dest.copy_from(source, "foobar", "foobaz", new_io_context(random)?)?;
        assert!(slow_file_exists(dest, "foobaz")?);
        let bytes2_len = bytes.len();
        let mut bytes2 = vec![0u8; bytes2_len];
        {
            let mut input = dest.open_input("foobaz", new_io_context(random)?)?;
            input.read_bytes(&mut bytes2, 0, bytes2_len as u32)?;
        }

        // Ensure that the original and copied data match
        assert_eq!(bytes, bytes2);

        Ok(())
    }

}

