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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::double_values::DoubleValues;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Display;

/// Base trait for producing [`DoubleValues`].
///
/// To obtain a [`DoubleValues`] object for a leaf reader, clients should call
/// [`DoubleValuesSource::rewrite`] against the top-level searcher, and then
/// call [`DoubleValuesSource::get_values`] on the resulting
/// `DoubleValuesSource`.
///
/// `DoubleValuesSource` objects for `NumericDocValues` fields can be obtained
/// from field-specific implementations when special `i64`-to-`f64` encoding is
/// required.
///
/// Scores may be used as a source for value calculations by wrapping a scorer
/// as [`DoubleValues`] and passing the resulting values to
/// [`DoubleValuesSource::get_values`]. The scores can then be accessed by
/// implementations that require them.
pub trait DoubleValuesSource<IRC>: SegmentCacheable<IRC> + Display
where
  IRC: IndexReaderContext,
{
  type Values: DoubleValues;

  /// Returns a [`DoubleValues`] instance for the passed-in
  /// [`LeafReaderContext`] and scores.
  ///
  /// If scores are not needed to calculate the values (i.e.,
  /// [`DoubleValuesSource::needs_scores`] returns `false`), callers may safely
  /// pass `None` for the `scores` parameter.
  fn get_values(
    &self,
    ctx: &LeafReaderContext<IRCLeafReader<IRC>>,
    scores: Option<Box<dyn DoubleValues>>,
  ) -> Result<Self::Values>;

  /// Return `true` if document scores are needed to calculate values.
  fn needs_scores(&self) -> bool;

  /// An explanation of the value for the named document.
  ///
  /// # Parameters
  /// - `ctx`: the reader's context to create the [`Explanation`] for.
  /// - `doc_id`: the document's id relative to the given context's reader.
  ///
  /// # Returns
  /// An [`Explanation`] for the value.
  fn explain(
    &self,
    _ctx: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc_id: i32,
    _score_explanation: Explanation,
  ) -> Result<Explanation> {
    todo!()
  }

  /// Return a `DoubleValuesSource` specialized for the given
  /// [`IndexSearcher`].
  ///
  /// Implementations should assume that this will only be called once.
  /// IndexReader-independent implementations can just return themselves.
  ///
  /// Queries that use `DoubleValuesSource` objects should call `rewrite`
  /// during `Query::create_weight` rather than during `Query::rewrite` to
  /// avoid IndexReader reference leakage.
  ///
  /// For the same reason, implementations that cache references to the
  /// [`IndexSearcher`] should return a new object from this method.
  type Rewritten: DoubleValuesSource<IRC>;
  fn rewrite(&self, reader: &IndexSearcher<IRC>) -> Result<Self::Rewritten>;

  /// Create a sort field based on the value of this producer.
  ///
  /// # Parameters
  /// - `reverse`: `true` if the sort should be decreasing.
  fn get_sort_field(&self, _reverse: bool) -> Result<SortFieldEnum> {
    todo!()
  }
}

impl<IRC, T> DoubleValuesSource<IRC> for Box<T>
where
  IRC: IndexReaderContext,
  T: DoubleValuesSource<IRC> + ?Sized,
{
  type Values = T::Values;

  fn get_values(
    &self,
    ctx: &LeafReaderContext<IRCLeafReader<IRC>>,
    scores: Option<Box<dyn DoubleValues>>,
  ) -> Result<Self::Values> {
    self.as_ref().get_values(ctx, scores)
  }

  fn needs_scores(&self) -> bool {
    self.as_ref().needs_scores()
  }

  fn explain(
    &self,
    ctx: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc_id: i32,
    score_explanation: Explanation,
  ) -> Result<Explanation> {
    self.as_ref().explain(ctx, doc_id, score_explanation)
  }

  type Rewritten = T::Rewritten;

  fn rewrite(&self, reader: &IndexSearcher<IRC>) -> Result<Self::Rewritten> {
    self.as_ref().rewrite(reader)
  }

  fn get_sort_field(&self, reverse: bool) -> Result<SortFieldEnum> {
    self.as_ref().get_sort_field(reverse)
  }
}
