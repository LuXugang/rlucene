/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::directory::Directory;
use crate::store::filter_directory::FilterDirectory;
use crate::store::IOContext;
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
    base: FilterDirectory<D, Arc<Mutex<D>>>,
    lock: Mutex<()>,
}
impl<D> TrackingDirectoryWrapper<D>
where
    D: Directory,
{
    pub fn new(input: Arc<Mutex<D>>) -> Self {
        TrackingDirectoryWrapper {
            created_filenames: HashSet::new(),
            base: FilterDirectory::new(input),
            lock: Mutex::new(()),
        }
    }
    pub fn get_created_files(&self) -> &HashSet<String> {
        &self.created_filenames
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

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutputType> {
        let output = self.base.delegate.lock().create_output(name, context)?;
        self.created_filenames.insert(name.to_string());
        Ok(output)
    }

    type IndexOutputType = <FilterDirectory<D, Arc<Mutex<D>>> as Directory>::IndexOutputType;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
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

    type IndexInputType = <FilterDirectory<D, Arc<Mutex<D>>> as Directory>::IndexInputType;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInputType> {
        self.base.open_input(name, context)
    }

    fn open_checksum_input(
        &self,
        name: &str,
    ) -> Result<BufferedChecksumIndexInput<Self::IndexInputType>> {
        self.base.open_checksum_input(name)
    }

    type Lock = <FilterDirectory<D, Arc<Mutex<D>>> as Directory>::Lock;

    fn obtain_lock(&mut self, name: &str) -> Result<Self::Lock> {
        self.base.obtain_lock(name)
    }

    fn copy_from<T: Directory>(
        &mut self,
        from: Arc<Mutex<T>>,
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
