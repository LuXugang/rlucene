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
use crate::store::directory::Directory;
use crate::store::dummy::dummy_index_input::DummyIndexInput;
use crate::store::dummy::dummy_index_output::DummyIndexOutput;
use crate::store::dummy::dummy_lock::DummyLock;
use crate::store::lock::Lock;
use crate::store::{IOContext, IndexOutput};
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

pub struct Lucene90CompoundReader;

impl Display for Lucene90CompoundReader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Directory for Lucene90CompoundReader {
    fn list_all(&self) -> Result<Vec<String>, LuceneError> {
        todo!()
    }

    fn delete_file(&mut self, _name: &str) -> Result<(), LuceneError> {
        Err(LuceneError::illegal_state(
            "delete_file() wrapped by CompoundDirectory, this method should never not be called"
                .to_string(),
        ))
    }

    fn file_length(&self, name: &str) -> Result<i64, LuceneError> {
        todo!()
    }

    fn create_output(
        &mut self,
        _name: &str,
        _context: &IOContext,
    ) -> Result<impl IndexOutput, LuceneError> {
        Err::<DummyIndexOutput, LuceneError>(LuceneError::illegal_state(
            "create_output() wrapped by CompoundDirectory, this method should never not be called"
                .to_string(),
        ))
    }

    fn create_temp_output(
        &mut self,
        _prefix: &str,
        _suffix: &str,
        _context: &IOContext,
    ) -> Result<impl IndexOutput, LuceneError> {
        Err::<DummyIndexOutput, LuceneError>(LuceneError::illegal_state(
            "create_temp_output() wrapped by CompoundDirectory, this method should never not be called".to_string(),
        ))
    }

    fn sync(&mut self, _names: &[&str]) -> Result<(), LuceneError> {
        Err(LuceneError::illegal_state(
            "sync() wrapped by CompoundDirectory, this method should never not be called"
                .to_string(),
        ))
    }

    fn sync_metadata(&mut self) -> Result<(), LuceneError> {
        Err(LuceneError::illegal_state(
            "sync_metadata() wrapped by CompoundDirectory, this method should never not be called"
                .to_string(),
        ))
    }

    fn rename(&mut self, _source: &str, _dest: &str) -> Result<(), LuceneError> {
        Err(LuceneError::illegal_state(
            "rename() wrapped by CompoundDirectory, this method should never not be called"
                .to_string(),
        ))
    }

    type Output = DummyIndexInput;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::Output, LuceneError> {
        todo!()
    }

    fn obtain_lock(&mut self, _name: &str) -> Result<impl Lock, LuceneError> {
        Err::<DummyLock, LuceneError>(LuceneError::illegal_state(
            "obtain_lock() wrapped by CompoundDirectory, this method should never not be called"
                .to_string(),
        ))
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>, LuceneError> {
        todo!()
    }
}
impl CompoundDirectoryBase for Lucene90CompoundReader {
    fn check_integrity(&self) {
        todo!()
    }
}
