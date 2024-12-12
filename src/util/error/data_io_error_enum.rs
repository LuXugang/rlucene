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
use crate::util::error::corrupt_index::CorruptIndexError;
use crate::util::error::eof::Eof;
use crate::util::error::illegal_argument::IllegalArgumentError;
use crate::util::error::illegal_state::IllegalStateError;
use crate::util::error::index_format_too_new::IndexFormatTooNewError;
use crate::util::error::index_format_too_old::IndexFormatTooOldError;
use crate::util::error::integer_overflow::IntegerOverflow;
use crate::util::error::lock_already_held::LockAlreadyHeldError;
use crate::util::error::lock_held_by_other::LockHeldByOtherError;
use crate::util::error::not_found::NotFoundError;
use crate::util::error::runtime_error::RuntimeError;
use crate::util::error::unsupported_operation::UnsupportedOperationError;
use std::io::Error;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataIOError {
    #[error("IO error: {0}")]
    Io(#[from] Error),
    
    #[error("IO error on {path}: {source}")]
    IoWithPath {
        source: Error,
        path: String   
    },

    #[error("UTF-8 conversion error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgumentError),

    #[error("{0}")]
    IllegalState(#[from] IllegalStateError),

    #[error("{0}")]
    Eof(#[from] Eof),

    #[error("{0}")]
    IntegerOverflow(#[from] IntegerOverflow),

    #[error("{0}")]
    CorruptIndex(#[from] CorruptIndexError),

    #[error("{0}")]
    IndexFormatTooNew(#[from] IndexFormatTooNewError),

    #[error("{0}")]
    IndexFormatTooOld(#[from] IndexFormatTooOldError),

    #[error("{0}")]
    UnsupportedOperation(#[from] UnsupportedOperationError),

    #[error("{0}")]
    RuntimeError(#[from] RuntimeError),

    #[error("{0}")]
    NotFound(#[from] NotFoundError),

    #[error("{0}")]
    LockAlreadyHeld(#[from] LockAlreadyHeldError),

    #[error("{0}")]
    LockHeldByOther(#[from] LockHeldByOtherError),
}
impl DataIOError {
    pub fn io_with_path(path: impl Into<String>, err: std::io::Error) -> Self {
        DataIOError::IoWithPath {
            source: err,
            path: path.into(),
        }
    }
    pub fn io(err: std::io::Error) -> Self {
        Self::io_with_path("", err)
    }

    pub fn utf8(err: FromUtf8Error) -> Self {
        DataIOError::FromUtf8Error(err)
    }
    pub fn illegal_argument(msg: impl Into<String>) -> Self {
        DataIOError::IllegalArgument(IllegalArgumentError::new(msg))
    }
    pub fn illegal_state(msg: impl Into<String>) -> Self {
        DataIOError::IllegalState(IllegalStateError::new(msg))
    }

    pub fn eof(msg: impl Into<String>) -> Self {
        DataIOError::Eof(Eof::new(msg))
    }

    pub fn integer_overflow(msg: impl Into<String>) -> Self {
        DataIOError::IntegerOverflow(IntegerOverflow::new(msg))
    }

    pub fn corrupt_index(msg: impl Into<String>) -> Self {
        DataIOError::CorruptIndex(CorruptIndexError::new(msg))
    }

    pub fn index_format_too_new(msg: impl Into<String>) -> Self {
        DataIOError::IndexFormatTooNew(IndexFormatTooNewError::new(msg))
    }
    pub fn index_format_too_old(msg: impl Into<String>) -> Self {
        DataIOError::IndexFormatTooOld(IndexFormatTooOldError::new(msg))
    }

    pub fn unsupported_operation(msg: impl Into<String>) -> Self {
        DataIOError::UnsupportedOperation(UnsupportedOperationError::new(msg))
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        DataIOError::NotFound(NotFoundError::new(msg))
    }
    pub fn lock_already_held(msg: impl Into<String>) -> Self {
        DataIOError::LockAlreadyHeld(LockAlreadyHeldError::new(msg))
    }
    pub fn lock_held_by_other(msg: impl Into<String>) -> Self {
        DataIOError::LockHeldByOther(LockHeldByOtherError::new(msg))
    }
}
