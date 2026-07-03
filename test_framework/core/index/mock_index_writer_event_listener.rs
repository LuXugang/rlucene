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
use crate::core::index::index_writer_event_listener::IndexWriterEventListener;
use crate::core::index::merge_policy::MergeStat;
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};

/// Mock IndexWriterEventListener to verify invocation of event methods.
#[derive(Default)]
pub struct MockIndexWriterEventListener {
  begin_merge_called: AtomicBool,
  end_merge_called: AtomicBool,
}

impl MockIndexWriterEventListener {
  pub fn new() -> Self {
    Self {
      begin_merge_called: AtomicBool::new(false),
      end_merge_called: AtomicBool::new(false),
    }
  }

  pub fn is_events_recorded(&self) -> bool {
    self.begin_merge_called.load(Ordering::SeqCst) && self.end_merge_called.load(Ordering::SeqCst)
  }
}

impl Display for MockIndexWriterEventListener {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockIndexWriterEventListener")
  }
}

impl IndexWriterEventListener for MockIndexWriterEventListener {
  fn begin_merge_on_full_flush(&self, _merge_states: &[MergeStat]) {
    self.begin_merge_called.store(true, Ordering::SeqCst);
  }

  fn end_merge_on_full_flush(&self, _merge_states: &[MergeStat]) {
    self.end_merge_called.store(true, Ordering::SeqCst);
  }
}
