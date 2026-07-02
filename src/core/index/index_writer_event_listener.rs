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
use crate::core::index::merge_policy::DefaultMergeSpecification;
use crate::core::store::directory::Directory;
use std::fmt::{Display, Formatter};

/// A callback event listener for recording key events happened inside
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter).
///
/// # Experimental
///
/// This API follows the original Lucene experimental status.
pub trait IndexWriterEventListener<D>: Display
where
  D: Directory,
{
  /// Invoked at the start of merge on commit.
  ///
  /// * `merge` - merge specification to be tracked
  fn begin_merge_on_full_flush(&self, merge: &DefaultMergeSpecification<D>);

  /// Invoked at the end of merge on commit, due to either merge completed, or
  /// merge timed out according to
  /// [`IndexWriterConfig::set_max_full_flush_merge_wait_millis`](crate::core::index::index_writer_config::IndexWriterConfig::set_max_full_flush_merge_wait_millis).
  ///
  /// * `merge` - merge specification to be tracked
  fn end_merge_on_full_flush(&self, merge: &DefaultMergeSpecification<D>);
}

/// A no-op listener that helps to save `None` checks.
#[derive(Default)]
pub struct NoOpIndexWriterEventListener;

impl Display for NoOpIndexWriterEventListener {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "NoOpIndexWriterEventListener")
  }
}

impl<D> IndexWriterEventListener<D> for NoOpIndexWriterEventListener
where
  D: Directory,
{
  fn begin_merge_on_full_flush(&self, _merge: &DefaultMergeSpecification<D>) {}

  fn end_merge_on_full_flush(&self, _merge: &DefaultMergeSpecification<D>) {}
}

pub type DynIndexWriterEventListener<D> = dyn IndexWriterEventListener<D> + Send + Sync;
pub type CustomIndexWriterEventListener<D> = Box<DynIndexWriterEventListener<D>>;

pub enum IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  NoOp(NoOpIndexWriterEventListener),
  Custom(CustomIndexWriterEventListener<D>),
}

impl<D> IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  pub fn custom<T>(listener: T) -> Self
  where
    T: IndexWriterEventListener<D> + Send + Sync + 'static,
  {
    Self::Custom(Box::new(listener))
  }
}

impl<D> Default for IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  fn default() -> Self {
    Self::NoOp(NoOpIndexWriterEventListener)
  }
}

impl<D> From<NoOpIndexWriterEventListener> for IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  fn from(value: NoOpIndexWriterEventListener) -> Self {
    Self::NoOp(value)
  }
}

impl<D> From<CustomIndexWriterEventListener<D>> for IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  fn from(value: CustomIndexWriterEventListener<D>) -> Self {
    Self::Custom(value)
  }
}

impl<D> Display for IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoOp(listener) => write!(f, "{}", listener),
      Self::Custom(listener) => write!(f, "{}", listener),
    }
  }
}

impl<D> IndexWriterEventListener<D> for IndexWriterEventListenerEnum<D>
where
  D: Directory,
{
  fn begin_merge_on_full_flush(&self, merge: &DefaultMergeSpecification<D>) {
    match self {
      Self::NoOp(listener) => listener.begin_merge_on_full_flush(merge),
      Self::Custom(listener) => listener.begin_merge_on_full_flush(merge),
    }
  }

  fn end_merge_on_full_flush(&self, merge: &DefaultMergeSpecification<D>) {
    match self {
      Self::NoOp(listener) => listener.end_merge_on_full_flush(merge),
      Self::Custom(listener) => listener.end_merge_on_full_flush(merge),
    }
  }
}
