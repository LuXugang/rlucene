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
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs::File;
use crate::store::base_directory::BaseDirectory;
use crate::store::directory::Directory;
use crate::store::{IOContext, IndexOutput, NativeFSLock, OutputStreamIndexOutput};
use crate::store::fs_directory_base::FSDirectoryBase;
use crate::store::index_input::IndexInput;
use crate::store::lock::{FSLockEnum, Lock};
use crate::store::nio_fs_index_input::NIOFSIndexInput;
use crate::util::error::data_io_error_enum::DataIOError;

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

impl FSDirectoryBase for NIOFSDirectory {
    fn open_input(&self, name: &str, context: IOContext) -> Result<impl IndexInput, DataIOError> {
        let file = File::open(name)?;
        Ok(NIOFSIndexInput::new(file, name.to_string()))
    }
}
