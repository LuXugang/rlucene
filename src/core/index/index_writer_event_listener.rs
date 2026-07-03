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
use crate::core::index::merge_policy::MergeStat;
use std::fmt::{Display, Formatter};

/// A callback event listener for recording key events happened inside
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter).
///
/// # Experimental
///
/// This API follows the original Lucene experimental status.
pub trait IndexWriterEventListener: Display {
  /// Invoked at the start of merge on commit.
  ///
  /// * `merge_states` - merge states to be tracked
  fn begin_merge_on_full_flush(&self, merge_states: &[MergeStat]);

  /// Invoked at the end of merge on commit, due to either merge completed, or
  /// merge timed out according to
  /// [`IndexWriterConfig::set_max_full_flush_merge_wait_millis`](crate::core::index::index_writer_config::IndexWriterConfig::set_max_full_flush_merge_wait_millis).
  ///
  /// * `merge_states` - merge states to be tracked
  fn end_merge_on_full_flush(&self, merge_states: &[MergeStat]);
}

/// A no-op listener that helps to save `None` checks.
#[derive(Default)]
pub struct NoOpIndexWriterEventListener;

impl Display for NoOpIndexWriterEventListener {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "NoOpIndexWriterEventListener")
  }
}

impl IndexWriterEventListener for NoOpIndexWriterEventListener {
  fn begin_merge_on_full_flush(&self, _merge_states: &[MergeStat]) {}

  fn end_merge_on_full_flush(&self, _merge_states: &[MergeStat]) {}
}

pub type DynIndexWriterEventListener = dyn IndexWriterEventListener + Send + Sync;
pub type CustomIndexWriterEventListener = Box<DynIndexWriterEventListener>;

pub enum IndexWriterEventListenerEnum {
  NoOp(NoOpIndexWriterEventListener),
  Custom(CustomIndexWriterEventListener),
}

impl IndexWriterEventListenerEnum {
  pub fn custom<T>(listener: T) -> Self
  where
    T: IndexWriterEventListener + Send + Sync + 'static,
  {
    Self::Custom(Box::new(listener))
  }
}

impl Default for IndexWriterEventListenerEnum {
  fn default() -> Self {
    Self::NoOp(NoOpIndexWriterEventListener)
  }
}

impl From<NoOpIndexWriterEventListener> for IndexWriterEventListenerEnum {
  fn from(value: NoOpIndexWriterEventListener) -> Self {
    Self::NoOp(value)
  }
}

impl From<CustomIndexWriterEventListener> for IndexWriterEventListenerEnum {
  fn from(value: CustomIndexWriterEventListener) -> Self {
    Self::Custom(value)
  }
}

impl Display for IndexWriterEventListenerEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NoOp(listener) => write!(f, "{}", listener),
      Self::Custom(listener) => write!(f, "{}", listener),
    }
  }
}

impl IndexWriterEventListener for IndexWriterEventListenerEnum {
  fn begin_merge_on_full_flush(&self, merge_states: &[MergeStat]) {
    match self {
      Self::NoOp(listener) => listener.begin_merge_on_full_flush(merge_states),
      Self::Custom(listener) => listener.begin_merge_on_full_flush(merge_states),
    }
  }

  fn end_merge_on_full_flush(&self, merge_states: &[MergeStat]) {
    match self {
      Self::NoOp(listener) => listener.end_merge_on_full_flush(merge_states),
      Self::Custom(listener) => listener.end_merge_on_full_flush(merge_states),
    }
  }
}
