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
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// A delegating Directory that records which files were written to and deleted.
pub struct TrackingDirectoryWrapper<D>
where
    D: Directory,
{
    created_filenames: HashSet<String>,
    pub(crate) base: FilterDirectory<D, Arc<Mutex<D>>>,
    lock: Mutex<()>,
    #[cfg(debug_assertions)]
    taken: bool,
}
impl<D> TrackingDirectoryWrapper<D>
where
    D: Directory,
{
    #[cfg(not(debug_assertions))]
    pub fn new(input: Arc<Mutex<D>>) -> Self {
        TrackingDirectoryWrapper {
            created_filenames: HashSet::new(),
            base: FilterDirectory::new(input),
            lock: Mutex::new(()),
        }
    }
    #[cfg(debug_assertions)]
    pub fn new(input: Arc<Mutex<D>>) -> Self {
        TrackingDirectoryWrapper {
            created_filenames: HashSet::new(),
            base: FilterDirectory::new(input),
            lock: Mutex::new(()),
            taken: false,
        }
    }

    pub fn get_created_files(&self) -> &HashSet<String> {
        &self.created_filenames
    }
    pub fn take_created_files(&mut self) -> HashSet<String> {
        #[cfg(debug_assertions)]
        if !self.taken {
            self.taken = true;
        } else {
            debug_assert!(
                false,
                "TrackingDirectoryWrapper::take_created_files called multiple times"
            );
        }
        std::mem::take(&mut self.created_filenames)
    }
    pub fn clear_created_files(&mut self) {
        self.created_filenames.clear();
    }
}

impl<D> Display for TrackingDirectoryWrapper<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TrackingDirectoryWrapper({})", self.base)
    }
}

impl<D> Directory for TrackingDirectoryWrapper<D>
where
    D: Directory,
{
    fn list_all(&self) -> Result<Vec<String>> {
        self.base.list_all()
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        self.base.delegate.lock().delete_file(name)?;
        self.created_filenames.remove(name);
        Ok(())
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        self.base.file_length(name)
    }

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
        let output = self.base.delegate.lock().create_output(name, context)?;
        self.created_filenames.insert(name.to_string());
        Ok(output)
    }

    type IndexOutput = <FilterDirectory<D, Arc<Mutex<D>>> as Directory>::IndexOutput;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutput> {
        self.base.create_temp_output(prefix, suffix, context)
    }

    fn sync(&mut self, names: &[&str]) -> Result<()> {
        self.base.sync(names)
    }

    fn sync_metadata(&mut self) -> Result<()> {
        self.base.sync_metadata()
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<()> {
        self.base.delegate.lock().rename(source, dest)?;
        let _guide = self.lock.lock();
        self.created_filenames.insert(dest.to_string());
        self.created_filenames.remove(&source.to_string());
        Ok(())
    }

    type IndexInput = <FilterDirectory<D, Arc<Mutex<D>>> as Directory>::IndexInput;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
        self.base.open_input(name, context)
    }

    fn open_checksum_input(
        &self,
        name: &str,
    ) -> Result<BufferedChecksumIndexInput<Self::IndexInput>> {
        self.base.open_checksum_input(name)
    }

    type Lock = <FilterDirectory<D, Arc<Mutex<D>>> as Directory>::Lock;

    fn obtain_lock(&mut self, name: &str) -> Result<Self::Lock> {
        self.base.obtain_lock(name)
    }

    fn copy_from(
        &mut self,
        from: &mut impl Directory,
        src: &str,
        dest: &str,
        context: &IOContext,
    ) -> Result<()> {
        self.base
            .delegate
            .lock()
            .copy_from(from, src, dest, context)?;
        self.created_filenames.insert(src.to_string());
        Ok(())
    }

    fn delete_files_ignoring_exceptions(&mut self, files: &[String]) {
        self.base.delete_files_ignoring_exceptions(files);
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        self.base.get_pending_deletions()
    }

    fn is_fs_directory(&self) -> bool {
        self.base.is_fs_directory()
    }
}
