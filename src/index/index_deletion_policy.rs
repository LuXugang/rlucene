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
use crate::index::index_commit::IndexCommit;
use crate::util::error::lucene_error::Result;
use std::fmt::Display;
/// This [`IndexDeletionPolicy`] implementation keeps only the most recent commit and
/// immediately removes all prior commits after a new commit is done. This is the default deletion
/// policy.
pub trait IndexDeletionPolicy: Display {
    /// Deletes all commits except the most recent one.
    fn on_init<IC>(&mut self, commits: &mut [IC]) -> Result<()>
    where
        IC: IndexCommit;

    /// Deletes all commits except the most recent one.
    fn on_commit<IC>(&mut self, commits: &mut [IC]) -> Result<()>
    where
        IC: IndexCommit;
}
