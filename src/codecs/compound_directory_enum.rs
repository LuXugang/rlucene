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

use crate::codecs::compound_directory::CompoundDirectoryBase;
use crate::codecs::lucene90::lucene90_compound_reader::Lucene90CompoundReader;
use crate::store::directory::Directory;
use crate::store::{IOContext, IndexInput};
use crate::util::error::lucene_error::Result;

pub enum CompoundDirectoryEnum<D>
where
    D: Directory,
{
    Lucene90(Lucene90CompoundReader<D>),
}

impl<D> Display for CompoundDirectoryEnum<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => write!(f, "{reader}"),
        }
    }
}

impl<D> Directory for CompoundDirectoryEnum<D>
where
    D: Directory,

    CompoundDirectoryEnum<D>: Display,
{
    fn list_all(&self) -> Result<Vec<String>> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.list_all(),
        }
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.delete_file(name),
        }
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.file_length(name),
        }
    }

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutputType> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.create_output(name, context),
        }
    }

    type IndexOutputType = D::IndexOutputType;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => {
                reader.create_temp_output(prefix, suffix, context)
            },
        }
    }

    fn sync(&mut self, names: &[&str]) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.sync(names),
        }
    }

    fn sync_metadata(&mut self) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.sync_metadata(),
        }
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.rename(source, dest),
        }
    }

    type IndexInputType = <D::IndexInputType as IndexInput>::Slice;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInputType> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.open_input(name, context),
        }
    }

    type Lock = D::Lock;

    fn obtain_lock(&mut self, name: &str) -> Result<Self::Lock> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.obtain_lock(name),
        }
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.get_pending_deletions(),
        }
    }
}
impl<D> CompoundDirectoryBase for CompoundDirectoryEnum<D>
where
    D: Directory,
{
    fn check_integrity(&mut self) -> Result<()> {
        match self {
            CompoundDirectoryEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }
}
