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
use crate::core::store::directory::Directory;
use crate::core::store::lock_factory::LockFactory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

/// Base implementation for a concrete [`Directory`] that uses a
/// [`LockFactory`] for locking.
///
/// # Note
/// This is an experimental API.
///
/// # Special Note
/// This trait could actually be removed because `LockFactory` has been moved to
/// the implementation of `Directory`,
/// such as [`FSDirectory`](crate::core::store::fs_directory::FSDirectory). However,
/// it is temporarily retained to maintain consistency with the structure of
/// Java Lucene.
pub trait BaseDirectory: Directory {
    type LockFactory: LockFactory<Lock = Self::Lock>;
    fn get_lock_factory(&self) -> &BaseDirectoryBase<Self::LockFactory>;
}
pub struct BaseDirectoryBase<LF> {
    pub(crate) lock_factory: LF,
    is_open: AtomicBool,
}
impl<LF> Display for BaseDirectoryBase<LF>
where
    LF: LockFactory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BaseDirectoryBase with LockFactory:{}",
            self.lock_factory
        )
    }
}
impl<LF> BaseDirectoryBase<LF>
where
    LF: LockFactory,
{
    pub fn new(lock_factory: LF) -> Self {
        Self {
            lock_factory,
            is_open: AtomicBool::new(false),
        }
    }
    pub fn obtain_lock(&self, dir: &Path, name: &str) -> Result<LF::Lock> {
        self.lock_factory.obtain_lock(dir, name)
    }
    pub fn ensure_open(&self) -> Result<()> {
        if !self.is_open.load(SeqCst) {
            return Err(LuceneError::already_closed("this Directory is closed"));
        }
        Ok(())
    }
}
