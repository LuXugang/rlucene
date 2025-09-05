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
use crate::analysis::analyzer::Analyzer;
use std::sync::Arc;

pub struct IndexWriterConfig<A> {
    analyzer: Arc<A>,
}
impl<A> IndexWriterConfig<A>
where
    A: Analyzer,
{
    pub fn new(analyzer: Arc<A>) -> Self {
        Self { analyzer }
    }
}

/// Specifies the open mode for [`IndexWriter`](crate::index::index_writer::IndexWriter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// Creates a new index or overwrites an existing one.
    Create,

    /// Opens an existing index.
    Append,

    /// Creates a new index if one does not exist, otherwise it opens the index
    /// and documents will be appended.
    CreateOrAppend,
}

/// Denotes a flush trigger is disabled.
pub const DISABLE_AUTO_FLUSH: i32 = -1;

/// Disabled by default (because IndexWriter flushes by RAM usage by default).
pub const DEFAULT_MAX_BUFFERED_DELETE_TERMS: i32 = DISABLE_AUTO_FLUSH;

/// Disabled by default (because IndexWriter flushes by RAM usage by default).
pub const DEFAULT_MAX_BUFFERED_DOCS: i32 = DISABLE_AUTO_FLUSH;

/// Default value is 16 MB (which means flush when buffered docs consume approximately 16 MB RAM).
pub const DEFAULT_RAM_BUFFER_SIZE_MB: f64 = 16.0;

/// Default setting (true) for [`set_reader_pooling`](Self::set_reader_pooling).
///
/// We changed this default to true with concurrent deletes/updates (LUCENE-7868),
/// because we will otherwise need to open and close segment readers more frequently.
/// False is still supported, but will have worse performance since readers will
/// be forced to aggressively move all state to disk.
pub const DEFAULT_READER_POOLING: bool = true;

/// Default value is 1945. Change using [`set_ram_per_thread_hard_limit_mb`].
pub const DEFAULT_RAM_PER_THREAD_HARD_LIMIT_MB: i32 = 1945;

/// Default value for compound file system for newly written segments (set to `true`).
/// For batch indexing with very large ram buffers use `false`.
pub const DEFAULT_USE_COMPOUND_FILE_SYSTEM: bool = true;

/// Default value for whether calls to [`IndexWriter::close`] include a commit.
pub const DEFAULT_COMMIT_ON_CLOSE: bool = true;

/// Default value for time to wait for merges on commit or getReader (when using a
/// [`MergePolicy`] that implements [`MergePolicy::find_full_flush_merges`]).
pub const DEFAULT_MAX_FULL_FLUSH_MERGE_WAIT_MILLIS: i64 = 500;
