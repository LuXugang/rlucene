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

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use crate::core::store::directory::Directory;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{
  CaughtResult, LuceneError, PanicWithSuppressed, Result, SuppressedFailure,
};
#[cfg(test)]
use crate::test_framework::core::util::failure_context::{
  ExecutionMethod, ExecutionOwner, ExecutionScope,
};

enum CloseFailure {
  Panic(Box<dyn std::any::Any + Send>),
  Exception(LuceneError),
}

impl CloseFailure {
  fn into_suppressed(self) -> SuppressedFailure {
    match self {
      Self::Panic(payload) => SuppressedFailure::Panic(payload),
      Self::Exception(error) => SuppressedFailure::Exception(error),
    }
  }

  fn into_exception(self) -> LuceneError {
    match self {
      Self::Panic(payload) => {
        LuceneError::tragedy_from_panic("panic while closing", payload.as_ref())
      },
      Self::Exception(error) => error,
    }
  }
}

macro_rules! record_close_result {
  ($result:expr, $failures:ident) => {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $result)) {
      Ok(Ok(())) => {},
      Ok(Err(error)) => $failures.push(CloseFailure::Exception(error)),
      Err(payload) => $failures.push(CloseFailure::Panic(payload)),
    }
  };
}

macro_rules! finish_close_result {
  ($failures:ident) => {{
    let mut failures = $failures.into_iter();
    match failures.next() {
      Some(CloseFailure::Panic(primary)) => {
        let suppressed = failures
          .map(CloseFailure::into_suppressed)
          .collect::<Vec<_>>();
        if suppressed.is_empty() {
          std::panic::resume_unwind(primary);
        }
        std::panic::resume_unwind(Box::new(PanicWithSuppressed::with_suppressed(
          primary, suppressed,
        )));
      },
      Some(CloseFailure::Exception(primary)) => {
        let mut errors = failures
          .map(CloseFailure::into_exception)
          .collect::<Vec<_>>();
        let mut error = None;
        while let Some(mut current) = errors.pop() {
          if let Some(error) = error {
            current.add_suppressed(error);
          }
          error = Some(current);
        }
        let mut primary = primary;
        if let Some(error) = error {
          primary.add_suppressed(error);
        }
        Err(primary)
      },
      None => Ok(()),
    }
  }};
}

pub(crate) trait CloseableRefTuple {
  fn close_refs(self) -> Result<()>;
}

macro_rules! impl_closeable_ref_tuple {
  ($(($T:ident, $index:tt)),+) => {
    impl<'a, $($T),+> CloseableRefTuple for ($(Option<&'a $T>,)+)
    where
      $($T: CloseableRef + ?Sized + 'a),+
    {
      fn close_refs(self) -> Result<()> {
        let mut failures = Vec::new();
        $(
          if let Some(object) = self.$index {
            record_close_result!(object.close(), failures);
          }
        )+
        finish_close_result!(failures)
      }
    }
  };
}

impl_closeable_ref_tuple!((A, 0), (B, 1));
impl_closeable_ref_tuple!((A, 0), (B, 1), (C, 2));
impl_closeable_ref_tuple!((A, 0), (B, 1), (C, 2), (D, 3));
impl_closeable_ref_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_closeable_ref_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_closeable_ref_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_closeable_ref_tuple!(
  (A, 0),
  (B, 1),
  (C, 2),
  (D, 3),
  (E, 4),
  (F, 5),
  (G, 6),
  (H, 7)
);

pub struct IOUtils;
pub(crate) struct CloseWhileHandlingException {
  first_panic: Option<Box<dyn std::any::Any + Send>>,
  suppressed_panics: Vec<Box<dyn std::any::Any + Send>>,
  suppressed_exceptions: Vec<LuceneError>,
}
impl CloseWhileHandlingException {
  pub(crate) fn new() -> Self {
    Self {
      first_panic: None,
      suppressed_panics: Vec::new(),
      suppressed_exceptions: Vec::new(),
    }
  }

