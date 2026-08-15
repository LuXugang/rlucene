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
use crate::core::index::index_reader_context::{IndexReaderContext, IndexReaderContextBase};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::Result;
use std::fmt;

/// [`IndexReaderContext`] for [`LeafReader`] instances.
pub struct LeafReaderContext<LR> {
  /// The reader's ord in the top-level's leaves array
  pub ord: usize,
  /// The reader's absolute doc base
  pub doc_base: usize,
  reader: LR,
  base: IndexReaderContextBase,
  pub(crate) top_parent: TopParentMeta,
}
#[derive(Clone, Default)]
pub struct TopParentMeta {
  pub(crate) leaves_num: usize,
  pub(crate) max_doc: i32,
  pub(crate) id: Identity,
}
impl<LR> LeafReaderContext<LR> {
  pub(crate) fn new(
    reader: LR,
    ord: i32,
    doc_base: usize,
    leaf_ord: usize,
    leaf_doc_base: usize,
    parent: TopParentMeta,
  ) -> Self {
    Self {
      ord: leaf_ord,
      doc_base: leaf_doc_base,
      reader,
      base: IndexReaderContextBase::new(true, ord, doc_base),
      top_parent: parent,
    }
  }
  pub(crate) fn from_top_lr(reader: LR) -> Self {
    let mut v = Self::new(reader, 0, 0, 0, 0, TopParentMeta::default());
    // leaf_reader is top
    v.top_parent.id = v.base.id().clone();
    v
  }
}

impl<LR> IndexReaderContext for LeafReaderContext<LR>
where
  LR: LeafReader,
{
  type IndexReader = LR;

  fn reader(&self) -> &Self::IndexReader {
    &self.reader
  }

  type LeafReader = LR;

  fn leaves(&self) -> Result<&[LeafReaderContext<Self::LeafReader>]> {
    Ok(std::slice::from_ref(self))
  }

  fn base(&self) -> &IndexReaderContextBase {
    &self.base
  }
}

impl<LR> LeafReaderContext<LR> {
  pub fn top_parent(&self) -> &TopParentMeta {
    &self.top_parent
  }
}
impl<LR> fmt::Display for LeafReaderContext<LR>
where
  LR: LeafReader,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "LeafReaderContext({} docBase={} ord={})",
      self.reader, self.doc_base, self.ord
    )
  }
}
