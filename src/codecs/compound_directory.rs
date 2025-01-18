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
use crate::codecs::compound_directory_enum::CompoundDirectoryEnum;
use crate::store::directory::Directory;
use crate::store::dummy::dummy_index_output::DummyIndexOutput;
use crate::store::dummy::dummy_lock::DummyLock;
use crate::store::lock::Lock;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{IOContext, IndexInput, IndexOutput};
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
/// A read-only [`Directory`] that provides a view over a compound file.
///
/// # See Also
/// - [`CompoundFormat`](crate::codecs::compound_format::CompoundFormat)
///
/// # Note
/// This API is experimental and may change in future versions.
pub struct CompoundDirectory<D: Directory, I: IndexInput<Slice = I> + RandomAccessInput> {
    sub_compound_dir: CompoundDirectoryEnum<D, I>,
}

impl<D, I> CompoundDirectory<D, I>
where
    D: Directory,
    I: IndexInput<Slice = I> + RandomAccessInput,
{
    pub fn new(sub_compound_dir: CompoundDirectoryEnum<D, I>) -> Self {
        CompoundDirectory { sub_compound_dir }
    }
}

impl<D, I> Display for CompoundDirectory<D, I>
where
    D: Directory,
    I: IndexInput<Slice = I> + RandomAccessInput,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.sub_compound_dir.fmt(f)
    }
}

impl<D, I> Directory for CompoundDirectory<D, I>
where
    D: Directory<Output = I>,
    I: IndexInput<Slice = I> + RandomAccessInput,
{
    fn list_all(&self) -> Result<Vec<String>, LuceneError> {
        self.sub_compound_dir.list_all()
    }

    fn delete_file(&mut self, _name: &str) -> Result<(), LuceneError> {
        Err(LuceneError::unsupported_operation(
            "delete_file".to_string(),
        ))
    }

    fn file_length(&self, name: &str) -> Result<i64, LuceneError> {
        self.sub_compound_dir.file_length(name)
    }

    fn create_output(
        &mut self,
        _name: &str,
        _context: &IOContext,
    ) -> Result<impl IndexOutput, LuceneError> {
        Err::<DummyIndexOutput, LuceneError>(LuceneError::unsupported_operation(
            "create_output".to_string(),
        ))
    }

    fn create_temp_output(
        &mut self,
        _prefix: &str,
        _suffix: &str,
        _context: &IOContext,
    ) -> Result<impl IndexOutput, LuceneError> {
        Err::<DummyIndexOutput, LuceneError>(LuceneError::unsupported_operation(
            "create_temp_output".to_string(),
        ))
    }

    fn sync(&mut self, _names: &[&str]) -> Result<(), LuceneError> {
        Err(LuceneError::unsupported_operation("sync".to_string()))
    }

    fn sync_metadata(&mut self) -> Result<(), LuceneError> {
        Ok(())
    }

    fn rename(&mut self, _source: &str, _dest: &str) -> Result<(), LuceneError> {
        Err(LuceneError::unsupported_operation("rename".to_string()))
    }

    type Output = D::Output;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::Output, LuceneError> {
        self.sub_compound_dir.open_input(name, context)
    }

    fn obtain_lock(&mut self, _name: &str) -> Result<impl Lock, LuceneError> {
        Err::<DummyLock, LuceneError>(LuceneError::unsupported_operation(
            "obtain_lock".to_string(),
        ))
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>, LuceneError> {
        self.sub_compound_dir.get_pending_deletions()
    }
}

pub trait CompoundDirectoryBase {
    /// Checks the consistency of this directory.
    ///
    /// # Note
    /// This operation may be costly in terms of I/O. For example, it might compute checksum values
    /// against large data files.
    fn check_integrity(&mut self) -> Result<(), LuceneError>;
}