  pub(crate) fn close<F>(&mut self, close: F)
  where
    F: FnOnce() -> Result<()>,
  {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(close)) {
      Ok(Ok(())) => {},
      // Java's catch (Throwable) branch suppresses non-Error exceptions.
      Ok(Err(error)) => self.suppressed_exceptions.push(error),
      // A Rust panic maps to Java Error: remember the first one and keep closing.
      Err(payload) if self.first_panic.is_none() => {
        self.first_panic = Some(payload);
      },
      Err(payload) => self.suppressed_panics.push(payload),
    }
  }

  pub(crate) fn finish(self) {
    if let Some(primary) = self.first_panic {
      if self.suppressed_panics.is_empty() && self.suppressed_exceptions.is_empty() {
        std::panic::resume_unwind(primary);
      }
      std::panic::resume_unwind(Box::new(PanicWithSuppressed::new(
        primary,
        self.suppressed_panics,
        self.suppressed_exceptions,
      )));
    }
  }
}

pub(crate) trait CloseWhileHandlingResource {
  fn close_while_handling_error(self, error: &mut CloseWhileHandlingException);
}

impl<T> CloseWhileHandlingResource for &mut T
where
  T: Closeable + ?Sized,
{
  fn close_while_handling_error(self, error: &mut CloseWhileHandlingException) {
    error.close(|| self.close());
  }
}

impl<T> CloseWhileHandlingResource for &T
where
  T: CloseableRef + ?Sized,
{
  fn close_while_handling_error(self, error: &mut CloseWhileHandlingException) {
    error.close(|| self.close());
  }
}

impl<T> CloseWhileHandlingResource for Option<T>
where
  T: CloseWhileHandlingResource,
{
  fn close_while_handling_error(self, error: &mut CloseWhileHandlingException) {
    if let Some(resource) = self {
      resource.close_while_handling_error(error);
    }
  }
}

macro_rules! impl_close_while_handling_resource_tuple {
  ($(($T:ident, $index:tt)),+) => {
    impl<$($T),+> CloseWhileHandlingResource for ($($T,)+)
    where
      $($T: CloseWhileHandlingResource),+
    {
      fn close_while_handling_error(self, error: &mut CloseWhileHandlingException) {
        $(self.$index.close_while_handling_error(error);)+
      }
    }
  };
}

impl_close_while_handling_resource_tuple!((A, 0), (B, 1));
impl_close_while_handling_resource_tuple!((A, 0), (B, 1), (C, 2));
impl_close_while_handling_resource_tuple!((A, 0), (B, 1), (C, 2), (D, 3));
impl_close_while_handling_resource_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_close_while_handling_resource_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_close_while_handling_resource_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_close_while_handling_resource_tuple!(
  (A, 0),
  (B, 1),
  (C, 2),
  (D, 3),
  (E, 4),
  (F, 5),
  (G, 6),
  (H, 7)
);

