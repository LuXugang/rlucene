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
use crate::util::accountable::Accountable;
use std::sync::atomic::AtomicI32;

//TODO
#[allow(unused)]
const BYTES_PER_DEL_QUERY: i64 = 0;

/// Holds buffered deletes and updates, including deletions by docID, term, or query for a single segment.
///
/// This structure is used to manage buffered pending deletes and updates that apply to the
/// segment to be flushed. Once these deletes and updates are pushed (during flush in
/// `DocumentsWriter`), they are converted into a `FrozenBufferedUpdates` instance and
/// forwarded to the `BufferedUpdatesStream`.
///
/// # Note
/// - Instances of this structure are accessed either via a private instance on `DocumentWriterPerThread`,
///   or through synchronized code in the `DocumentsWriterDeleteQueue`.
#[allow(unused)]
struct BufferedUpdates {
    segment_name: String,
    num_field_updates: AtomicI32,
}
impl BufferedUpdates {
    #[allow(unused)]
    pub fn new(_segment_name: &str) -> BufferedUpdates {
        todo!()
    }
}

impl Accountable for BufferedUpdates {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}
#[allow(unused)]
struct DeletedTerms {}
