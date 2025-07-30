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
use crate::index::documents_writer_flush_control::DocumentsWriterFlushControl;
use crate::index::documents_writer_per_thread::DocumentsWriterPerThread;
use crate::index::flush_policy::FlushPolicy;
use crate::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::search::query::Query;
use crate::store::directory::Directory;

pub struct DummyFlushPolicy;
impl FlushPolicy for DummyFlushPolicy {
    fn on_change<D, Q, L>(
        &self,
        _control: &DocumentsWriterFlushControl<D, Q, L>,
        _per_thread: Option<&DocumentsWriterPerThread<D, Q>>,
    ) where
        D: Directory,
        Q: Query,
        L: LiveIndexWriterConfig,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
