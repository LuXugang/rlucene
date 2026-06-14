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
use crate::core::util::error::lucene_error::Result;
/// A trait for implementations that support two-phase commit. You can use
/// `TwoPhaseCommitTool` to execute a 2-phase commit algorithm over several
/// [`TwoPhaseCommit`]s.
pub trait TwoPhaseCommit {
  /// The first stage of a 2-phase commit. Implementations should do as much
  /// work as possible in this method, but avoid actual committing changes. If
  /// the 2-phase commit fails, [`TwoPhaseCommit::rollback`] is called to
  /// discard all changes since last successful commit.
  fn prepare_commit(&self) -> Result<i64>;

  /// The second phase of a 2-phase commit. Implementations should ideally do
  /// very little work in this method following
  /// [`TwoPhaseCommit::prepare_commit`], and after it returns, the caller can
  /// assume that the changes were successfully committed to the underlying
  /// storage.
  fn commit(&self) -> Result<i64>;

  /// Discards any changes that have occurred since the last commit. In a
  /// 2-phase commit algorithm, where one of the objects failed to
  /// [`TwoPhaseCommit::commit`] or [`TwoPhaseCommit::prepare_commit`], this
  /// method is used to roll all other objects back to their previous state.
  fn rollback(&self) -> Result<()>;
}
