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
use crate::util::array_util::ArrayUtil;
use crate::util::SliceCopyOps;
use parking_lot::{Mutex, MutexGuard};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

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
/// use rlucene::util::error::lucene_error::{LuceneError, Result};
/// struct MyStruct;
/// impl MyStruct{
///    fn do_something(&self) {
///     }
/// }
///
/// fn update_state<S: Access<MyStruct>>(state: &mut S) -> Result<()> {
///     state.access_mut(|s| {
///         s.do_something();
///        // Help the compiler infer types.
///         Ok::<(), LuceneError>(())
///     })?;
///     state.access(|s| {
///         s.do_something();
///        // Help the compiler infer types.
///         Ok::<(), LuceneError>(())
///     })
/// }
/// ```
///
/// Then depending on your runtime context:
/// - `Rc<RefCell<MyStruct>>` can implement `Access<MyStruct>` for local use
/// - `Arc<Mutex<MyStruct>>` can implement `Access<MyStruct>` for concurrent use
pub trait Access<T>: Clone {
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R;

    fn access_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;
}

impl<T> Access<T> for Rc<RefCell<T>> {
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let borrow = self.borrow();
        f(&*borrow)
    }

    fn access_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut borrow = self.borrow_mut();
        f(&mut *borrow)
    }
}

impl<T> Access<T> for Arc<Mutex<T>> {
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard: MutexGuard<'_, T> = self.lock();
        f(&*guard)
    }

    fn access_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard: MutexGuard<'_, T> = self.lock();
        f(&mut *guard)
    }
}

/// Similar to the `Access` trait, but specifically for `Vec<T>`.
pub trait AccessVec<T>: Clone + Default {
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R;
    fn access_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R;
    fn slice_clone(&self, offset: usize, length: usize) -> Self;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn new() -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_vec(v: Vec<T>) -> Self;
    fn copy(&mut self, src: &[T], offset: usize);
}

impl<T> AccessVec<T> for Vec<T>
where
    T: Clone + Default,
{
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R,
    {
        f(self)
    }

    fn access_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R,
    {
        f(self)
    }

    fn slice_clone(&self, offset: usize, length: usize) -> Self {
        ArrayUtil::copy_of_sub_array(self, offset, offset + length)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn new() -> Self {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn from_vec(v: Vec<T>) -> Self {
        v
    }

    fn copy(&mut self, src: &[T], offset: usize) {
        self.copy_from(src, offset)
    }
}

impl<T> AccessVec<T> for Rc<RefCell<Vec<T>>>
where
    T: Clone + Default,
{
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R,
    {
        let borrow = self.borrow();
        f(&*borrow)
    }

    fn access_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R,
    {
        let mut borrow = self.borrow_mut();
        f(&mut *borrow)
    }

    fn slice_clone(&self, offset: usize, length: usize) -> Self {
        let borrow = &*self.borrow();
        Rc::new(RefCell::new(ArrayUtil::copy_of_sub_array(
            borrow,
            offset,
            offset + length,
        )))
    }

    fn len(&self) -> usize {
        self.borrow().len()
    }

    fn is_empty(&self) -> bool {
        self.borrow().is_empty()
    }

    fn new() -> Self {
        Rc::new(RefCell::new(Vec::new()))
    }

    fn with_capacity(capacity: usize) -> Self {
        Rc::new(RefCell::new(vec![T::default(); capacity]))
    }

    fn from_vec(v: Vec<T>) -> Self {
        Rc::new(RefCell::new(v))
    }

    fn copy(&mut self, src: &[T], offset: usize) {
        self.borrow_mut().copy_from(src, offset)
    }
}

impl<T> AccessVec<T> for Arc<Mutex<Vec<T>>>
where
    T: Clone + Default,
{
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R,
    {
        let guard: MutexGuard<'_, Vec<T>> = self.lock();
        f(&*guard)
    }

    fn access_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R,
    {
        let mut guard: MutexGuard<'_, Vec<T>> = self.lock();
        f(&mut *guard)
    }

    fn slice_clone(&self, offset: usize, length: usize) -> Self {
        let bytes = self.lock();
        Arc::new(Mutex::new(ArrayUtil::copy_of_sub_array(
            &bytes,
            offset,
            offset + length,
        )))
    }

    fn len(&self) -> usize {
        self.lock().len()
    }

    fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn new() -> Self {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn with_capacity(capacity: usize) -> Self {
        Arc::new(Mutex::new(vec![T::default(); capacity]))
    }

    fn from_vec(v: Vec<T>) -> Self {
        Arc::new(Mutex::new(v))
    }

    fn copy(&mut self, src: &[T], offset: usize) {
        self.lock().copy_from(src, offset)
    }
}

#[macro_export]
macro_rules! with_other {
    (mut, $x:expr, $y:expr, |$ia:ident, $ib:ident| $body:expr) => {
        $x.access_mut(|$ia| $y.access(|$ib| $body))
    };
    ($x:expr, $y:expr, |$ia:ident, $ib:ident| $body:expr) => {
        $x.access(|$ia| $y.access(|$ib| $body))
    };
}
