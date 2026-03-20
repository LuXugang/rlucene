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
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::leaf_reader::{
  LRNormNumericDocValues, LRNumericDocValues, LRSortedSetDocValues, LeafReader,
};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// A struct like class that represents a hierarchical relationship between IndexReader instances.
pub trait IndexReaderContext {
  type IndexReader: IndexReader;
  /// Returns the [`IndexReader`], this context represents.
  fn reader(&self) -> &Self::IndexReader;
  type LeafReader: LeafReader;
  /// Returns the context's leaves if this context is a top-level context.
  ///
  /// # Error
  ///
  /// Error with `UnsupportedOperationException` if this is not a top-level context.
  /// `IndexReaderContext::children`
  fn leaves(&self) -> Result<&[LeafReaderContext<Self::LeafReader>]>;

  fn base(&self) -> &IndexReaderContextBase;
}
#[derive(Clone)]
pub struct IndexReaderContextBase {
  /// `true` if this context struct represents the top-level reader within the hierarchical context.
  pub is_top_level: bool,

  /// The doc base for this reader in the parent, `0` if parent is `None`.
  pub doc_base_in_parent: usize,

  /// The ord for this reader in the parent, `0` if parent is `None`.
  pub ord_in_parent: i32,
  // An object that uniquely identifies this context without referencing segments.
  pub identity: Identity,
}

impl IndexReaderContextBase {
  pub fn new(is_top_level: bool, ord_in_parent: i32, doc_base_in_parent: usize) -> Self {
    Self {
      is_top_level,
      doc_base_in_parent,
      ord_in_parent,
      identity: Identity::new(),
    }
  }

  pub fn id(&self) -> &Identity {
    &self.identity
  }
}
pub type IRCLeafReader<IRC> = <IRC as IndexReaderContext>::LeafReader;
pub type IRCLeafReaderCacheHelper<IRC> = <IRCLeafReader<IRC> as LeafReader>::CacheHelper;
pub type IRCSSDV<IRC> = LRSortedSetDocValues<IRCLeafReader<IRC>>;
pub type IRCNDV<IRC> = LRNumericDocValues<IRCLeafReader<IRC>>;
pub type IRCNormNDV<IRC> = LRNormNumericDocValues<IRCLeafReader<IRC>>;

impl<IRC> IndexReaderContext for Arc<IRC>
where
  IRC: IndexReaderContext,
{
  type IndexReader = IRC::IndexReader;

  fn reader(&self) -> &Self::IndexReader {
    (**self).reader()
  }

  type LeafReader = IRC::LeafReader;

  fn leaves(&self) -> Result<&[LeafReaderContext<Self::LeafReader>]> {
    (**self).leaves()
  }

  fn base(&self) -> &IndexReaderContextBase {
    (**self).base()
  }
}
