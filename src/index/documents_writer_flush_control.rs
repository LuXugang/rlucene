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
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::analysis::token_stream::TokenStream;
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool;
use crate::index::documents_writer_stall_control::DocumentsWriterStallControl;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use crate::util::info_stream::InfoStreamLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64};
use std::sync::Arc;

pub(crate) struct DocumentsWriterFlushControl<D, P, T, O, TS, L, Q, F>
where
    D: Directory,
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
    TS: TokenStream,
    L: LiveIndexWriterConfig,
    Q: Query,
    F: Fn() -> Result<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>>,
{
    hard_max_bytes_per_dwpt: i64,
    active_bytes: i64,
    flush_bytes: AtomicI64,
    num_pending: AtomicI32,
    num_docs_since_stalled: i32,
    flush_deletes: AtomicBool,
    full_flush: bool,
    full_flush_mark_done: bool,
    flush_queue: VecDeque<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>>,
    blocked_flushes: VecDeque<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>>,
    flushing_writers: Vec<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>>,
    max_configured_ram_buffer: f64,
    peak_active_bytes: i64,
    peak_flush_bytes: i64,
    peak_net_bytes: i64,
    peak_delta: i64,
    flush_by_ram_was_disabled: bool,
    stall_control: DocumentsWriterStallControl,
    per_thread_pool: DocumentsWriterPerThreadPool<D, P, T, O, TS, L, Q, F>,
    closed: bool,
    config: Arc<L>,
    info_stream: InfoStreamLock,
}
