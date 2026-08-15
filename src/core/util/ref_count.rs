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
pub struct RefCount<T> {
  ref_count: AtomicI32,
  pub(crate) object: T,
}
impl<T> RefCount<T> {
  pub fn new(object: T) -> Self {
    Self {
      ref_count: AtomicI32::new(1),
      object,
    }
  }
  /// Decrements the reference count. Calls `release` when it reaches zero and restores the
  /// reference count if `release` fails.
  pub fn dec_ref<F>(&self, release: F) -> Result<bool>
  where
    F: FnOnce() -> Result<()>,
  {
    let rc = self.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;

    if rc == 0 {
      let mut success = false;
      let release_result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
          release()?;
          success = true;
          Ok(())
        }));
      if !success {
        // Put reference back on failure
        self.ref_count.fetch_add(1, Ordering::SeqCst);
      }
      unwrap_caught_result!(release_result)?;
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
  /// Increments the reference count. Each call must be matched by [`Self::dec_ref`].
  pub fn inc_ref(&self) {
    self.ref_count.fetch_add(1, Ordering::SeqCst);
  }
}
