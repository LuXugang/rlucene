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
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use crate::store::directory::Directory;
use crate::store::dummy::dummy_index_input::DummyIndexInput;
use crate::store::dummy::dummy_index_output::DummyIndexOutput;
use crate::store::dummy::dummy_lock::DummyLock;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;

pub struct DummyDirectory;
impl Display for DummyDirectory {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl Directory for DummyDirectory {
    fn list_all(&self) -> Result<Vec<String>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn delete_file(&mut self, _name: &str) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn file_length(&self, _name: &str) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
    fn create_output(&mut self, _name: &str, _context: &IOContext) -> Result<DummyIndexOutput> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type IndexOutputType = DummyIndexOutput;
    fn create_temp_output(
        &mut self,
        _prefix: &str,
        _suffix: &str,
        _context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn sync(&mut self, _names: &[&str]) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn sync_metadata(&mut self) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn rename(&mut self, _source: &str, _dest: &str) -> Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type IndexInputType = DummyIndexInput;

    fn open_input(&self, _name: &str, _context: &IOContext) -> Result<Self::IndexInputType> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Lock = DummyLock;

    fn obtain_lock(&mut self, _name: &str) -> Result<Self::Lock> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
