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
use std::cell::RefCell;

/// A directory operation at which deterministic test failures are evaluated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FailurePoint {
  Sync,
  Rename,
  SyncMetadata,
  DeleteFile,
  CreateOutput,
  CreateTempOutput,
  OpenInput,
  CloseOutput,
  WriteOutput,
  CopyBytes,
  CloseInput,
}

/// The implementation type that owns a logical execution frame.
///
/// These values intentionally describe the Java-level type identity used by
/// `LuceneTestCase.callStackContains`. They do not depend on Rust symbol names
/// or generic parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionOwner {
  DocumentsWriterPerThread,
  IOUtils,
  IndexFileDeleter,
  IndexingChain,
  IndexWriter,
  LuceneTestCase,
  Lucene90LiveDocsFormat,
  PersistentSnapshotDeletionPolicy,
  ReadersAndUpdates,
  SegmentInfos,
  SegmentMerger,
  StoredFieldsConsumer,
  TermVectorsConsumer,
}

/// A method represented in the logical execution stack used by failure tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionMethod {
  Abort,
  ApplyAllDeletesAndUpdates,
  Checkpoint,
  Close,
  DecRef,
  DeleteCommits,
  DeleteFiles,
  FinishCommit,
  FinishDocument,
  Flush,
  GetReadOnlyClone,
  InitTermVectorsWriter,
  Merge,
  MergeTerms,
  Operation,
  Persist,
  PrepareCommit,
  ReadLiveDocs,
  RollbackInternal,
  RollbackInternalNoCommit,
  SlowFileExists,
  WriteGlobalFieldMap,
  WriteLiveDocs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionFrame {
  owner: ExecutionOwner,
  method: ExecutionMethod,
}

impl ExecutionFrame {
  pub(crate) const fn new(owner: ExecutionOwner, method: ExecutionMethod) -> Self {
    Self { owner, method }
  }
}

thread_local! {
  static EXECUTION_STACK: RefCell<Vec<ExecutionFrame>> = const { RefCell::new(Vec::new()) };
}

/// Adds a platform-independent logical frame for the lifetime of this guard.
pub(crate) struct ExecutionScope {
  frame: ExecutionFrame,
}

impl ExecutionScope {
  pub(crate) fn enter(owner: ExecutionOwner, method: ExecutionMethod) -> Self {
    let frame = ExecutionFrame::new(owner, method);
    EXECUTION_STACK.with_borrow_mut(|stack| stack.push(frame));
    Self { frame }
  }

  pub(crate) fn contains(owner: ExecutionOwner, method: ExecutionMethod) -> bool {
    let frame = ExecutionFrame::new(owner, method);
    EXECUTION_STACK.with_borrow(|stack| stack.contains(&frame))
  }

  pub(crate) fn contains_method(method: ExecutionMethod) -> bool {
    EXECUTION_STACK.with_borrow(|stack| stack.iter().any(|frame| frame.method == method))
  }

  pub(crate) fn contains_owner(owner: ExecutionOwner) -> bool {
    EXECUTION_STACK.with_borrow(|stack| stack.iter().any(|frame| frame.owner == owner))
  }
}

impl Drop for ExecutionScope {
  fn drop(&mut self) {
    EXECUTION_STACK.with_borrow_mut(|stack| {
      debug_assert_eq!(Some(self.frame), stack.pop());
    });
  }
}

/// Stable context supplied to every deterministic failure evaluation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FailureContext {
  point: FailurePoint,
}

impl FailureContext {
  pub(crate) const fn new(point: FailurePoint) -> Self {
    Self { point }
  }

  pub(crate) const fn point(&self) -> FailurePoint {
    self.point
  }

  pub(crate) fn contains(&self, owner: ExecutionOwner, method: ExecutionMethod) -> bool {
    ExecutionScope::contains(owner, method)
  }

  pub(crate) fn contains_method(&self, method: ExecutionMethod) -> bool {
    ExecutionScope::contains_method(method)
      || matches!(
        (self.point, method),
        (
          FailurePoint::CloseInput | FailurePoint::CloseOutput,
          ExecutionMethod::Close
        )
      )
  }

  pub(crate) fn contains_owner(&self, owner: ExecutionOwner) -> bool {
    ExecutionScope::contains_owner(owner)
  }
}

#[cfg(test)]
mod tests {
  use super::{ExecutionMethod, ExecutionOwner, ExecutionScope, FailureContext, FailurePoint};

  #[test]
  fn close_failure_points_match_close_method() {
    assert!(FailureContext::new(FailurePoint::CloseInput).contains_method(ExecutionMethod::Close));
    assert!(FailureContext::new(FailurePoint::CloseOutput).contains_method(ExecutionMethod::Close));
    assert!(
      !FailureContext::new(FailurePoint::WriteOutput).contains_method(ExecutionMethod::Close)
    );
  }

  #[test]
  fn execution_scopes_are_nested_and_removed_on_drop() {
    let context = FailureContext::new(FailurePoint::Sync);
    assert!(!context.contains_owner(ExecutionOwner::IndexWriter));
    {
      let _scope = ExecutionScope::enter(ExecutionOwner::IndexWriter, ExecutionMethod::Operation);
      assert!(context.contains(ExecutionOwner::IndexWriter, ExecutionMethod::Operation));
    }
    assert!(!context.contains_owner(ExecutionOwner::IndexWriter));
  }
}
