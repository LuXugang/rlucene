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
use std::sync::{Arc, Mutex};

/// Debugging API for Lucene classes such as [`IndexWriter`](crate::index::index_writer::IndexWriter)
/// and [`SegmentInfos`](crate::index::segment_infos::SegmentInfos).
pub trait InfoStream: Send + Sync {
    /// Prints a message.
    fn message(&self, component: &str, message: &str);

    /// Returns true if messages are enabled and should be posted to `message`.
    fn is_enabled(&self, component: &str) -> bool;

    /// Closes the stream.
    fn close(&self);
}

/// A global, thread-safe reference to a default `InfoStream`,
/// mirroring `private static InfoStream defaultInfoStream` in Java.
static DEFAULT_INFOSTREAM: Lazy<Mutex<Arc<InfoStreamEnum>>> =
    Lazy::new(|| Mutex::new(Arc::new(InfoStreamEnum::NoOutput(NoOutput))));

/// Instance of InfoStream that does no logging at all.
#[derive(Clone)]
pub struct NoOutput;

impl InfoStream for NoOutput {
    fn message(&self, _component: &str, _message: &str) {
        debug_assert!(
            false,
            "message() should not be called when is_enabled returns false"
        );
    }

    fn is_enabled(&self, _component: &str) -> bool {
        false
    }

    fn close(&self) {
        // Nothing to do.
    }
}

/// The default `InfoStream` used by a newly instantiated classes.
pub fn get_default() -> Arc<InfoStreamEnum> {
    let lock = DEFAULT_INFOSTREAM.lock().unwrap();
    lock.clone()
}

/// Sets the default [`InfoStream`] used by a newly instantiated classes.
pub fn set_default(info_stream: Arc<InfoStreamEnum>) {
    let mut lock = DEFAULT_INFOSTREAM.lock().unwrap();
    *lock = info_stream;
}
#[derive(Clone)]
pub enum InfoStreamEnum {
    NoOutput(NoOutput),
}
impl InfoStream for InfoStreamEnum {
    fn message(&self, component: &str, message: &str) {
        match self {
            InfoStreamEnum::NoOutput(output) => output.message(component, message),
        }
    }

    fn is_enabled(&self, component: &str) -> bool {
        match self {
            InfoStreamEnum::NoOutput(output) => output.is_enabled(component),
        }
    }

    fn close(&self) {
        match self {
            InfoStreamEnum::NoOutput(output) => output.close(),
        }
    }
}
