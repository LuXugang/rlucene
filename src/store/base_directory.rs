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
use crate::store::directory::Directory;
use crate::store::lock::Lock;
use crate::util::error::data_io_error_enum::RuntimeError;

/// Base implementation for a concrete [`Directory`] that uses a [`LockFactory`](crate::store::lock_factory::LockFactory) for locking.
///
/// # Note
/// This is an experimental API.
///
/// # Special Note
/// This trait could actually be removed because `LockFactory` has been moved to the implementation of `Directory`,
/// such as [`FSDirectory`](crate::store::fs_directory::FSDirectory). However, it is temporarily retained to maintain consistency with the structure of Java Lucene.
pub trait BaseDirectory: Directory {
    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock, RuntimeError>;
}
