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
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;

/// Debugging API for Lucene classes such as [`IndexWriter`](crate::index::index_writer::IndexWriter)
/// and [`SegmentInfos`](crate::index::segment_infos::SegmentInfos).
pub trait InfoStream: Send + Sync {
    /// Prints a message.
    fn message(&mut self, component: &str, message: &str);

    /// Returns true if messages are enabled and should be posted to `message`.
    fn enabled(&mut self, component: &str) -> bool;

    /// Closes the stream.
    fn close(&mut self);
}

/// A global, thread-safe reference to a default `InfoStream`,
/// mirroring `private static InfoStream defaultInfoStream` in Java.
static DEFAULT_INFOSTREAM: Lazy<Arc<Mutex<InfoStreamEnum>>> =
    Lazy::new(|| Arc::new(Mutex::new(InfoStreamEnum::NoOutput(NoOutput))));

/// Instance of InfoStream that does no logging at all.
#[derive(Clone)]
pub struct NoOutput;

impl InfoStream for NoOutput {
    fn message(&mut self, _component: &str, _message: &str) {
        debug_assert!(
            false,
            "this method should never be called when is_enabled returns false"
        );
    }

    fn enabled(&mut self, _component: &str) -> bool {
        false
    }

    fn close(&mut self) {
        // Nothing to do.
    }
}

/// The default `InfoStream` used by a newly instantiated classes.
pub fn get_default_info_stream() -> Arc<Mutex<InfoStreamEnum>> {
    DEFAULT_INFOSTREAM.clone()
}

/// Sets the default [`InfoStream`] used by a newly instantiated classes.
pub fn set_default(info_stream: InfoStreamEnum) {
    let mut lock = DEFAULT_INFOSTREAM.lock();
    *lock = info_stream;
}
#[derive(Clone)]
pub enum InfoStreamEnum {
    NoOutput(NoOutput),
}
impl InfoStream for InfoStreamEnum {
    fn message(&mut self, component: &str, message: &str) {
        match self {
            InfoStreamEnum::NoOutput(output) => {
                output.message(component, message)
            },
        }
    }

    fn enabled(&mut self, component: &str) -> bool {
        match self {
            InfoStreamEnum::NoOutput(output) => output.enabled(component),
        }
    }

    fn close(&mut self) {
        match self {
            InfoStreamEnum::NoOutput(output) => output.close(),
        }
    }
}
/// for multi-threaded scenarios
pub type InfoStreamLock = Arc<Mutex<InfoStreamEnum>>;
/// for single-threaded scenarios
pub type InfoStreamBorrow = Arc<Mutex<InfoStreamEnum>>;
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
