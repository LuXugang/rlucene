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
use std::fmt::{Display, Formatter};
use std::path::Path;

use crate::core::store::IOContext;
use crate::core::store::buffered_index_input::BufferedIndexInput;
use crate::core::store::index_input::{IndexInput, IndexInputEnum2};
use crate::core::store::memory_segment_index_input::MemorySegmentIndexInput;
use crate::core::store::mmap_directory::MMapDirectory;
use crate::core::store::nio_fs_directory::{NIOFSDirectory, NIOFSIndexInput};
use crate::core::util::error::lucene_error::Result;

pub type BuiltInFSIndexInput =
  IndexInputEnum2<MemorySegmentIndexInput, BufferedIndexInput<NIOFSIndexInput>>;

pub trait FSDirectoryBase: Display {
  type Output: IndexInput<IndexInput = Self::Output>;
  fn open_input(&self, name: &str, context: &IOContext, path: &Path) -> Result<Self::Output>;
}

pub enum FSDirectoryBaseEnum {
  NIO(NIOFSDirectory),
  MMap(MMapDirectory),
}

impl Display for FSDirectoryBaseEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      FSDirectoryBaseEnum::NIO(dir) => write!(f, "{}", dir),
      FSDirectoryBaseEnum::MMap(dir) => write!(f, "{}", dir),
    }
  }
}

impl FSDirectoryBase for FSDirectoryBaseEnum {
  type Output = BuiltInFSIndexInput;

  fn open_input(&self, name: &str, context: &IOContext, path: &Path) -> Result<Self::Output> {
    match self {
      FSDirectoryBaseEnum::MMap(dir) => {
        Ok(BuiltInFSIndexInput::A(dir.open_input(name, context, path)?))
      },
      FSDirectoryBaseEnum::NIO(dir) => {
        Ok(BuiltInFSIndexInput::B(dir.open_input(name, context, path)?))
      },
    }
  }
}
