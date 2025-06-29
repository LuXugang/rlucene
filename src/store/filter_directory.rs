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
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::access::Access;
use crate::util::error::lucene_error::Result;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

/// Directory implementation that delegates calls to another directory.
///
/// This struct can be used to add limitations on top of an existing
/// [`Directory`] implementation such as
/// [`NRTCachingDirectory`](crate::store::nrt_caching_directory::NRTCachingDirectory), or to add additional
/// sanity checks for tests.
///
/// However, if you plan to write your own [`Directory`] implementation,
/// you should consider extending directly [`Directory`] or
/// [`BaseDirectory`](crate::store::base_directory::BaseDirectory) rather than
/// trying to reuse functionality of existing [`Directory`]s by wrapping this
/// one.
pub struct FilterDirectory<D, A>
where
    D: Directory,
    A: Access<D>,
{
    pub(crate) delegate: A,
    phantom: PhantomData<D>,
}
impl<D, A> FilterDirectory<D, A>
where
    D: Directory,
    A: Access<D>,
{
    pub fn new(inner: A) -> Self {
        FilterDirectory {
            delegate: inner,
            phantom: Default::default(),
        }
    }
    pub fn get_inner(&mut self) -> &mut A {
        &mut self.delegate
    }
}

impl<D, A> Display for FilterDirectory<D, A>
where
    D: Directory,
    A: Access<D>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FilterDirectory({})",
            self.delegate.access(|dir| dir.to_string())
        )
    }
}

impl<D, A> Directory for FilterDirectory<D, A>
where
    D: Directory,
    A: Access<D>,
{
    fn list_all(&self) -> Result<Vec<String>> {
        self.delegate.access(|dir| dir.list_all())
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        self.delegate.access_mut(|dir| dir.delete_file(name))
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        self.delegate.access(|dir| dir.file_length(name))
    }

    fn create_output(&mut self, name: &str, context: &IOContext) -> Result<Self::IndexOutputType> {
        self.delegate
            .access_mut(|dir| dir.create_output(name, context))
    }

    type IndexOutputType = D::IndexOutputType;

    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        context: &IOContext,
    ) -> Result<Self::IndexOutputType> {
        self.delegate
            .access_mut(|dir| dir.create_temp_output(prefix, suffix, context))
    }

    fn sync(&mut self, names: &[&str]) -> Result<()> {
        self.delegate.access_mut(|dir| dir.sync(names))
    }

    fn sync_metadata(&mut self) -> Result<()> {
        self.delegate.access_mut(|dir| dir.sync_metadata())
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<()> {
        self.delegate.access_mut(|dir| dir.rename(source, dest))
    }

    type IndexInputType = D::IndexInputType;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInputType> {
        self.delegate
            .access_mut(|dir| dir.open_input(name, context))
    }

    type Lock = D::Lock;

    fn obtain_lock(&mut self, name: &str) -> Result<Self::Lock> {
        self.delegate.access_mut(|dir| dir.obtain_lock(name))
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        self.delegate.access_mut(|dir| dir.get_pending_deletions())
    }
}
