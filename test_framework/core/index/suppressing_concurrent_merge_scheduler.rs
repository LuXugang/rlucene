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
use crate::core::index::concurrent_merge_scheduler::{
  ConcurrentMergeScheduler, ConcurrentMergeSchedulerBase, ConcurrentMergeSchedulerDefaults,
};
use crate::core::util::error::lucene_error::{CaughtResult, CaughtResultExt, LuceneError, Result};

/** A `ConcurrentMergeScheduler` hook that ignores expected merge exceptions. */
#[derive(Clone)]
pub struct SuppressingConcurrentMergeScheduler {
  expected: ExpectedMergeException,
}

#[derive(Clone)]
enum ExpectedMergeException {
  All,
  WriterClosedOrTragic,
}

impl SuppressingConcurrentMergeScheduler {
  pub fn all() -> Self {
    Self {
      expected: ExpectedMergeException::All,
    }
  }

  pub fn writer_closed_or_tragic() -> Self {
    Self {
      expected: ExpectedMergeException::WriterClosedOrTragic,
    }
  }

  fn is_ok(&self, error: &LuceneError) -> bool {
    match self.expected {
      ExpectedMergeException::All => true,
      ExpectedMergeException::WriterClosedOrTragic => {
        matches!(error, LuceneError::AlreadyClosed(_))
          || matches!(error, LuceneError::IllegalState(_))
            && error
              .to_string()
              .contains("this writer hit an unrecoverable error")
      },
    }
  }
}

impl ConcurrentMergeSchedulerBase for SuppressingConcurrentMergeScheduler {
  fn handle_merge_exception(
    &self,
    scheduler: &ConcurrentMergeScheduler,
    result: CaughtResult,
  ) -> Result<()> {
    let error = result
      .caught_failure("panic in merge thread")
      .ok_or_else(|| LuceneError::illegal_argument("merge result must contain a failure"))?;
    if self.is_ok(&error) {
      Ok(())
    } else {
      ConcurrentMergeSchedulerDefaults::handle_merge_exception(scheduler, result)
    }
  }
}
