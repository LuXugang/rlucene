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
use crate::store::directory::Directory;
use crate::store::lock::Lock;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
/// Directory implementation that delegates calls to another directory.
///
/// This struct can be used to add limitations on top of an existing
/// [`Directory`] implementation such as
/// [`NRTCachingDirectory`](crate::store::nrt_caching_directory::NRTCachingDirectory), or to add additional
/// sanity checks for tests.
///
/// However, if you plan to write your own [`Directory`] implementation,
/// you should consider extending directly [`Directory`] or
/// [`BaseDirectory`](crate::store::base_directory::BaseDirectory) rather than trying to reuse functionality of
/// existing [`Directory`]s by wrapping this one.
pub struct FilterDirectory<D>
where
    D: Directory,
{
    delegate: D,
}
impl<D> FilterDirectory<D>
where
    D: Directory,
{
    pub fn new(inner: D) -> Self {
        FilterDirectory { delegate: inner }
    }
    pub fn get_inner(&mut self) -> &mut D {
        &mut self.delegate
    }
}

impl<D> Display for FilterDirectory<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FilterDirectory({})", self.delegate)
    }
}

impl<D> Directory for FilterDirectory<D>
where
    D: Directory,
{
    fn list_all(&self) -> Result<Vec<String>> {
        self.delegate.list_all()
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        self.delegate.delete_file(name)
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        self.delegate.file_length(name)
    }

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutputType> {
        self.delegate.create_output(name, context)
    }

    type IndexOutputType = D::IndexOutputType;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
        self.delegate.create_temp_output(prefix, suffix, context)
    }

    fn sync(&mut self, names: &[&str]) -> Result<()> {
        self.delegate.sync(names)
    }

    fn sync_metadata(&mut self) -> Result<()> {
        self.delegate.sync_metadata()
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<()> {
        self.delegate.rename(source, dest)
    }

    type IndexInputType = D::IndexInputType;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInputType> {
        self.delegate.open_input(name, context)
    }

    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock> {
        self.delegate.obtain_lock(name)
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        self.delegate.get_pending_deletions()
    }
}
