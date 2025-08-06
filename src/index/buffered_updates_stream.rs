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
use crate::util::info_stream::{InfoStream, InfoStreamLock};
use parking_lot::Mutex;
use std::collections::HashSet;

pub(crate) struct BufferedUpdatesStream;

pub(crate) struct SegmentState;

/// Tracks the contiguous range of packets that have finished resolving.
///
/// Packets are resolved concurrently, and only contiguous completed packets can be written to disk.
pub(crate) struct FinishedSegments {
    info_stream: InfoStreamLock,
    inner: Mutex<Inner>,
}
pub(crate) struct Inner {
    /// Largest del gen, inclusive, for which all prior packets have finished applying.
    completed_del_gen: i64,
    /// This lets us track the "holes" in the current frontier of applying del gens;
    /// once the holes are filled in we can advance completedDelGen.
    finished_del_gens: HashSet<i64>,
}
impl FinishedSegments {
    pub(crate) fn new(info_stream: InfoStreamLock) -> Self {
        FinishedSegments {
            info_stream,
            inner: Mutex::new(Inner {
                completed_del_gen: 0,
                finished_del_gens: HashSet::new(),
            }),
        }
    }

    pub(crate) fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.finished_del_gens.clear();
        inner.completed_del_gen = 0;
    }

    pub(crate) fn still_running(&self, del_gen: i64) -> bool {
        let inner = self.inner.lock();
        del_gen > inner.completed_del_gen && !inner.finished_del_gens.contains(&del_gen)
    }
    pub fn get_completed_del_gen(&self) -> i64 {
        let inner = self.inner.lock();
        inner.completed_del_gen
    }

    pub(crate) fn finished_segment(&self, del_gen: i64) {
        let mut inner = self.inner.lock();
        inner.finished_del_gens.insert(del_gen);
        while inner
            .finished_del_gens
            .contains(&(inner.completed_del_gen + 1))
        {
            let v = inner.completed_del_gen + 1;
            inner.finished_del_gens.remove(&v);
            inner.completed_del_gen += 1;
        }
        {
            let mut info_stream = self.info_stream.lock();
            if info_stream.enabled("BD") {
                info_stream.message(
                    "BD",
                    &format!(
                        "finished packet delGen={} now completedDelGen={}",
                        del_gen, inner.completed_del_gen
                    ),
                );
            }
        }
    }
}
