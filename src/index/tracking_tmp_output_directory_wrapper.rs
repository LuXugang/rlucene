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
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::directory::Directory;
use crate::store::filter_directory::FilterDirectory;
use crate::store::{IOContext, IndexOutput};
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct TrackingTmpOutputDirectoryWrapper<D>
where
    D: Directory,
{
    file_names: HashMap<String, String>,
    base: FilterDirectory<D, Arc<Mutex<D>>>,
}
impl<D> TrackingTmpOutputDirectoryWrapper<D>
where
    D: Directory,
{
    pub fn new(input: Arc<Mutex<D>>) -> Self {
        TrackingTmpOutputDirectoryWrapper {
            file_names: HashMap::new(),
            base: FilterDirectory::new(input),
        }
    }
    pub fn get_temporary_files(&mut self) -> HashMap<String, String> {
        std::mem::take(&mut self.file_names)
    }
}

impl<D> Display for TrackingTmpOutputDirectoryWrapper<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", std::any::type_name::<Self>(), self.base)
    }
}

impl<D> Directory for TrackingTmpOutputDirectoryWrapper<D>
where
    D: Directory,
{
    fn list_all(&self) -> Result<Vec<String>> {
        self.base.list_all()
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        self.base.delete_file(name)
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        self.base.file_length(name)
    }

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
        let output = self.base.create_temp_output(name, "", context)?;
        self.file_names
            .insert(name.to_string(), output.get_name().to_string());
        Ok(output)
    }

    type IndexOutput = D::IndexOutput;

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
        self.base.rename(source, dest)
    }

    type IndexInput = D::IndexInput;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
        let tmp_name = self
            .file_names
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(name);
        self.base.open_input(tmp_name, context)
    }

    fn open_checksum_input(
        &self,
        name: &str,
    ) -> Result<BufferedChecksumIndexInput<Self::IndexInput>> {
        self.base.open_checksum_input(name)
    }

    type Lock = D::Lock;

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
        self.base.copy_from(from, src, dest, context)
    }

    fn delete_files_ignoring_exceptions(&mut self, files: &[String]) {
        self.base.delete_files_ignoring_exceptions(files)
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        self.base.get_pending_deletions()
    }
}
