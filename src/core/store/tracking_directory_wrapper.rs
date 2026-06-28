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
use crate::core::index::index_reader::Identity;
use crate::core::store::directory::Directory;
use crate::core::store::{IOContext, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

/// A delegating Directory that records which files were written to and deleted.
pub struct TrackingDirectoryWrapper<D>
where
  D: Directory,
{
  pub(crate) in_: D,
  inner: Mutex<Inner>,
  id: Identity,
}
pub struct Inner {
  pub(crate) created_filenames: HashSet<String>,
}
impl<D> TrackingDirectoryWrapper<D>
where
  D: Directory,
{
  pub fn new(input: D) -> Self {
    let lock = Mutex::new(Inner {
      created_filenames: HashSet::new(),
    });
    TrackingDirectoryWrapper {
      in_: input,
      inner: lock,
      id: Identity::new(),
    }
  }

  pub fn get_created_files(&self) -> &Mutex<Inner> {
    &self.inner
  }
  pub fn take_created_files(&mut self) -> HashSet<String> {
    std::mem::take(&mut self.inner.lock().created_filenames)
  }

  pub fn clear_created_files(&mut self) {
    self.inner.lock().created_filenames.clear();
  }
}

impl<D> Display for TrackingDirectoryWrapper<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "TrackingDirectoryWrapper({})", self.in_)
  }
}

impl<D> Closeable for TrackingDirectoryWrapper<D>
where
  D: Directory,
{
  fn close(&mut self) -> Result<()> {
    // TODO
    Ok(())
  }
}

impl<D> HasIdentity for TrackingDirectoryWrapper<D>
where
  D: Directory,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D> Directory for TrackingDirectoryWrapper<D>
where
  D: Directory,
{
  fn list_all(&self) -> Result<Vec<String>> {
    self.in_.list_all()
  }

  fn delete_file(&self, name: &str) -> Result<()> {
    self.in_.delete_file(name)?;
    self.inner.lock().created_filenames.remove(name);
    Ok(())
  }

  fn file_length(&self, name: &str) -> Result<usize> {
    self.in_.file_length(name)
  }

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    let output = self.in_.create_output(name, context)?;
    self.inner.lock().created_filenames.insert(name.to_string());
    Ok(output)
  }

  type IndexOutput = D::IndexOutput;

  fn create_temp_output(
    &self,
    prefix: &str,
    suffix: &str,
    context: &IOContext,
  ) -> Result<Self::IndexOutput> {
    let temp = self.in_.create_temp_output(prefix, suffix, context)?;
    let name = temp.get_name().to_string();
    self.inner.lock().created_filenames.insert(name);
    Ok(temp)
  }

  fn sync(&self, names: &[String]) -> Result<()> {
    self.in_.sync(names)
  }

  fn sync_metadata(&self) -> Result<()> {
    self.in_.sync_metadata()
  }

  fn rename(&self, source: &str, dest: &str) -> Result<()> {
    self.in_.rename(source, dest)?;
    let mut inner = self.inner.lock();
    inner.created_filenames.insert(dest.to_string());
    inner.created_filenames.remove(source);
    drop(inner);
    Ok(())
  }

  type IndexInput = D::IndexInput;

  fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
    self.in_.open_input(name, context)
  }

  type Lock = D::Lock;

  fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
    self.in_.obtain_lock(name)
  }

  fn copy_from<T>(&self, from: &T, src: &str, dest: &str, context: &IOContext) -> Result<()>
  where
    T: Directory + ?Sized,
  {
    self.in_.copy_from(from, src, dest, context)?;
    self.inner.lock().created_filenames.insert(dest.to_string());
    Ok(())
  }

  fn get_pending_deletions(&self) -> Result<HashSet<String>> {
    self.in_.get_pending_deletions()
  }

  #[cfg(debug_assertions)]
  fn is_fs_directory(&self) -> bool {
    self.in_.is_fs_directory()
  }

  fn ensure_open(&self) -> Result<()> {
    self.in_.ensure_open()
  }
}
