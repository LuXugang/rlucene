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
use crate::codecs::compound_directory::CompoundDirectoryBase;
use crate::codecs::lucene90::lucene90_compound_reader::Lucene90CompoundReader;
use crate::store::directory::Directory;
use crate::store::lock::Lock;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{IOContext, IndexInput, IndexOutput};
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub enum CompoundDirectoryEnum {
    Lucene90(Lucene90CompoundReader),
}

impl Display for CompoundDirectoryEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => write!(f, "{}", reader),
        }
    }
}

impl Directory for CompoundDirectoryEnum {
    fn list_all(&self) -> Result<Vec<String>, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.list_all(),
        }
    }

    fn delete_file(&mut self, name: &str) -> Result<(), LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.delete_file(name),
        }
    }

    fn file_length(&self, name: &str) -> Result<i64, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.file_length(name),
        }
    }

    fn create_output(
        &mut self,
        name: &str,
        context: &IOContext,
    ) -> Result<impl IndexOutput, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.create_output(name, context),
        }
    }

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<impl IndexOutput, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => {
                reader.create_temp_output(prefix, suffix, context)
            }
        }
    }

    fn sync(&mut self, names: &[&str]) -> Result<(), LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.sync(names),
        }
    }

    fn sync_metadata(&mut self) -> Result<(), LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.sync_metadata(),
        }
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<(), LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.rename(source, dest),
        }
    }

    fn open_input(
        &self,
        name: &str,
        context: &IOContext,
    ) -> Result<impl IndexInput + RandomAccessInput + 'static, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.open_input(name, context),
        }
    }

    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.obtain_lock(name),
        }
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>, LuceneError> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.get_pending_deletions(),
        }
    }
}
impl CompoundDirectoryBase for CompoundDirectoryEnum {
    fn check_integrity(&self) {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }
}
