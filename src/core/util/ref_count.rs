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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::atomic::{AtomicI32, Ordering};
/// Manages reference counting for a given object.
pub struct RefCount<T>
where
    T: Clone,
{
    ref_count: AtomicI32,
    pub(crate) object: T,
}
impl<T> RefCount<T>
where
    T: Clone,
{
    pub fn new(object: T) -> Self {
        Self {
            ref_count: AtomicI32::new(1),
            object,
        }
    }
    /// Decrements the reference counting of this object. When reference counting hits 0, calls #release().
    pub fn dec_ref(&self) -> Result<bool> {
        let rc = self.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;

        if rc == 0 {
            // TODO IMPORTANT 目前没有实现字段的close 但是rust中的自动释放资源能满足我们的close吗
            Ok(true)
        } else if rc < 0 {
            Err(LuceneError::illegal_state(format!(
                "too many decRef calls: refCount is {} after decrement",
                rc
            )))
        } else {
            Ok(false)
        }
    }

    pub fn get(&self) -> &T {
        &self.object
    }
    /// Returns the current reference count.
    pub fn get_ref_count(&self) -> i32 {
        self.ref_count.load(Ordering::SeqCst)
    }
    /// Increments the reference count. Calls to this method must be matched with calls to #decRef().
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }
}
