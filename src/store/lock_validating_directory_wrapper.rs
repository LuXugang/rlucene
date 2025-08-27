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
use crate::store::IOContext;
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::directory::Directory;
use crate::store::filter_directory::FilterDirectory;
use crate::store::lock::Lock;
use crate::util::error::lucene_error::Result;
use std::sync::Arc;
/// This class makes a best-effort check that a provided [`Lock`] is valid before any destructive filesystem operation.
pub struct LockValidatingDirectoryWrapper<D>
where
    D: Directory,
{
    base: FilterDirectory<D, Arc<D>>,
    write_lock: D::Lock,
}

impl<D> LockValidatingDirectoryWrapper<D>
where
    D: Directory,
{
    pub fn new(delegate: Arc<D>, write_lock: D::Lock) -> Self {
        Self {
            base: FilterDirectory::new(delegate),
            write_lock,
        }
    }
}

impl<D> std::fmt::Display for LockValidatingDirectoryWrapper<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", std::any::type_name::<Self>(), self.base)
    }
}

impl<D> Directory for LockValidatingDirectoryWrapper<D>
where
    D: Directory,
{
    type IndexOutput = <FilterDirectory<D, Arc<D>> as Directory>::IndexOutput;
    type IndexInput = <FilterDirectory<D, Arc<D>> as Directory>::IndexInput;
    type Lock = <FilterDirectory<D, Arc<D>> as Directory>::Lock;

    fn list_all(&self) -> Result<Vec<String>> {
        self.base.list_all()
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        self.base.file_length(name)
    }

    fn delete_file(&self, name: &str) -> Result<()> {
        self.write_lock.ensure_valid()?;
        self.base.delegate.delete_file(name)
    }

    fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
        self.write_lock.ensure_valid()?;
        self.base.delegate.create_output(name, context)
    }

    fn create_temp_output(
        &self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutput> {
        self.base
            .delegate
            .create_temp_output(prefix, suffix, context)
    }

    fn sync_metadata(&self) -> Result<()> {
        self.write_lock.ensure_valid()?;
        self.base.delegate.sync_metadata()
    }

    fn rename(&self, source: &str, dest: &str) -> Result<()> {
        self.write_lock.ensure_valid()?;
        self.base.delegate.rename(source, dest)
    }

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
        self.base.open_input(name, context)
    }

    fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
        self.base.obtain_lock(name)
    }

    fn copy_from(
        &self,
        from: &impl Directory,
        src: &str,
        dest: &str,
        context: &IOContext,
    ) -> Result<()> {
        self.write_lock.ensure_valid()?;
        self.base.delegate.copy_from(from, src, dest, context)
    }

    fn delete_files_ignoring_exceptions(&self, files: &[String]) {
        self.base.delete_files_ignoring_exceptions(files)
    }

    fn get_pending_deletions(&self) -> Result<std::collections::HashSet<String>> {
        self.base.get_pending_deletions()
    }

    fn is_fs_directory(&self) -> bool {
        self.base.is_fs_directory()
    }

    fn open_checksum_input(
        &self,
        name: &str,
    ) -> Result<BufferedChecksumIndexInput<Self::IndexInput>> {
        self.base.open_checksum_input(name)
    }

    fn sync<'a, T>(&self, names: T) -> Result<()>
    where
        T: IntoIterator<Item = &'a String>,
    {
        self.write_lock.ensure_valid()?;
        self.base.delegate.sync(names)
    }
}
