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
use crate::store::lock::Lock;

/**
 * Base class for Locking implementation. `Directory` uses instances of this class to
 * implement locking.
 *
 * Lucene uses `NativeFSLockFactory` by default for `FSDirectory`based index
 * directories.
 *
 * Special care needs to be taken if you change the locking implementation: First be certain that
 * no writer is in fact writing to the index otherwise you can easily corrupt your index. Be sure to
 * do the LockFactory change on all Lucene instances and clean up all leftover lock files before
 * starting the new configuration for the first time. Different implementations can not work
 * together!
 *
 * If you suspect that some LockFactory implementation is not working properly in your
 * environment, you can easily test it by using `VerifyingLockFactory`,`LockVerifyServer` and `LockStressTest`.
 *
 */
pub trait LockFactory {
    /**
    * Return a new obtained Lock instance identified by lockName.
    *
    */
    fn obtain_lock(&self, lock_name: &str) -> impl Lock;
}