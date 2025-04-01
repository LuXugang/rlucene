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
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

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
pub trait Shared<T>: Clone + Deref<Target = T> {
    fn new(value: T) -> Self;
}

impl<T> Shared<T> for Rc<T> {
    fn new(value: T) -> Self {
        Rc::new(value)
    }
}

impl<T> Shared<T> for Arc<T> {
    fn new(value: T) -> Self {
        Arc::new(value)
    }
}
