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
use crate::index::index_commit::IndexCommit;
use crate::index::standard_directory_reader::StandardDirectoryReader;
use crate::store::directory::Directory;
use crate::store::dummy::dummy_directory::DummyDirectory;
use crate::util::error::lucene_error;
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct DummyIndexCommit<D>
where
    D: Directory,
{
    dir: Arc<Mutex<D>>,
}
impl<D> DummyIndexCommit<D>
where
    D: Directory,
{
    pub fn new(dir: Arc<Mutex<D>>) -> Self {
        DummyIndexCommit { dir }
    }
}

impl<D> PartialEq for DummyIndexCommit<D>
where
    D: Directory,
{
    fn eq(&self, _other: &Self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<D> Eq for DummyIndexCommit<D> where D: Directory {}

impl<D> PartialOrd for DummyIndexCommit<D>
where
    D: Directory,
{
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<D> Ord for DummyIndexCommit<D>
where
    D: Directory,
{
    fn cmp(&self, _other: &Self) -> Ordering {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<D> Display for DummyIndexCommit<D>
where
    D: Directory,
{
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl<D> IndexCommit for DummyIndexCommit<D>
where
    D: Directory,
{
    fn get_segments_file_name(&self) -> &str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn delete(&mut self) -> lucene_error::Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn is_deleted(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_segment_count(&self) -> usize {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_generation(&self) -> i64 {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn user_data(&self) -> &HashMap<String, String> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_file_names(&self) -> lucene_error::Result<&[String]> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_reader(&self) -> Option<StandardDirectoryReader> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type Directory = DummyDirectory;

    fn get_directory(&self) -> Arc<Mutex<Self::Directory>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
