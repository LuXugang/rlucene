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
use crate::index::documents_writer_delete_queue::DocumentsWriterDeleteQueue;
use crate::index::segment_info::SegmentInfo;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use std::sync::Arc;

pub(crate) struct DocumentsWriter<Q>
where
    Q: Query,
{
    pub(crate) delete_queue: Arc<DocumentsWriterDeleteQueue<Q>>,
}
impl<Q> DocumentsWriter<Q>
where
    Q: Query,
{
    pub(crate) fn reset_delete_queue(&self, max_num_pending_ops: usize) -> i64 {
        todo!()
    }
}

pub(crate) trait FlushNotifications {
    /// Called when files were written to disk that are not used anymore.
    /// It's the implementation's responsibility to clean these files up.
    fn delete_unused_files<I>(&mut self, files: I)
    where
        I: IntoIterator<Item = String>;

    /// Called when a segment failed to flush.
    fn flush_failed<D>(&mut self, info: &SegmentInfo<D>)
    where
        D: Directory;

    /// Called after one or more segments were flushed to disk.
    fn after_segments_flushed(&mut self) -> Result<()>;

    /// Should be called if a flush or an indexing operation caused
    /// a tragic / unrecoverable event.
    fn on_tragic_event(&mut self, event: LuceneError, message: &str);

    /// Called once deletes have been applied either after a flush or on a deletes call.
    fn on_deletes_applied(&mut self);

    /// Called once the DocumentsWriter ticket queue has a backlog. This means there is an inner
    /// thread that tries to publish flushed segments but can't keep up with the other threads
    /// flushing new segments. This likely requires other thread to forcefully purge the buffer to
    /// help publishing. This can't be done in-place since we might hold index writer locks when this
    /// is called. The caller must ensure that the purge happens without an index writer lock being
    /// held.
    fn on_ticket_backlog(&mut self);
}
