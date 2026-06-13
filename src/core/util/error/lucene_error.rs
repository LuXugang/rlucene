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
use std::any::Any;
use std::fmt;
use std::io::Error;
use std::string::FromUtf8Error;

use thiserror::Error;

use crate::core::util::VersionError;
use crate::core::util::error::parse::Parse;
use crate::core::util::error::{
  AlreadyClosedError, ArrayIndexOutOfBoundsError, BufferAllocationError, CollectionTerminatedError,
  CorruptIndexError, Eof, FuzzyTermsError, IllegalArgumentError, IllegalStateError,
  IndexFormatTooNewError, IndexFormatTooOldError, IndexNotFound, LockAlreadyHeldError,
  LockHeldByOtherError, LockObtainFailedError, LockReleaseFailedError, MaxBytesLengthExceededError,
  MergeAbortedError, MergeError, NeedImplementedError, NoMoreTermsError, NoSuchElementError,
  NotImplementedError, NotSuchFileError, NumberFormatError, NumberOverflow, TimeExceededError,
  TooComplexToDeterminizeError, TooManyClausesError, TooManyNestedClausesError, TragedyError,
  UncheckedIOError, UnreachableError, UnsupportedOperationError,
};

#[derive(Debug, Error)]
pub enum LuceneError {
  #[error("{0}")]
  AlreadyClosed(#[from] AlreadyClosedError),
  #[error("{0}")]
  ArrayIndexOutOfBounds(#[from] ArrayIndexOutOfBoundsError),
  #[error("{0}")]
  BorrowError(String),
  #[error("{0}")]
  BufferAllocation(#[from] BufferAllocationError),
  #[error("{0}")]
  CollectionTerminated(#[from] CollectionTerminatedError),
  #[error("{0}")]
  CorruptIndex(#[from] CorruptIndexError),
  #[error("{0}")]
  Eof(#[from] Eof),
  #[error("conversion failed: {0}")]
  Fmt(#[from] fmt::Error),
  #[error("UTF-8 conversion error: {0}")]
  FromUtf8Error(#[from] FromUtf8Error),
  #[error("{0}")]
  FuzzyTerms(#[from] FuzzyTermsError),
  #[error("{0}")]
  IllegalArgument(#[from] IllegalArgumentError),
  #[error("{0}")]
  IllegalState(#[from] IllegalStateError),
  #[error("{0}")]
  IndexFormatTooNew(#[from] IndexFormatTooNewError),
  #[error("{0}")]
  IndexFormatTooOld(#[from] IndexFormatTooOldError),
  #[error("{0}")]
  IndexNotFound(#[from] IndexNotFound),
  #[error("IO error: {0}")]
  Io(#[from] Error),
  #[error("IO error on {path}: {source}, {err_kind}")]
  IoWithPath {
    source: Error,
    path: String,
    err_kind: String,
  },
  #[error("{0}")]
  LockAlreadyHeld(#[from] LockAlreadyHeldError),
  #[error("{0}")]
  LockError(String),
  #[error("{0}")]
  LockHeldByOther(#[from] LockHeldByOtherError),
  #[error("{0}")]
  LockObtainFailed(#[from] LockObtainFailedError),
  #[error("{0}")]
  LockReleaseFailed(#[from] LockReleaseFailedError),
  #[error("{0}")]
  MaxBytesLengthExceeded(#[from] MaxBytesLengthExceededError),
  #[error("{0}")]
  Merge(#[from] MergeError),
  #[error("{0}")]
  MergeAborted(#[from] MergeAbortedError),
  #[error("{0}")]
  NeedImplemented(#[from] NeedImplementedError),
  #[error("{0}")]
  NoMoreTerms(#[from] NoMoreTermsError),
  #[error("{0}")]
  NoSuchElement(#[from] NoSuchElementError),
  #[error("{0}")]
  NoSuchFile(#[from] NotSuchFileError),
  #[error("{0}")]
  NotImplemented(#[from] NotImplementedError),
  #[error("{0}")]
  NumberFormat(#[from] NumberFormatError),
  #[error("{0}")]
  NumberOverflow(#[from] NumberOverflow),
  #[error("{0}")]
  Parse(#[from] Parse),
  #[error("parse int error: {0}")]
  ParseIntError(#[from] std::num::ParseIntError),
  #[error("{0}")]
  TimeExceeded(#[from] TimeExceededError),
  #[error("{0}")]
  TooComplexToDeterminize(#[from] TooComplexToDeterminizeError),
  #[error("{0}")]
  TooManyClauses(#[from] TooManyClausesError),
  #[error("{0}")]
  TooManyNestedClauses(#[from] TooManyNestedClausesError),
  #[error("{0}")]
  Tragedy(#[from] TragedyError),
  #[error("{0}")]
  UncheckedIO(#[from] UncheckedIOError),
  #[error("{0}")]
  Unreachable(#[from] UnreachableError),
  #[error("{0}")]
  UnsupportedOperation(#[from] UnsupportedOperationError),
  #[error("UTF-8 decoding error: {0}")]
  Utf8Error(#[from] std::str::Utf8Error),
  #[error("{0}")]
  VersionError(#[from] VersionError),
}
macro_rules! error_ctor {
  (@add_suppressed $(($variant:ident)),+ $(,)?) => {
    pub fn add_suppressed(&mut self, source: LuceneError) -> Result<()> {
      match self {
        $(
          LuceneError::$variant(err) => {
            err.add_suppressed(source);
            Ok(())
          },
        )+
        _ => Err(LuceneError::unsupported_operation(
          "add_suppressed is not supported for this error",
        )),
      }
    }

    pub fn get_suppressed(&self) -> Result<Option<&LuceneError>> {
      match self {
        $(
          LuceneError::$variant(err) => Ok(err.get_suppressed()),
        )+
        _ => Err(LuceneError::unsupported_operation(
          "get_suppressed is not supported for this error",
        )),
      }
    }
  };

  ($fn_name:ident, $variant:ident, $error_type:ident) => {
    pub fn $fn_name(err: impl Into<$error_type>) -> Self {
      LuceneError::$variant(err.into())
    }
  };
}
impl LuceneError {
  pub fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
      (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
      message.clone()
    } else {
      "unknown panic payload".to_string()
    }
  }

  pub fn tragedy_from_panic(prefix: &str, payload: &(dyn Any + Send)) -> Self {
    LuceneError::tragedy(format!(
      "{prefix}: {}",
      LuceneError::panic_payload_message(payload)
    ))
  }

  pub fn io_with_path(path: impl Into<String>, err: std::io::Error) -> Self {
    let message = err.kind().to_string();
    LuceneError::IoWithPath {
      source: err,
      path: path.into(),
      err_kind: message,
    }
  }

  pub fn io(err: std::io::Error) -> Self {
    Self::io_with_path("", err)
  }

  error_ctor!(already_closed, AlreadyClosed, AlreadyClosedError);
  error_ctor!(
    array_index_out_of_bounds,
    ArrayIndexOutOfBounds,
    ArrayIndexOutOfBoundsError
  );
  error_ctor!(buffer_allocation, BufferAllocation, BufferAllocationError);
  error_ctor!(
    collection_terminated,
    CollectionTerminated,
    CollectionTerminatedError
  );
  error_ctor!(corrupt_index, CorruptIndex, CorruptIndexError);
  error_ctor!(eof, Eof, Eof);
  error_ctor!(fuzzy_terms, FuzzyTerms, FuzzyTermsError);
  error_ctor!(illegal_argument, IllegalArgument, IllegalArgumentError);
  error_ctor!(illegal_state, IllegalState, IllegalStateError);
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
  error_ctor!(index_not_found, IndexNotFound, IndexNotFound);
  error_ctor!(lock_already_held, LockAlreadyHeld, LockAlreadyHeldError);
  error_ctor!(lock_held_by_other, LockHeldByOther, LockHeldByOtherError);
  error_ctor!(lock_obtain_failed, LockObtainFailed, LockObtainFailedError);
  error_ctor!(
    lock_release_failed,
    LockReleaseFailed,
    LockReleaseFailedError
  );
  error_ctor!(
    max_bytes_length_exceeded,
    MaxBytesLengthExceeded,
    MaxBytesLengthExceededError
  );
  error_ctor!(merge, Merge, MergeError);
  error_ctor!(merge_abort, MergeAborted, MergeAbortedError);
  error_ctor!(need_implemented, NeedImplemented, NeedImplementedError);
  error_ctor!(no_more_terms, NoMoreTerms, NoMoreTermsError);
  error_ctor!(no_such_element, NoSuchElement, NoSuchElementError);
  error_ctor!(not_such_file, NoSuchFile, NotSuchFileError);
  error_ctor!(not_implemented, NotImplemented, NotImplementedError);
  error_ctor!(number_format, NumberFormat, NumberFormatError);
  error_ctor!(number_overflow, NumberOverflow, NumberOverflow);
  error_ctor!(time_exceeded, TimeExceeded, TimeExceededError);
  error_ctor!(
    too_complex_to_determinize,
    TooComplexToDeterminize,
    TooComplexToDeterminizeError
  );
  error_ctor!(too_many_clauses, TooManyClauses, TooManyClausesError);
  error_ctor!(
    too_many_nested_clauses,
    TooManyNestedClauses,
    TooManyNestedClausesError
  );
  error_ctor!(tragedy, Tragedy, TragedyError);
  error_ctor!(unchecked_io_error, UncheckedIO, UncheckedIOError);
  error_ctor!(unreachable, Unreachable, UnreachableError);
  error_ctor!(
    unsupported_operation,
    UnsupportedOperation,
    UnsupportedOperationError
  );

  error_ctor!(
    @add_suppressed
    (AlreadyClosed),
    (ArrayIndexOutOfBounds),
    (BufferAllocation),
    (CollectionTerminated),
    (CorruptIndex),
    (Eof),
    (FuzzyTerms),
    (IllegalArgument),
    (IllegalState),
    (IndexFormatTooNew),
    (IndexFormatTooOld),
    (IndexNotFound),
    (LockAlreadyHeld),
    (LockHeldByOther),
    (LockObtainFailed),
    (LockReleaseFailed),
    (MaxBytesLengthExceeded),
    (Merge),
    (MergeAborted),
    (NeedImplemented),
    (NoMoreTerms),
    (NoSuchElement),
    (NoSuchFile),
    (NotImplemented),
    (NumberFormat),
    (NumberOverflow),
    (TimeExceeded),
    (TooComplexToDeterminize),
    (TooManyClauses),
    (TooManyNestedClauses),
    (Tragedy),
    (UncheckedIO),
    (Unreachable),
    (UnsupportedOperation),
  );
}

pub type Result<T> = core::result::Result<T, LuceneError>;
