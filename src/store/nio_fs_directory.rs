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
use crate::store::fs_directory_base::FSDirectoryBase;
use crate::store::nio_fs_index_input::NIOFSIndexInput;
use crate::store::{BufferedIndexInput, IOContext};
use crate::util::error::data_io_error_enum::DataIOError;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::{Path, PathBuf};

/// An implementation of [`FSDirectory`](crate::store::fs_directory::FSDirectory)that uses `std::fs::File` for positional reads,
/// allowing multiple threads to read from the same file without synchronization.
///
/// # Read and Write Modes
///
/// This class uses `std::fs::File` for reading, enabling thread-safe concurrent reads. Writing
/// is achieved using [`OutputStreamIndexOutput`](crate::store::output_stream_index_output).

pub struct NIOFSDirectory;

impl Default for NIOFSDirectory {
    fn default() -> Self {
        Self::new()
    }
}

impl NIOFSDirectory {
    pub fn new() -> Self {
        Self
    }
}

impl Display for NIOFSDirectory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

/// this method should only be called in [`FSDirectory::open_input`], which will first check whether file could be read
impl FSDirectoryBase for NIOFSDirectory {
    fn open_input(
        &self,
        name: &str,
        context: IOContext,
        path: &PathBuf,
    ) -> Result<BufferedIndexInput<NIOFSIndexInput>, DataIOError> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => {
                return Err(DataIOError::io_with_path(name,err));
            }
        };
        let resource_desc = format!("NIOFSIndexInput(path=\"{}\")", path.display());
        // let resource_desc_string = resource_desc.to_string();
        let index_input = NIOFSIndexInput::new(file, &resource_desc);
        Ok(BufferedIndexInput::new_with_io_context(
            index_input,
            &resource_desc,
            context,
        ))
    }
}
