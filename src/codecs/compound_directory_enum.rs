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

use crate::codecs::compound_directory::CompoundDirectoryBase;
use crate::codecs::lucene90::lucene90_compound_reader::Lucene90CompoundReader;
use crate::store::directory::Directory;
use crate::store::{IOContext, IndexInput};
use crate::util::error::lucene_error::Result;

pub enum CompoundDirectoryEnum<D>
where
    D: Directory,
{
    Lucene90(Lucene90CompoundReader<D>),
}

impl<D> Display for CompoundDirectoryEnum<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => write!(f, "{reader}"),
        }
    }
}

impl<D> Directory for CompoundDirectoryEnum<D>
where
    D: Directory,
    CompoundDirectoryEnum<D>: Display,
{
    fn list_all(&self) -> Result<Vec<String>> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.list_all(),
        }
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.delete_file(name),
        }
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.file_length(name),
        }
    }

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.create_output(name, context),
        }
    }

    type IndexOutput = D::IndexOutput;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutput> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => {
                reader.create_temp_output(prefix, suffix, context)
            },
        }
    }

    fn sync(&mut self, names: &[&str]) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.sync(names),
        }
    }

    fn sync_metadata(&mut self) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.sync_metadata(),
        }
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.rename(source, dest),
        }
    }

    type IndexInput = <D::IndexInput as IndexInput>::Slice;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.open_input(name, context),
        }
    }

    type Lock = D::Lock;

    fn obtain_lock(&mut self, name: &str) -> Result<Self::Lock> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.obtain_lock(name),
        }
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.get_pending_deletions(),
        }
    }
}
impl<D> CompoundDirectoryBase for CompoundDirectoryEnum<D>
where
    D: Directory,
{
    fn check_integrity(&mut self) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }
}
