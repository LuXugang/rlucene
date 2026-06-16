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
use crate::core::index::composite_reader_context::{CompositeReaderContext, create};
use crate::core::index::index_reader::{IndexReader, IndexReaderEnum};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;
use std::sync::Arc;

/// A reader composed from sub-readers.
///
/// Instances of this reader type can only be used to get stored fields from the
/// underlying [`LeafReader`]s, and it is not possible to directly retrieve
/// postings. To do that, get the
/// [`LeafReaderContext`](crate::core::index::leaf_reader_context::LeafReaderContext)
/// for all sub-readers via
/// [`IndexReaderContext::leaves`](crate::core::index::index_reader_context::IndexReaderContext::leaves).
///
/// [`IndexReader`] instances for indexes on disk are usually constructed with a
/// call to one of the `DirectoryReader::open` methods, for example
/// [`directory_reader::open`](crate::core::index::directory_reader::open).
/// [`DirectoryReader`](crate::core::index::directory_reader::DirectoryReader)
/// implements the [`CompositeReader`] interface, so it is not possible to
/// directly get postings from it.
///
/// Concrete implementations are usually constructed with a call to one of the
/// static `open` methods, for example
/// [`directory_reader::open`](crate::core::index::directory_reader::open).
///
/// For efficiency, this API often refers to documents via document numbers:
/// non-negative integers that each name a unique document in the index. These
/// document numbers are ephemeral and may change as documents are added to and
/// deleted from an index. Clients should not rely on a document having the same
/// number between sessions.
///
/// NOTE: [`IndexReader`] instances are completely thread safe, meaning multiple
/// threads can call any of their methods concurrently. If your application
/// requires external synchronization, do not synchronize on the reader instance;
/// use your own non-Lucene objects instead.
pub trait CompositeReader: IndexReader {
  type LeafReader: LeafReader + Clone;
  type SubCompositeReader: CompositeReader<LeafReader = Self::LeafReader>;

  /// Expert: returns the sequential sub-readers that this reader is logically
  /// composed of.
  ///
  /// This method may not return `None`.
  ///
  /// NOTE: In contrast to previous Lucene versions, code that wants to get all
  /// [`LeafReader`]s this composite is composed of should use
  /// [`IndexReaderContext::leaves`](crate::core::index::index_reader_context::IndexReaderContext::leaves).
  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>];

  fn to_string(&self) -> String {
    String::new()
  }
}

/// Returns the [`CompositeReaderContext`] for this reader.
pub fn get_context<CR>(composite_reader: CR) -> Result<CompositeReaderContext<CR>>
where
  CR: CompositeReader,
{
  composite_reader.ensure_open()?;
  create(composite_reader)
}
impl<CR> CompositeReader for &CR
where
  CR: CompositeReader,
{
  type LeafReader = CR::LeafReader;
  type SubCompositeReader = CR::SubCompositeReader;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    (**self).get_sequential_sub_readers()
  }

  fn to_string(&self) -> String {
    (**self).to_string()
  }
}
impl<CR> CompositeReader for Arc<CR>
where
  CR: CompositeReader,
{
  type LeafReader = CR::LeafReader;
  type SubCompositeReader = CR::SubCompositeReader;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    (**self).get_sequential_sub_readers()
  }

  fn to_string(&self) -> String {
    (**self).to_string()
  }
}
impl<CR> CompositeReader for Rc<CR>
where
  CR: CompositeReader,
{
  type LeafReader = CR::LeafReader;
  type SubCompositeReader = CR::SubCompositeReader;

  fn get_sequential_sub_readers(
    &self,
  ) -> &[IndexReaderEnum<Self::LeafReader, Self::SubCompositeReader>] {
    (**self).get_sequential_sub_readers()
  }

  fn to_string(&self) -> String {
    (**self).to_string()
  }
}

pub type CompositeReaderBits<CR> = <<CR as CompositeReader>::LeafReader as LeafReader>::Bits;
pub type CompositeReaderTerms<CR> = <<CR as CompositeReader>::LeafReader as LeafReader>::Terms;
