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
use crate::store::dummy::dummy_index_input::DummyIndexInput;
use crate::store::dummy::dummy_index_output::DummyIndexOutput;
use crate::store::dummy::dummy_lock::DummyLock;
use crate::store::IOContext;
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub struct DummyDirectory;
impl Display for DummyDirectory {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("DummyDirectory should not be called")
    }
}

impl Directory for DummyDirectory {
    fn list_all(&self) -> Result<Vec<String>, LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }

    fn delete_file(&mut self, _name: &str) -> Result<(), LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }

    fn file_length(&self, _name: &str) -> Result<i64, LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }
    #[allow(refining_impl_trait)]
    fn create_output(
        &mut self,
        _name: &str,
        _context: &IOContext,
    ) -> Result<DummyIndexOutput, LuceneError> {
        unreachable!("DummyDirectory should not be called");
    }

    type IndexOutputType = DummyIndexOutput;
    fn create_temp_output(
        &mut self,
        _prefix: &str,
        _suffix: &str,
        _context: &IOContext,
    ) -> Result<Self::IndexOutputType, LuceneError> {
        unreachable!("DummyDirectory should not be called");
    }

    fn sync(&mut self, _names: &[&str]) -> Result<(), LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }

    fn sync_metadata(&mut self) -> Result<(), LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }

    fn rename(&mut self, _source: &str, _dest: &str) -> Result<(), LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }

    type IndexInputType = DummyIndexInput;

    fn open_input(
        &self,
        _name: &str,
        _context: &IOContext,
    ) -> Result<Self::IndexInputType, LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }
    #[allow(refining_impl_trait)]
    fn obtain_lock(&mut self, _name: &str) -> Result<DummyLock, LuceneError> {
        unreachable!("DummyDirectory should not be called");
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>, LuceneError> {
        unreachable!("DummyDirectory should not be called")
    }
}
