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
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::SliceCopyOps;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

/// Provides a unified interface for accessing shared data, abstracting over
/// single-threaded (`Rc<RefCell<T>>`) and multi-threaded (`Arc<Mutex<T>>`) containers.
///
/// This trait allows the caller to interact with the wrapped value through immutable or mutable
/// closures, without needing to know whether the underlying implementation uses `RefCell` or `Mutex`.
///
/// ### Use Case
/// This trait is especially useful when you have components or fields that may be used in either
/// single-threaded or multi-threaded contexts. By defining them in terms of `Access<T>`, you can:
///
/// - Use `Rc<RefCell<T>>` in single-threaded mode for better performance (no locking).
/// - Use `Arc<Mutex<T>>` in multi-threaded mode for thread-safe access.
/// - Write common logic that doesn’t care about the underlying synchronization strategy.
///
/// ### Example
/// ```rust
/// use rlucene::util::access::Access;
/// use rlucene::util::error::lucene_error::Result;
/// struct MyStruct;
/// impl MyStruct{
///    fn do_something(&self) {
///     }
/// }
///
/// fn update_state<S: Access<MyStruct>>(state: &mut S) -> Result<()> {
///     state.access_mut(|s| {
///         s.do_something();
///         Ok(())
///     })?;
///     state.access(|s| {
///         s.do_something();
///         Ok(())
///     })
/// }
/// ```
///
/// Then depending on your runtime context:
/// - `Rc<RefCell<MyStruct>>` can implement `Access<MyStruct>` for local use
/// - `Arc<Mutex<MyStruct>>` can implement `Access<MyStruct>` for concurrent use
pub trait Access<T>: Clone {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> Result<R>;

    fn access_mut<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> Result<R>;
}

impl<T> Access<T> for Rc<RefCell<T>> {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> Result<R>,
    {
        let borrow = self.borrow();
        f(&*borrow)
    }

    fn access_mut<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> Result<R>,
    {
        let mut borrow = self.borrow_mut();
        f(&mut *borrow)
    }
}

impl<T> Access<T> for Arc<Mutex<T>> {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&T) -> Result<R>,
    {
        let guard: MutexGuard<T> = self
            .lock()
            .map_err(|e| LuceneError::LockError(format!("{:?}", e)))?;
        f(&*guard)
    }

    fn access_mut<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> Result<R>,
    {
        let mut guard: MutexGuard<T> = self
            .lock()
            .map_err(|e| LuceneError::LockError(format!("{:?}", e)))?;
        f(&mut *guard)
    }
}

/// Similar to the `Access` trait, but specifically for `Vec<T>`.
pub trait AccessVec<T>: Clone {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Vec<T>) -> Result<R>;

    fn access_mut<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Vec<T>) -> Result<R>;
    fn slice_clone(&self, offset: usize, length: usize) -> Result<Self>;
}

impl<T: Clone> AccessVec<T> for Vec<T> {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Vec<T>) -> Result<R>,
    {
        f(self)
    }

    fn access_mut<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Vec<T>) -> Result<R>,
    {
        f(self)
    }
    fn slice_clone(&self, offset: usize, length: usize) -> Result<Self> {
        let mut sub = Vec::with_capacity(length);
        sub.copy_from(&self[offset..offset + length], 0);
        Ok(sub)
    }
}

impl<T: Clone> AccessVec<T> for Rc<RefCell<Vec<T>>> {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Vec<T>) -> Result<R>,
    {
        let borrow = self.borrow();
        f(&*borrow)
    }

    fn access_mut<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Vec<T>) -> Result<R>,
    {
        let mut borrow = self.borrow_mut();
        f(&mut *borrow)
    }

    fn slice_clone(&self, offset: usize, length: usize) -> Result<Self> {
        let mut sub = Vec::with_capacity(length);
        sub.copy_from(&self.borrow()[offset..offset + length], 0);
        let new_vec = Rc::new(RefCell::new(sub));
        Ok(new_vec)
    }
}
impl<T: Clone> AccessVec<T> for Arc<Mutex<Vec<T>>> {
    fn access<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&Vec<T>) -> Result<R>,
    {
        let guard: MutexGuard<Vec<T>> = self
            .lock()
            .map_err(|e| LuceneError::LockError(format!("{:?}", e)))?;
        f(&*guard)
    }

    fn access_mut<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Vec<T>) -> Result<R>,
    {
        let mut guard: MutexGuard<Vec<T>> = self
            .lock()
            .map_err(|e| LuceneError::LockError(format!("{:?}", e)))?;
        f(&mut *guard)
    }
    fn slice_clone(&self, offset: usize, length: usize) -> Result<Self> {
        let mut sub = Vec::with_capacity(length);
        sub.copy_from(
            &self
                .lock()
                .map_err(|e| LuceneError::LockError(format!("Mutex poisoned: {:?}", e)))?
                [offset..offset + length],
            0,
        );
        let new_vec = Arc::new(Mutex::new(sub));
        Ok(new_vec)
    }
}
