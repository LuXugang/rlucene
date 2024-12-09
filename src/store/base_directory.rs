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
use crate::store::lock::{FSLockEnum};
use crate::store::lock_factory::LockFactory;
use crate::util::error::data_io_error_enum::DataIOError;

/// Base implementation for a concrete [`Directory`] that uses a [`LockFactory`] for locking.
///
/// # Note
/// This is an experimental API.
pub trait BaseDirectory: Directory {
    fn obtain_lock(
        &mut self,
        lock_name: &str,
        lock_factory: &mut impl LockFactory,
    ) -> Result<FSLockEnum, DataIOError> {
        lock_factory.obtain_lock(self, lock_name)
    }
}
