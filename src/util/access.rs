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
use crate::util::error::lucene_error::LuceneError;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

pub trait Access<T>: Clone {
    fn with_shared<F, R>(&self, f: F) -> Result<R, LuceneError>
    where
        F: FnOnce(&T) -> Result<R, LuceneError>;

    fn with_exclusive<F, R>(&self, f: F) -> Result<R, LuceneError>
    where
        F: FnOnce(&mut T) -> Result<R, LuceneError>;
}

impl<T> Access<T> for Rc<RefCell<T>> {
    fn with_shared<F, R>(&self, f: F) -> Result<R, LuceneError>
    where
        F: FnOnce(&T) -> Result<R, LuceneError>,
    {
        let borrow: Ref<T> = self
            .try_borrow()
            .map_err(|e| LuceneError::BorrowError(format!("{:?}", e)))?;
        f(&*borrow)
    }

    fn with_exclusive<F, R>(&self, f: F) -> Result<R, LuceneError>
    where
        F: FnOnce(&mut T) -> Result<R, LuceneError>,
    {
        let mut borrow: RefMut<T> = self
            .try_borrow_mut()
            .map_err(|e| LuceneError::BorrowError(format!("{:?}", e)))?;
        f(&mut *borrow)
    }
}

impl<T> Access<T> for Arc<Mutex<T>> {
    fn with_shared<F, R>(&self, f: F) -> Result<R, LuceneError>
    where
        F: FnOnce(&T) -> Result<R, LuceneError>,
    {
        let guard: MutexGuard<T> = self
            .lock()
            .map_err(|e| LuceneError::LockError(format!("{:?}", e)))?;
        f(&*guard)
    }

    fn with_exclusive<F, R>(&self, f: F) -> Result<R, LuceneError>
    where
        F: FnOnce(&mut T) -> Result<R, LuceneError>,
    {
        let mut guard: MutexGuard<T> = self
            .lock()
            .map_err(|e| LuceneError::LockError(format!("{:?}", e)))?;
        f(&mut *guard)
    }
}
