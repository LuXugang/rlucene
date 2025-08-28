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
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;

use thiserror::Error;

use crate::util::VersionError;
use crate::util::error::parse::Parse;
use crate::util::error::{
    AlreadyClosedError, ArrayIndexOutOfBoundsError, BufferAllocationError, CorruptIndexError, Eof,
    IllegalArgumentError, IllegalStateError, IndexFormatTooNewError, IndexFormatTooOldError,
    IndexNotFound, LockAlreadyHeldError, LockHeldByOtherError, MaxBytesLengthExceededError,
    MergeAbortedError, MergeError, NeedImplementedError, NoSuchElementError, NotFoundError,
    NotImplementedError, NumberFormatError, NumberOverflow, TooComplexToDeterminizeError,
    UnreachableError, UnsupportedOperationError,
};

#[derive(Debug, Error)]
pub enum LuceneError {
    #[error("parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("IO error: {0}")]
    Io(#[from] Error),
    #[error("conversion failed: {0}")]
    Convert(#[from] TryFromIntError),
    #[error("format error: {0}")]
    Fmt(#[from] fmt::Error),
    #[error("IO error on {path}: {source}")]
    IoWithPath { source: Error, path: String },
    #[error("{0}")]
    BorrowError(String),
    #[error("{0}")]
    LockError(String),
    #[error("UTF-8 conversion error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgumentError),
    #[error("{0}")]
    IllegalState(#[from] IllegalStateError),
    #[error("{0}")]
    Eof(#[from] Eof),
    #[error("{0}")]
    NumberOverflow(#[from] NumberOverflow),
    #[error("{0}")]
    CorruptIndex(#[from] CorruptIndexError),
    #[error("{0}")]
    IndexFormatTooNew(#[from] IndexFormatTooNewError),
    #[error("{0}")]
    IndexFormatTooOld(#[from] IndexFormatTooOldError),
    #[error("{0}")]
    UnsupportedOperation(#[from] UnsupportedOperationError),
    #[error("{0}")]
    NotFound(#[from] NotFoundError),
    #[error("{0}")]
    LockAlreadyHeld(#[from] LockAlreadyHeldError),
    #[error("{0}")]
    LockHeldByOther(#[from] LockHeldByOtherError),
    #[error("{0}")]
    ArrayIndexOutOfBounds(#[from] ArrayIndexOutOfBoundsError),
    #[error("{0}")]
    IndexNotFound(#[from] IndexNotFound),
    #[error("{0}")]
    NumberFormat(#[from] NumberFormatError),
    #[error("{0}")]
    NeedImplemented(#[from] NeedImplementedError),
    #[error("{0}")]
    MaxBytesLengthExceeded(#[from] MaxBytesLengthExceededError),
    #[error("{0}")]
    BufferAllocation(#[from] BufferAllocationError),
    #[error("{0}")]
    Merge(#[from] MergeError),
    #[error("{0}")]
    MergeAborted(#[from] MergeAbortedError),
    #[error("{0}")]
    AlreadyClosed(#[from] AlreadyClosedError),
    #[error("{0}")]
    NotImplemented(#[from] NotImplementedError),
    #[error("{0}")]
    VersionError(#[from] VersionError),
    #[error("{0}")]
    Unreachable(#[from] UnreachableError),
    #[error("{0}")]
    Parse(#[from] Parse),
    #[error("{0}")]
    TooComplexToDeterminize(#[from] TooComplexToDeterminizeError),
    #[error("{0}")]
    NoSuchElement(#[from] NoSuchElementError),
}
macro_rules! error_ctor {
    ($fn_name:ident, $variant:ident, $error_type:ty) => {
        pub fn $fn_name(msg: impl Into<String>) -> Self {
            LuceneError::$variant(<$error_type>::new(msg))
        }
    };
}
impl LuceneError {
    pub fn with_payload<T>(self, payload: T) -> PayloadError<T> {
        PayloadError {
            error: self,
            payload,
        }
    }
    pub fn io_with_path(path: impl Into<String>, err: std::io::Error) -> Self {
        LuceneError::IoWithPath {
            source: err,
            path: path.into(),
        }
    }

    pub fn io(err: std::io::Error) -> Self {
        Self::io_with_path("", err)
    }

    pub fn utf8(err: FromUtf8Error) -> Self {
        LuceneError::FromUtf8Error(err)
    }

    pub fn utf8_error(err: std::str::Utf8Error) -> Self {
        LuceneError::Utf8Error(err)
    }

    error_ctor!(illegal_argument, IllegalArgument, IllegalArgumentError);
    error_ctor!(illegal_state, IllegalState, IllegalStateError);
    error_ctor!(eof, Eof, Eof);
    error_ctor!(number_overflow, NumberOverflow, NumberOverflow);
    error_ctor!(corrupt_index, CorruptIndex, CorruptIndexError);
    error_ctor!(
        index_format_too_new,
        IndexFormatTooNew,
        IndexFormatTooNewError
    );
    error_ctor!(
        index_format_too_old,
        IndexFormatTooOld,
        IndexFormatTooOldError
    );
    error_ctor!(
        unsupported_operation,
        UnsupportedOperation,
        UnsupportedOperationError
    );
    error_ctor!(not_found, NotFound, NotFoundError);
    error_ctor!(lock_already_held, LockAlreadyHeld, LockAlreadyHeldError);
    error_ctor!(lock_held_by_other, LockHeldByOther, LockHeldByOtherError);
    error_ctor!(
        array_index_out_of_bounds,
        ArrayIndexOutOfBounds,
        ArrayIndexOutOfBoundsError
    );
    error_ctor!(index_not_found, IndexNotFound, IndexNotFound);
    error_ctor!(number_format, NumberFormat, NumberFormatError);
    error_ctor!(need_implemented, NeedImplemented, NeedImplementedError);
    error_ctor!(
        max_bytes_length_exceeded,
        MaxBytesLengthExceeded,
        MaxBytesLengthExceededError
    );
    error_ctor!(buffer_allocation, BufferAllocation, BufferAllocationError);
    error_ctor!(merge, Merge, MergeError);
    error_ctor!(merge_abort, MergeAborted, MergeAbortedError);
    error_ctor!(already_closed, AlreadyClosed, AlreadyClosedError);
    error_ctor!(not_implemented, NotImplemented, NotImplementedError);
    error_ctor!(unreachable, Unreachable, UnreachableError);
    error_ctor!(
        too_complex_to_determinize,
        TooComplexToDeterminize,
        TooComplexToDeterminizeError
    );
    error_ctor!(no_such_element, NoSuchElement, NoSuchElementError);
}

pub type Result<T> = core::result::Result<T, LuceneError>;

pub struct PayloadError<T> {
    pub error: LuceneError,
    pub payload: T,
}

impl<T> fmt::Debug for PayloadError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WithPayload")
            .field("error", &self.error)
            .field("payload", &"<omitted>")
            .finish()
    }
}

impl<T: 'static> Display for PayloadError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("WithPayload")
            .field("error", &self.error)
            .field("payload", &"<omitted>")
            .finish()
    }
}
impl<T> PayloadError<T> {
    #[inline]
    pub fn into_parts(self) -> (LuceneError, T) {
        (self.error, self.payload)
    }
    #[inline]
    pub fn into_result<U>(self) -> std::result::Result<U, Self> {
        Err(self)
    }
}

impl<T: 'static> std::error::Error for PayloadError<T> {}
