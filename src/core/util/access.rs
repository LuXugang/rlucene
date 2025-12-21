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
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::core::util::SliceCopyOps;
use crate::core::util::array_util::ArrayUtil;

pub trait SharedReadOnly<T>: Clone {
    fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R;
}

impl<T> SharedReadOnly<T> for Rc<T> {
    fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(self.as_ref())
    }
}

impl<T> SharedReadOnly<T> for Arc<T> {
    fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(self.as_ref())
    }
}

impl<T> SharedReadOnly<T> for &T {
    fn access<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(self)
    }
}

/// Similar to the `Access` trait, but specifically for `Vec<T>`.
pub trait SharedAccessVec<T>: Clone + Default {
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R;
    fn access_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R;
    fn slice_clone(&self, offset: usize, length: usize) -> Self;
    fn new() -> Self;
    fn with_capacity(capacity: usize) -> Self;
    fn from_vec(v: Vec<T>) -> Self;
    fn copy(&mut self, src: &[T], offset: usize);
}

// Vec<T>
impl<T> SharedAccessVec<T> for Vec<T>
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

    fn new() -> Self {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self {
        vec![T::default(); capacity]
    }

    fn from_vec(v: Vec<T>) -> Self {
        v
    }

    fn copy(&mut self, src: &[T], offset: usize) {
        self.copy_from(src, offset)
    }
}
// Rc<Vec<T>>
impl<T> SharedAccessVec<T> for Rc<Vec<T>>
where
    T: Clone + Default,
{
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R,
    {
        f(self)
    }

    fn access_mut<F, R>(&mut self, _f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R,
    {
        panic!("access_mut is not supported for Rc<Vec<T>>");
    }

    fn slice_clone(&self, offset: usize, length: usize) -> Self {
        let slice = &self[offset..offset + length];
        Rc::new(slice.to_vec())
    }

    fn new() -> Self {
        Rc::new(Vec::new())
    }

    fn with_capacity(_capacity: usize) -> Self {
        Rc::new(Vec::new()) // Rc<Vec<T>> can't preallocate meaningfully
    }

    fn from_vec(v: Vec<T>) -> Self {
        Rc::new(v)
    }

    fn copy(&mut self, _src: &[T], _offset: usize) {
        panic!("copy is not supported for Rc<Vec<T>>");
    }
}
// Arc<Vec<T>>
impl<T> SharedAccessVec<T> for Arc<Vec<T>>
where
    T: Clone + Default,
{
    fn access<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Vec<T>) -> R,
    {
        f(self)
    }

    fn access_mut<F, R>(&mut self, _f: F) -> R
    where
        F: FnOnce(&mut Vec<T>) -> R,
    {
        panic!("access_mut is not supported for Rc<Vec<T>>");
    }

    fn slice_clone(&self, offset: usize, length: usize) -> Self {
        let slice = &self[offset..offset + length];
        Arc::new(slice.to_vec())
    }

    fn new() -> Self {
        Arc::new(Vec::new())
    }

    fn with_capacity(_capacity: usize) -> Self {
        Arc::new(Vec::new()) // Rc<Vec<T>> can't preallocate meaningfully
    }

    fn from_vec(v: Vec<T>) -> Self {
        Arc::new(v)
    }

    fn copy(&mut self, _src: &[T], _offset: usize) {
        panic!("copy is not supported for Rc<Vec<T>>");
    }
}

#[macro_export]
macro_rules! with_other {
    (mut, $x:expr_2021, $y:expr_2021, |$ia:ident, $ib:ident| $body:expr_2021) => {
        $x.access_mut(|$ia| $y.access(|$ib| $body))
    };
    ($x:expr_2021, $y:expr_2021, |$ia:ident, $ib:ident| $body:expr_2021) => {
        $x.access(|$ia| $y.access(|$ib| $body))
    };
}
/// A trait for ergonomic, read/write access to `Rc<RefCell<T>>` and variants.
///
/// `access()` returns a shared borrow (`Ref<T>`);
/// `access_mut()` returns a mutable borrow (`RefMut<'_,T>`).
///
/// # Panics
///
/// If implemented on `Option<Rc<RefCell<T>>>`, the value must be `Some`.
/// Calling `.access()` or `.access_mut()` on `None` will panic.
pub trait BorrowExt<T> {
    fn access(&self) -> Ref<'_, T>;
    fn access_mut(&self) -> RefMut<'_, T>;
}
/// Implementation for `Option<Rc<RefCell<T>>>`
///
/// # Panics
///
/// Will panic if the `Option` is `None`.
impl<T> BorrowExt<T> for Option<Rc<RefCell<T>>> {
    #[inline]
    fn access(&self) -> Ref<'_, T> {
        self.as_ref().unwrap().borrow()
    }

    #[inline]
    fn access_mut(&self) -> RefMut<'_, T> {
        self.as_ref().unwrap().borrow_mut()
    }
}
/// A trait for ergonomic, read/write access to `Arc<Mutex<T>>` and variants.
///
/// `access()` acquires a lock and returns a guard (`MutexGuard<T>`).
/// `access_mut()` is equivalent to `access()` (alias).
///
/// # Panics
///
/// If implemented on `Option<Arc<Mutex<T>>>`, the value must be `Some`.
/// Calling `.access()` or `.access_mut()` on `None` will panic.
pub trait MutexAccess<T> {
    fn access(&self) -> MutexGuard<'_, T>;
    fn access_mut(&self) -> MutexGuard<'_, T>;
}
/// Implementation for `Option<Arc<Mutex<T>>>`
///
/// # Panics
///
/// Will panic if the `Option` is `None`.
impl<T> MutexAccess<T> for Option<Arc<Mutex<T>>> {
    #[inline]
    fn access(&self) -> MutexGuard<'_, T> {
        self.as_ref().unwrap().lock()
    }

    #[inline]
    fn access_mut(&self) -> MutexGuard<'_, T> {
        self.as_ref().unwrap().lock()
    }
}
