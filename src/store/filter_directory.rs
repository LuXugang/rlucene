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
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

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
    fn list_all(&self) -> Result<Vec<String>, LuceneError> {
        self.delegate.list_all()
    }

    fn delete_file(&mut self, name: &str) -> Result<(), LuceneError> {
        self.delegate.delete_file(name)
    }

    fn file_length(&self, name: &str) -> Result<i64, LuceneError> {
        self.delegate.file_length(name)
    }

    fn create_output(
        &mut self,
        name: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutputType, LuceneError> {
        self.delegate.create_output(name, context)
    }

    type IndexOutputType = D::IndexOutputType;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutputType, LuceneError> {
        self.delegate.create_temp_output(prefix, suffix, context)
    }

    fn sync(&mut self, names: &[&str]) -> Result<(), LuceneError> {
        self.delegate.sync(names)
    }

    fn sync_metadata(&mut self) -> Result<(), LuceneError> {
        self.delegate.sync_metadata()
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<(), LuceneError> {
        self.delegate.rename(source, dest)
    }

    type IndexInputType = D::IndexInputType;

    fn open_input(
        &self,
        name: &str,
        context: &IOContext,
    ) -> Result<Self::IndexInputType, LuceneError> {
        self.delegate.open_input(name, context)
    }

    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock, LuceneError> {
        self.delegate.obtain_lock(name)
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>, LuceneError> {
        self.delegate.get_pending_deletions()
    }
}
