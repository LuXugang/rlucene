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
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::store::lock::Lock;
use crate::core::util::HasIdentity;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;

/// This struct makes a best-effort check that a provided [`Lock`] is valid before any destructive filesystem operation.
pub struct LockValidatingDirectoryWrapper<D>
where
  D: Directory,
{
  in_: D,
  pub(crate) write_lock: D::Lock,
  id: Identity,
}

impl<D> LockValidatingDirectoryWrapper<D>
where
  D: Directory,
{
  pub fn new(delegate: D, write_lock: D::Lock) -> Self {
    Self {
      in_: delegate,
      write_lock,
      id: Identity::new(),
    }
  }
}

impl<D> std::fmt::Display for LockValidatingDirectoryWrapper<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}({})", std::any::type_name::<Self>(), self.in_)
  }
}

impl<D> Closeable for LockValidatingDirectoryWrapper<D>
where
  D: Directory,
{
  fn close(&mut self) -> Result<()> {
    // TODO
    Ok(())
  }
}

impl<D> HasIdentity for LockValidatingDirectoryWrapper<D>
where
  D: Directory,
{
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D> Directory for LockValidatingDirectoryWrapper<D>
where
  D: Directory,
{
  fn list_all(&self) -> Result<Vec<String>> {
    self.in_.list_all()
  }
  fn delete_file(&self, name: &str) -> Result<()> {
    self.write_lock.ensure_valid()?;
    self.in_.delete_file(name)
  }
  fn file_length(&self, name: &str) -> Result<usize> {
    self.in_.file_length(name)
  }

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    self.write_lock.ensure_valid()?;
    self.in_.create_output(name, context)
  }

  type IndexOutput = D::IndexOutput;

  fn create_temp_output(
    &self,
    prefix: &str,
    suffix: &str,
    context: &IOContext,
  ) -> Result<Self::IndexOutput> {
    self.in_.create_temp_output(prefix, suffix, context)
  }

  fn sync(&self, names: &[String]) -> Result<()> {
    self.write_lock.ensure_valid()?;
    self.in_.sync(names)
  }

  fn sync_metadata(&self) -> Result<()> {
    self.write_lock.ensure_valid()?;
    self.in_.sync_metadata()
  }

  fn rename(&self, source: &str, dest: &str) -> Result<()> {
    self.write_lock.ensure_valid()?;
    self.in_.rename(source, dest)
  }

  type IndexInput = D::IndexInput;

  fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
    self.in_.open_input(name, context)
  }
  type Lock = D::Lock;

  fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
    self.in_.obtain_lock(name)
  }

  fn copy_from(
    &self,
    from: &impl Directory,
    src: &str,
    dest: &str,
    context: &IOContext,
  ) -> Result<()> {
    self.write_lock.ensure_valid()?;
    self.in_.copy_from(from, src, dest, context)
  }

  fn delete_files_ignoring_exceptions(&self, files: &[String]) {
    self.in_.delete_files_ignoring_exceptions(files)
  }

  fn get_pending_deletions(&self) -> Result<std::collections::HashSet<String>> {
    self.in_.get_pending_deletions()
  }

  #[cfg(debug_assertions)]
  fn is_fs_directory(&self) -> bool {
    self.in_.is_fs_directory()
  }
}
