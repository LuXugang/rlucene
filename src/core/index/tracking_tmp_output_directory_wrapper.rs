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
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub(crate) struct TrackingTmpOutputDirectoryWrapper<D> {
  pub(crate) inner: Arc<Mutex<Inner>>,
  in_: D,
  id: Identity,
}
pub(crate) struct Inner {
  pub(crate) file_names: HashMap<String, String>,
}
impl<D> TrackingTmpOutputDirectoryWrapper<D> {
  pub(crate) fn new(input: D) -> Self {
    let inner = Arc::new(Mutex::new(Inner {
      file_names: HashMap::new(),
    }));
    TrackingTmpOutputDirectoryWrapper {
      inner,
      in_: input,
      id: Identity::new(),
    }
  }
  pub(crate) fn get_temporary_files(&self) -> &Arc<Mutex<Inner>> {
    &self.inner
  }
}

impl<D> Clone for TrackingTmpOutputDirectoryWrapper<D>
where
  D: Clone,
{
  fn clone(&self) -> Self {
    Self {
      inner: Arc::clone(&self.inner),
      in_: self.in_.clone(),
      id: self.id.clone(),
    }
  }
}

impl<D> Display for TrackingTmpOutputDirectoryWrapper<D>
where
  D: Display,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}({})", std::any::type_name::<Self>(), self.in_)
  }
}

impl<D> CloseableRef for TrackingTmpOutputDirectoryWrapper<D>
where
  D: CloseableRef,
{
  fn close(&self) -> Result<()> {
    self.in_.close()
  }
}

impl<D> HasIdentity for TrackingTmpOutputDirectoryWrapper<D> {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl<D> Directory for TrackingTmpOutputDirectoryWrapper<D>
where
  D: Directory,
{
  fn list_all(&self) -> Result<Vec<String>> {
    self.in_.list_all()
  }

  fn delete_file(&self, name: &str) -> Result<()> {
    self.in_.delete_file(name)
  }

  fn file_length(&self, name: &str) -> Result<usize> {
    self.in_.file_length(name)
  }

  fn create_output(&self, name: &str, context: &IOContext) -> Result<Self::IndexOutput> {
    let output = self.in_.create_temp_output(name, "", context)?;
    self
      .inner
      .lock()
      .file_names
      .insert(name.to_string(), output.get_name().to_string());
    Ok(output)
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
    self.in_.sync(names)
  }

  fn sync_metadata(&self) -> Result<()> {
    self.in_.sync_metadata()
  }

  fn rename(&self, source: &str, dest: &str) -> Result<()> {
    self.in_.rename(source, dest)
  }

  type IndexInput = D::IndexInput;

  fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
    let inner = self.inner.lock();
    let tmp_name = inner
      .file_names
      .get(name)
      .map(|s| s.as_str())
      .unwrap_or(name);
    self.in_.open_input(tmp_name, context)
  }

  type Lock = D::Lock;

  fn obtain_lock(&self, name: &str) -> Result<Self::Lock> {
    self.in_.obtain_lock(name)
  }

  fn copy_from<T>(&self, from: &T, src: &str, dest: &str, context: &IOContext) -> Result<()>
  where
    T: Directory + ?Sized,
  {
    self.in_.copy_from(from, src, dest, context)
  }

  fn get_pending_deletions(&self) -> Result<HashSet<String>> {
    self.in_.get_pending_deletions()
  }
}