impl IOUtils {
  /// Deletes all given filesystem paths, suppressing all returned errors.
  ///
  /// Note: The `files` collection should not be empty.
  pub fn delete_paths_ignoring_exceptions<I, P>(files: I)
  where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
  {
    for file in files {
      let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fs::remove_file(file.as_ref())
      }));
    }
  }

  /// Deletes all given filesystem paths if they exist.
  ///
  /// If more than one path cannot be deleted, the first error is returned and
  /// the following errors are added to it as suppressed errors.
  pub fn delete_files_if_exist<I, P>(files: I) -> Result<()>
  where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
  {
    Self::close(files, |file| {
      let path = file.as_ref();
      match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(LuceneError::io_with_path(
          path.to_string_lossy().to_string(),
          error,
        )),
      }
    })
  }

  /// Deletes all given files, suppressing all returned errors.
  ///
  /// Note: The `files` collection should not be empty or contain `None`.
  pub fn delete_files_ignoring_exceptions<'a, T, D>(dir: &D, files: T)
  where
    T: IntoIterator<Item = &'a String>,
    D: Directory + ?Sized,
  {
    for name in files {
      let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dir.delete_file(name)));
    }
  }
  pub fn delete_files<'a, T, D>(dir: &D, names: T) -> Result<()>
  where
    T: IntoIterator<Item = &'a String>,
    D: Directory + ?Sized,
  {
    #[cfg(test)]
    let _execution_scope =
      ExecutionScope::enter(ExecutionOwner::IOUtils, ExecutionMethod::DeleteFiles);
    Self::close(names, |name| dir.delete_file(name))
  }

  /// Closes the given object.
  pub fn close_one<T>(object: &mut T) -> Result<()>
  where
    T: Closeable,
  {
    Self::close(std::iter::once(object), Closeable::close)
  }

  /// Closes all given objects.
  ///
  /// After everything is closed, the method either returns the first failure it
  /// hit while closing, or completes normally if there were no failures.
  pub fn close<I, F>(objects: I, mut close: F) -> Result<()>
  where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<()>,
  {
    let mut failures = Vec::new();
    for object in objects {
      record_close_result!(close(object), failures);
    }
    finish_close_result!(failures)
  }

  /// Closes the given object by shared reference.
  pub fn close_one_ref<T>(object: &T) -> Result<()>
  where
    T: CloseableRef,
  {
    Self::close_refs(std::iter::once(object))
  }

  /// Closes all given objects by shared reference.
  ///
  /// After everything is closed, the method either returns the first error it hit
  /// while closing, or completes normally if there were no errors.
  pub fn close_refs<I>(objects: I) -> Result<()>
  where
    I: IntoIterator,
    I::Item: CloseableRef,
  {
    Self::close(objects, |object| object.close())
  }

  /// Closes a tuple of different concrete types by shared reference.
  ///
  /// `None` elements are ignored. After everything is closed, the method either
  /// returns the first failure it hit while closing, or completes normally if
  /// there were no failures.
  pub(crate) fn close_refs_tuple<T>(objects: T) -> Result<()>
  where
    T: CloseableRefTuple,
  {
    objects.close_refs()
  }

  /// Closes all given objects, suppressing all returned errors.
  ///
  /// Even if a panic is raised, all given closeables are closed before the
  /// first panic is resumed.
  pub fn close_while_handling_error<I, F>(objects: I, mut close: F) -> Result<()>
  where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<()>,
  {
    let mut error = CloseWhileHandlingException::new();
    for object in objects {
      error.close(|| close(object));
    }
    error.finish();
    Ok(())
  }

  /// Closes one or more resources of different concrete types while handling
  /// an exception, equivalent to Java's `IOUtils.closeWhileHandlingException`.
  ///
  /// `None` resources are ignored. Returned errors are suppressed. Even if a
  /// panic is raised, all given resources are closed before the first panic is
  /// resumed with subsequent panics and returned errors retained as suppressed
  /// failures.
  pub(crate) fn close_while_handling_exception<T>(resources: T)
  where
    T: CloseWhileHandlingResource,
  {
    let mut error = CloseWhileHandlingException::new();
    resources.close_while_handling_error(&mut error);
    error.finish()
  }

  /// Rethrows a previously caught failure.
  ///
  /// This method never returns a successful value. A returned error is
  /// propagated as a [`Result::Err`], while a panic is resumed with its
  /// original payload. Passing a successful caught result is a programming
  /// error.
  pub fn rethrow_always<T, R>(result: CaughtResult<T>) -> Result<R> {
    match result {
      Ok(Err(error)) => Err(error),
      Err(payload) => std::panic::resume_unwind(payload),
      Ok(Ok(_)) => panic!("rethrow argument must contain a failure"),
    }
  }

  /// Ensure that any writes to the given file are written to the storage
  /// device.
  ///
  /// # Arguments
  ///
  /// * `file_to_sync` - The path to the file or directory to sync.
  /// * `is_dir` - If `true`, the given path is a directory. On platforms
  ///   where directory syncing is unsupported (like Windows), this will be
  ///   ignored for directories.
  pub fn fsync(file_to_sync: &PathBuf, is_dir: bool) -> Result<()> {
    if is_dir {
      if cfg!(windows) {
        if !file_to_sync.exists() {
          return Err(LuceneError::not_such_file(format!(
            "Directory not found: {}",
            file_to_sync.display()
          )));
        }
        return Ok(());
      }

      let dir_file = File::options()
        .read(true)
        .open(file_to_sync)
        .map_err(|e| match e.kind() {
          io::ErrorKind::NotFound => {
            LuceneError::not_such_file(format!("Directory not found: {}", file_to_sync.display()))
          },
          _ => LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e),
        })?;

      if let Err(_e) = dir_file.sync_all() {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        debug_assert!(
          false,
          "On Linux and macOS, syncing a directory should not throw an error. Got: {_e}"
        );
        return Ok(());
      }
    } else {
      let file = File::options()
        .write(true)
        .open(file_to_sync)
        .map_err(|e| LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e))?;

      file
        .sync_all()
        .map_err(|e| LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e))?;
    }

    Ok(())
  }

  /// Returns the second error if the first is [`None`], otherwise adds the second
  /// as suppressed to the first and returns it.
  pub fn use_or_suppress(first: Option<LuceneError>, second: LuceneError) -> LuceneError {
    match first {
      None => second,
      Some(mut first) => {
        first.add_suppressed(second);
        first
      },
    }
  }

  /// Combines a body result with a following close result using Java
  /// try-with-resources suppression semantics.
  #[inline]
  pub fn use_or_suppress_result<T>(result: Result<T>, close_result: Result<()>) -> Result<T> {
    match (result, close_result) {
      (Ok(value), Ok(())) => Ok(value),
      (Ok(_), Err(err)) => Err(err),
      (Err(err), Ok(())) => Err(err),
      (Err(err), Err(close_err)) => Err(Self::use_or_suppress(Some(err), close_err)),
    }
  }

  /// Combines a caught body result with a caught close result using Java
  /// try-with-resources suppression semantics.
  #[inline]
  pub fn use_or_suppress_caught_result<T>(
    result: CaughtResult<T>,
    close_result: CaughtResult,
  ) -> Result<T> {
    match result {
      Ok(result) => match close_result {
        Ok(close_result) => Self::use_or_suppress_result(result, close_result),
        Err(payload) => match result {
          Ok(_) => std::panic::resume_unwind(payload),
          Err(error) => Err(Self::use_or_suppress(
            Some(error),
            LuceneError::tragedy_from_panic("panic while closing", payload.as_ref()),
          )),
        },
      },
      Err(payload) => match close_result {
        Ok(Ok(())) => std::panic::resume_unwind(payload),
        Ok(Err(error)) => std::panic::resume_unwind(Box::new(
          PanicWithSuppressed::with_suppressed(payload, vec![SuppressedFailure::Exception(error)]),
        )),
        Err(close_payload) => {
          std::panic::resume_unwind(Box::new(PanicWithSuppressed::with_suppressed(
            payload,
            vec![SuppressedFailure::Panic(close_payload)],
          )))
        },
      },
    }
  }

  /// Combines a caught body result with a caught `finally` result using Java
  /// `try`/`finally` semantics. A failure or panic in the `finally` block
  /// overrides the body result or panic.
  #[inline]
  pub fn finally_caught_result<T>(
    result: CaughtResult<T>,
    finally_result: CaughtResult,
  ) -> Result<T> {
    match finally_result {
      Ok(Ok(())) => unwrap_caught_result!(result),
      Ok(Err(error)) => Err(error),
      Err(payload) => std::panic::resume_unwind(payload),
    }
  }

  /// Applies the consumer to all elements in the collection even if an error or panic occurs.
  /// The first failure is propagated and subsequent failures are suppressed.
  pub fn apply_to_all<T, F>(collection: &[T], consumer: F) -> Result<()>
  where
    F: FnMut(&T) -> Result<()>,
  {
    Self::close(collection, consumer)
  }
}
