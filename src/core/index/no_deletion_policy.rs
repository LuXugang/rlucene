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
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_deletion_policy::IndexDeletionPolicy;
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};
/// An [`IndexDeletionPolicy`] which keeps all index commits around and never deletes them.
#[derive(Default)]
pub struct NoDeletionPolicy;

impl Display for NoDeletionPolicy {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl IndexDeletionPolicy for NoDeletionPolicy {
  fn on_init<IC>(&self, _commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
    Ok(())
  }

  fn on_commit<IC>(&self, _commits: &[IC]) -> Result<()>
  where
    IC: IndexCommit + Clone,
  {
    Ok(())
  }
}
