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
use crate::core::index::directory_reader::DirectoryReader;
use crate::core::index::index_reader::{CacheHelper, CacheKey, ClosedListener};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::Result;

pub trait FilterDirectoryReader: DirectoryReader {
  type Delegate: DirectoryReader;
  fn get_delegate(&self) -> &Self::Delegate;
  type WrapDirectoryReader: DirectoryReader;
  fn do_wrap_directory_reader(
    &self,
    in_: Option<<Self::Delegate as DirectoryReader>::DirectoryReader>,
  ) -> Result<Option<Self::WrapDirectoryReader>>;
  fn wrap_directory_reader(
    &self,
    in_: Option<<Self::Delegate as DirectoryReader>::DirectoryReader>,
  ) -> Result<Option<Self::WrapDirectoryReader>> {
    in_.map_or(Ok(None), |in_reader| {
      self.do_wrap_directory_reader(Some(in_reader))
    })
  }
}

/// Wraps sub readers.
pub trait SubReaderWrapper<LR>
where
  LR: LeafReader,
{
  type LeafReader1: LeafReader;
  /// Wraps a list of `LeafReader`s.
  ///
  /// Returns an array of wrapped `LeafReader`s. The returned array might contain fewer elements
  /// compared to the given reader list if an entire reader is filtered out.
  fn wrap_readers(&self, readers: Vec<LR>) -> Result<Vec<Self::LeafReader1>>;
  fn default_wrap_readers(&self, readers: Vec<LR>) -> Result<Vec<Self::LeafReader2>> {
    let mut wrapped = Vec::with_capacity(readers.len());
    for reader in readers {
      let wrapped_reader = self.wrap(reader)?;
      wrapped.push(wrapped_reader);
    }
    Ok(wrapped)
  }

  type LeafReader2: LeafReader;
  /// Wrap one of the parent `DirectoryReader`'s sub readers.
  ///
  /// * `reader` - the sub reader to wrap
  ///
  /// Returns a wrapped/filtered `LeafReader`.
  fn wrap(&self, reader: LR) -> Result<Self::LeafReader2>;
}
#[derive(Clone)]
pub struct DelegatingCacheHelper<CH>
where
  CH: CacheHelper + Clone,
{
  _delegate: CH,
  cache_key: CacheKey,
}

impl<CH> DelegatingCacheHelper<CH>
where
  CH: CacheHelper + Clone,
{
  pub(crate) fn new(delegate: CH) -> Self {
    Self {
      _delegate: delegate,
      cache_key: CacheKey::new(),
    }
  }
}

impl<CH> CacheHelper for DelegatingCacheHelper<CH>
where
  CH: CacheHelper + Clone,
{
  fn get_key(&self) -> CacheKey {
    self.cache_key.clone()
  }

  fn add_closed_listener(&self, listener: Box<dyn ClosedListener>) -> Result<()> {
    let cache_key = self.cache_key.clone();
    self
      ._delegate
      .add_closed_listener(Box::new(move |_unused: &CacheKey| {
        listener.on_close(&cache_key)
      }))
  }
}
