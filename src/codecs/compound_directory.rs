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

use crate::codecs::compound_directory_enum::CompoundDirectoryEnum;
use crate::store::directory::Directory;
use crate::store::dummy::dummy_index_output::DummyIndexOutput;
use crate::store::dummy::dummy_lock::DummyLock;
use crate::store::{IOContext, IndexInput};
use crate::util::error::lucene_error::{LuceneError, Result};
/// A read-only [`Directory`] that provides a view over a compound file.
///
/// # See Also
/// - [`CompoundFormat`](crate::codecs::compound_format::CompoundFormat)
///
/// # Note
/// This API is experimental and may change in future versions.
pub struct CompoundDirectory<D>
where
    D: Directory,
{
    sub_compound_dir: CompoundDirectoryEnum<D>,
}

impl<D> CompoundDirectory<D>
where
    D: Directory,
{
    pub fn new(sub_compound_dir: CompoundDirectoryEnum<D>) -> Self {
        CompoundDirectory { sub_compound_dir }
    }
}

impl<D> Display for CompoundDirectory<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.sub_compound_dir.fmt(f)
    }
}

impl<D> Directory for CompoundDirectory<D>
where
    D: Directory,

    CompoundDirectory<D>: Display,
{
    fn list_all(&self) -> Result<Vec<String>> {
        self.sub_compound_dir.list_all()
    }

    fn delete_file(&mut self, _name: &str) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "delete_file".to_string(),
        ))
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        self.sub_compound_dir.file_length(name)
    }

    fn create_output(
        &mut self,
        _name: &str,
        _context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
        Err(LuceneError::unsupported_operation(
            "create_output".to_string(),
        ))
    }

    type IndexOutputType = DummyIndexOutput;
    fn create_temp_output(
        &mut self,
        _prefix: &str,
        _suffix: &str,
        _context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
        Err(LuceneError::unsupported_operation(
            "create_temp_output".to_string(),
        ))
    }

    fn sync(&mut self, _names: &[&str]) -> Result<()> {
        Err(LuceneError::unsupported_operation("sync".to_string()))
    }

    fn sync_metadata(&mut self) -> Result<()> {
        Ok(())
    }

    fn rename(&mut self, _source: &str, _dest: &str) -> Result<()> {
        Err(LuceneError::unsupported_operation("rename".to_string()))
    }

    type IndexInputType = <D::IndexInputType as IndexInput>::Slice;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInputType> {
        self.sub_compound_dir.open_input(name, context)
    }

    type Lock = DummyLock;

    fn obtain_lock(&mut self, _name: &str) -> Result<Self::Lock> {
        Err::<DummyLock, LuceneError>(LuceneError::unsupported_operation(
            "obtain_lock".to_string(),
        ))
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        self.sub_compound_dir.get_pending_deletions()
    }
}

pub trait CompoundDirectoryBase {
    /// Checks the consistency of this directory.
    ///
    /// # Note
    /// This operation may be costly in terms of I/O. For example, it might
    /// compute checksum values against large data files.
    fn check_integrity(&mut self) -> Result<()>;
}
