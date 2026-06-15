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
use std::sync::LazyLock;
use std::{fmt, sync::Arc};

use crate::core::util::error::lucene_error::Result;

/// Debugging API for Lucene components such as
/// [`IndexWriter`](crate::core::index::index_writer::IndexWriter)
/// and [`SegmentInfos`](crate::core::index::segment_infos::SegmentInfos).
pub trait InfoStream: Send + Sync {
  /// Prints a message.
  fn message(&self, component: &str, message: &str) -> Result<()>;

  /// Returns true if messages are enabled and should be posted to `message`.
  fn enabled(&self, component: &str) -> bool;

  /// Closes the stream.
  fn close(&self) -> Result<()>;
}

impl<T> InfoStream for Arc<T>
where
  T: InfoStream + ?Sized,
{
  fn message(&self, component: &str, message: &str) -> Result<()> {
    self.as_ref().message(component, message)
  }

  fn enabled(&self, component: &str) -> bool {
    self.as_ref().enabled(component)
  }

  fn close(&self) -> Result<()> {
    self.as_ref().close()
  }
}

/// A global, thread-safe reference to a default `InfoStream`,
/// mirroring `private static InfoStream defaultInfoStream` in Java.
static DEFAULT_INFO_STREAM: LazyLock<Arc<InfoStreamEnum>> =
  LazyLock::new(|| Arc::new(InfoStreamEnum::NoOutput(NoOutput)));

/// Instance of InfoStream that does no logging at all.
#[derive(Clone, Debug, Default)]
pub struct NoOutput;

impl InfoStream for NoOutput {
  fn message(&self, _component: &str, _message: &str) -> Result<()> {
    debug_assert!(
      false,
      "this method should never be called when is_enabled returns false"
    );
    Ok(())
  }

  fn enabled(&self, _component: &str) -> bool {
    false
  }

  fn close(&self) -> Result<()> {
    // Nothing to do.
    Ok(())
  }
}

/// The default `InfoStream` used by newly created types.
pub fn get_default_info_stream() -> Arc<InfoStreamEnum> {
  DEFAULT_INFO_STREAM.clone()
}

/// Sets the default [`InfoStream`] used by newly created types.
pub fn set_default(_info_stream: InfoStreamEnum) {
  todo!()
}
pub enum InfoStreamEnum {
  NoOutput(NoOutput),
  Custom(Box<dyn InfoStream>),
}
impl fmt::Debug for InfoStreamEnum {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      InfoStreamEnum::NoOutput(output) => f.debug_tuple("NoOutput").field(output).finish(),
      InfoStreamEnum::Custom(_) => f.write_str("CustomInfoStream"),
    }
  }
}
impl Default for InfoStreamEnum {
  fn default() -> Self {
    InfoStreamEnum::NoOutput(NoOutput)
  }
}
impl InfoStream for InfoStreamEnum {
  fn message(&self, component: &str, message: &str) -> Result<()> {
    match self {
      InfoStreamEnum::NoOutput(output) => output.message(component, message),
      InfoStreamEnum::Custom(output) => output.message(component, message),
    }
  }

  fn enabled(&self, component: &str) -> bool {
    match self {
      InfoStreamEnum::NoOutput(output) => output.enabled(component),
      InfoStreamEnum::Custom(output) => output.enabled(component),
    }
  }

  fn close(&self) -> Result<()> {
    match self {
      InfoStreamEnum::NoOutput(output) => output.close(),
      InfoStreamEnum::Custom(output) => output.close(),
    }
  }
}
pub type InfoStreamMT = Arc<InfoStreamEnum>;
