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
use crate::index::documents_writer_flush_control::DocumentsWriterFlushControl;
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;

pub(crate) trait FlushPolicy {
    fn on_change<D, P, T, O, TS, L, Q, F>(
        &self,
        control: DocumentsWriterFlushControl<D, P, T, O, TS, L, Q, F>,
    ) where
        D: Directory,
        O: OffsetAttribute,
        P: PayloadAttribute,
        T: TermFrequencyAttribute,
        TS: TokenStream,
        L: LiveIndexWriterConfig,
        Q: Query,
        F: Fn() -> Result<DocumentsWriterPerThread<D, P, T, O, TS, L, Q>>;
}
