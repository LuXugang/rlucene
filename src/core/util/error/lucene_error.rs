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
use std::string::FromUtf8Error;

use thiserror::Error;

use crate::core::util::VersionError;
use crate::core::util::error::parse::Parse;
use crate::core::util::error::{AlreadyClosedError, ArrayIndexOutOfBoundsError, BufferAllocationError, CollectionTerminatedError, CorruptIndexError, Eof, FuzzyTermsError, IllegalArgumentError, IllegalStateError, IndexFormatTooNewError, IndexFormatTooOldError, IndexNotFound, LockAlreadyHeldError, LockHeldByOtherError, LockObtainFailedError, MaxBytesLengthExceededError, MergeAbortedError, MergeError, NeedImplementedError, NoMoreTermsError, NoSuchElementError, NotFoundError, NotImplementedError, NumberFormatError, NumberOverflow, TimeExceededError, TooComplexToDeterminizeError, TooManyClausesError, TooManyNestedClausesError, UncheckedIOError, UnreachableError, UnsupportedOperationError};

#[derive(Debug, Error)]
pub enum LuceneError {
  #[error("parse int error: {0}")]
  ParseIntError(#[from] std::num::ParseIntError),
  #[error("IO error: {0}")]
  Io(#[from] Error),
  #[error("conversion failed: {0}")]
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
  #[error("{0}")]
  UncheckedIO(#[from] UncheckedIOError),
  #[error("{0}")]
  CollectionTerminated(#[from] CollectionTerminatedError),
  #[error("{0}")]
  TooManyClauses(#[from] TooManyClausesError),
  #[error("{0}")]
  TooManyNestedClauses(#[from] TooManyNestedClausesError),
  #[error("{0}")]
  TimeExceeded(#[from] TimeExceededError),
  #[error("{0}")]
  LockObtainFailed(#[from] LockObtainFailedError),
  #[error("{0}")]
  NoMoreTerms(#[from] NoMoreTermsError),
  #[error("{0}")]
  FuzzyTerms(#[from] FuzzyTermsError),
}
macro_rules! error_ctor {
  ($fn_name:ident, $fn_name_with_source:ident, $variant:ident, $error_type:ident) => {
    pub fn $fn_name(err: impl Into<$error_type>) -> Self {
      LuceneError::$variant(err.into())
    }
    pub fn $fn_name_with_source(msg: impl Into<String>, source: LuceneError) -> Self {
      let source_error = format!("{}:( supper error: ({}))", msg.into(), source);
      let err = $error_type {
        message: source_error,
        source: Some(Box::new(source)),
      };
      LuceneError::$variant(err)
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

  error_ctor!(
    illegal_argument,
    illegal_argument_with_source,
    IllegalArgument,
    IllegalArgumentError
  );
  error_ctor!(
    illegal_state,
    illegal_state_with_source,
    IllegalState,
    IllegalStateError
  );
  error_ctor!(eof, eof_with_source, Eof, Eof);
  error_ctor!(
    number_overflow,
    number_overflow_with_source,
    NumberOverflow,
    NumberOverflow
  );
  error_ctor!(
    corrupt_index,
    corrupt_index_with_source,
    CorruptIndex,
    CorruptIndexError
  );
  error_ctor!(
    index_format_too_new,
    index_format_too_with_source,
    IndexFormatTooNew,
    IndexFormatTooNewError
  );
  error_ctor!(
    index_format_too_old,
    index_format_too_old_with_source,
    IndexFormatTooOld,
    IndexFormatTooOldError
  );
  error_ctor!(
    unsupported_operation,
    unsupported_operation_with_source,
    UnsupportedOperation,
    UnsupportedOperationError
  );
  error_ctor!(not_found, not_found_with_source, NotFound, NotFoundError);
  error_ctor!(
    lock_already_held,
    lock_already_held_with_source,
    LockAlreadyHeld,
    LockAlreadyHeldError
  );
  error_ctor!(
    lock_held_by_other,
    lock_held_by_other_with_source,
    LockHeldByOther,
    LockHeldByOtherError
  );
  error_ctor!(
    array_index_out_of_bounds,
    array_index_out_of_bounds_with_source,
    ArrayIndexOutOfBounds,
    ArrayIndexOutOfBoundsError
  );
  error_ctor!(
    index_not_found,
    index_not_found_with_source,
    IndexNotFound,
    IndexNotFound
  );
  error_ctor!(
    number_format,
    number_format_with_source,
    NumberFormat,
    NumberFormatError
  );
  error_ctor!(
    need_implemented,
    need_implemented_with_source,
    NeedImplemented,
    NeedImplementedError
  );
  error_ctor!(
    max_bytes_length_exceeded,
    max_bytes_length_exceeded_with_source,
    MaxBytesLengthExceeded,
    MaxBytesLengthExceededError
  );
  error_ctor!(
    buffer_allocation,
    buffer_allocation_with_source,
    BufferAllocation,
    BufferAllocationError
  );
  error_ctor!(merge, merge_with_source, Merge, MergeError);
  error_ctor!(
    merge_abort,
    merge_abort_with_source,
    MergeAborted,
    MergeAbortedError
  );
  error_ctor!(
    already_closed,
    already_closed_with_source,
    AlreadyClosed,
    AlreadyClosedError
  );
  error_ctor!(
    not_implemented,
    not_implemented_with_source,
    NotImplemented,
    NotImplementedError
  );
  error_ctor!(
    unreachable,
    unreachable_with_source,
    Unreachable,
    UnreachableError
  );
  error_ctor!(
    too_complex_to_determinize,
    too_complex_to_determinize_with_source,
    TooComplexToDeterminize,
    TooComplexToDeterminizeError
  );
  error_ctor!(
    no_such_element,
    no_such_element_with_source,
    NoSuchElement,
    NoSuchElementError
  );
  error_ctor!(
    unchecked_io_error,
    unchecked_io_error_with_source,
    UncheckedIO,
    UncheckedIOError
  );
  error_ctor!(
    collection_terminated,
    collection_terminated_with_source,
    CollectionTerminated,
    CollectionTerminatedError
  );
  error_ctor!(
    too_many_clauses,
    too_many_clauses_with_source,
    TooManyClauses,
    TooManyClausesError
  );
  error_ctor!(
    too_many_nested_clauses,
    too_many_nested_clauses_with_source,
    TooManyNestedClauses,
    TooManyNestedClausesError
  );
  error_ctor!(
    time_exceeded,
    time_exceeded_with_source,
    TimeExceeded,
    TimeExceededError
  );
  error_ctor!(
    lock_obtain_failed,
    lock_obtain_failed_with_source,
    LockObtainFailed,
    LockObtainFailedError
  );
  error_ctor!(
    no_more_terms,
    no_more_terms_with_source,
    NoMoreTerms,
    NoMoreTermsError
  );
  error_ctor!(
    fuzzy_terms,
    fuzzy_terms_error_source,
    FuzzyTerms,
    FuzzyTermsError
  );
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::error::Error;

  #[test]
  fn wrap_lucene_error() {
    let inner = LuceneError::illegal_argument("inner error");
    let outer = LuceneError::illegal_state(inner);
    let source = outer.source().expect("not fail").to_string();
    assert_eq!(source, "inner error");
  }

  #[test]
  fn wrap_with_message_and_source() {
    let inner = LuceneError::illegal_argument("inner error");
    let outer = LuceneError::illegal_state_with_source("outer error", inner);
    assert_eq!(
      outer.to_string(),
      "outer error:( supper error: (inner error))"
    );
    let source = outer
      .source()
      .expect("not fail")
      .source()
      .expect("not fail")
      .to_string();
    assert_eq!(source, "inner error");
  }
}
