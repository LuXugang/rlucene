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
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::concurrent_merge_scheduler::{
  ConcurrentMergeScheduler, ConcurrentMergeSchedulerBase, ConcurrentMergeSchedulerDefaults,
};
use crate::core::index::merge_policy::OneMerge;
use crate::core::search::task_executor::TaskExecutor;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{CaughtResult, CaughtResultExt, LuceneError, Result};
use std::sync::Arc;

/** A `ConcurrentMergeScheduler` hook that ignores expected merge errors. */
#[derive(Clone)]
pub struct SuppressingConcurrentMergeScheduler {
  expected: ExpectedMergeException,
  always_use_intra_merge_executor: bool,
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
      always_use_intra_merge_executor: false,
    }
  }

  pub fn writer_closed_or_tragic() -> Self {
    Self {
      expected: ExpectedMergeException::WriterClosedOrTragic,
      always_use_intra_merge_executor: false,
    }
  }

  pub fn writer_closed_or_tragic_with_parallel_executor() -> Self {
    Self {
      expected: ExpectedMergeException::WriterClosedOrTragic,
      always_use_intra_merge_executor: true,
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
  fn get_intra_merge_executor<D, CR>(
    &self,
    scheduler: &ConcurrentMergeScheduler,
    merge: &OneMerge<D, CR>,
  ) -> Result<Arc<TaskExecutor>>
  where
    D: Directory,
    CR: CodecReader,
  {
    if self.always_use_intra_merge_executor {
      scheduler.get_parallel_merge_executor()
    } else {
      ConcurrentMergeSchedulerDefaults::get_intra_merge_executor(scheduler, merge)
    }
  }

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
