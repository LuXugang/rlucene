/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Debugging API for Lucene classes such as
/// [`IndexWriter`](crate::index::index_writer::IndexWriter)
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
            InfoStreamEnum::NoOutput(output) => output.message(component, message),
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
