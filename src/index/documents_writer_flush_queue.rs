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
use crate::index::documents_writer_per_thread::FlushedSegment;
use crate::index::frozen_buffered_updates::FrozenBufferedUpdates;
use crate::search::query::Query;
use crate::store::directory::Directory;
use parking_lot::Mutex;

pub(crate) struct DocumentsWriterFlushQueue {}

pub(crate) struct FlushTicket<D, Q>
where
    Q: Query,
    D: Directory,
{
    frozen_updates: FrozenBufferedUpdates<Q>,
    has_segment: bool,
    segment: Option<FlushedSegment<D, Q>>,
    failed: bool,
    published: bool,
    lock: Mutex<()>,
}
impl<D, Q> FlushTicket<D, Q>
where
    D: Directory,
    Q: Query,
{
    pub(crate) fn new(frozen_updates: FrozenBufferedUpdates<Q>, has_segment: bool) -> Self {
        FlushTicket {
            frozen_updates,
            has_segment,
            segment: None,
            failed: false,
            published: false,
            lock: Mutex::new(()),
        }
    }
    pub(crate) fn can_publish(&self) -> bool {
        !self.has_segment || self.segment.is_some() || self.failed
    }

    pub(crate) fn mark_published(&mut self) {
        let _ = self.lock.lock();
        assert!(
            !self.published,
            "ticket was already published - can not publish twice"
        );
        self.published = true;
    }

    fn set_segment(&mut self, segment: FlushedSegment<D, Q>) {
        assert!(!self.failed, "cannot set segment on a failed ticket");
        self.segment = Some(segment);
    }

    fn set_failed(&mut self) {
        assert!(self.segment.is_none());
        self.failed = true;
    }
    /// Returns the flushed segment, or `None` if this flush ticket doesn’t have a segment.
    /// This can occur when the ticket represents a flushed global frozen updates package.
    pub(crate) fn get_flushed_segment(&self) -> Option<&FlushedSegment<D, Q>> {
        self.segment.as_ref()
    }
    /// Returns a frozen global deletes package.
    pub(crate) fn get_frozen_updates(&self) -> &FrozenBufferedUpdates<Q> {
        &self.frozen_updates
    }
}
