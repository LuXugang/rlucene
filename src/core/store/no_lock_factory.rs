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
use std::fmt::{Display, Formatter};
use std::path::Path;

use crate::core::store::lock::Lock;
use crate::core::store::lock_factory::LockFactory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
/// Use this [`LockFactory`] to disable locking entirely. This is a singleton, you have to use
/// See also [`LockFactory`].
pub struct NoLockFactory;

impl Display for NoLockFactory {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "NoLockFactory")
  }
}

impl LockFactory for NoLockFactory {
  type Lock = NoLock;

  fn obtain_lock(&self, _dir: &Path, _lock_name: &str) -> Result<Self::Lock> {
    Ok(NoLock)
  }
}

pub struct NoLock;
impl Display for NoLock {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "NoLock")
  }
}

impl CloseableRef for NoLock {
  fn close(&self) -> Result<()> {
    Ok(())
  }
}

impl Lock for NoLock {
  fn ensure_valid(&self) -> Result<()> {
    Ok(())
  }
}
