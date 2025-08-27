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
use std::cell::RefCell;
use std::sync::Arc;

use once_cell::sync::Lazy;

/// Debugging API for Lucene classes such as
/// [`IndexWriter`](crate::index::index_writer::IndexWriter)
/// and [`SegmentInfos`](crate::index::segment_infos::SegmentInfos).
pub trait InfoStream: Send + Sync {
    /// Prints a message.
    fn message(&self, component: &str, message: &str);

    /// Returns true if messages are enabled and should be posted to `message`.
    fn enabled(&self, component: &str) -> bool;

    /// Closes the stream.
    fn close(&self);
}

/// A global, thread-safe reference to a default `InfoStream`,
/// mirroring `private static InfoStream defaultInfoStream` in Java.
static DEFAULT_INFO_STREAM: Lazy<Arc<InfoStreamEnum>> =
    Lazy::new(|| Arc::new(InfoStreamEnum::NoOutput(NoOutput)));

/// Instance of InfoStream that does no logging at all.
#[derive(Clone, Debug)]
pub struct NoOutput;

impl InfoStream for NoOutput {
    fn message(&self, _component: &str, _message: &str) {
        debug_assert!(
            false,
            "this method should never be called when is_enabled returns false"
        );
    }

    fn enabled(&self, _component: &str) -> bool {
        false
    }

    fn close(&self) {
        // Nothing to do.
    }
}

/// The default `InfoStream` used by a newly instantiated classes.
pub fn get_default_info_stream() -> Arc<InfoStreamEnum> {
    DEFAULT_INFO_STREAM.clone()
}

/// Sets the default [`InfoStream`] used by a newly instantiated classes.
pub fn set_default(_info_stream: InfoStreamEnum) {
    todo!()
}
#[derive(Clone, Debug)]
pub enum InfoStreamEnum {
    NoOutput(NoOutput),
}
impl InfoStream for InfoStreamEnum {
    fn message(&self, component: &str, message: &str) {
        match self {
            InfoStreamEnum::NoOutput(output) => output.message(component, message),
        }
    }

    fn enabled(&self, component: &str) -> bool {
        match self {
            InfoStreamEnum::NoOutput(output) => output.enabled(component),
        }
    }

    fn close(&self) {
        match self {
            InfoStreamEnum::NoOutput(output) => output.close(),
        }
    }
}
/// for multi-threaded scenarios
pub type InfoStreamMT = Arc<InfoStreamEnum>;
/// for single-threaded scenarios
pub type InfoStreamST = RefCell<InfoStreamEnum>;
#[cfg(test)]
mod tests {
    use crate::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    pub struct TestInfoStream;
    #[test]
    fn test_test_points_off() -> Result<()> {
        // TODO : IndexWriter not implemented
        Ok(())
    }
    #[test]
    fn test_test_pointson() -> Result<()> {
        // TODO : IndexWriter not implemented
        Ok(())
    }
}
