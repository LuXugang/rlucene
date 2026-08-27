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

use linked_hash_map::LinkedHashMap;

use crate::core::store::directory::Directory;
use crate::core::util::close::{Closeable, CloseableRef, CloseableWrite};
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

/// One or more resources that can be passed directly to [`IOUtils::close`].
///
/// This trait is an implementation detail of [`IOUtils`](crate::core::util::io_utils::IOUtils); callers should use
/// [`IOUtils::close`] rather than invoking it directly.
#[doc(hidden)]
pub trait CloseResources {
  fn close_resources(self) -> Result<()>;
}

impl<T> CloseResources for &mut T
where
  T: Closeable + ?Sized,
{
  fn close_resources(self) -> Result<()> {
    Closeable::close(self)
  }
}

impl<T> CloseResources for &T
where
  T: CloseableRef + ?Sized,
{
  fn close_resources(self) -> Result<()> {
    CloseableRef::close(self)
  }
}

impl<T> CloseResources for Option<T>
where
  T: CloseResources,
{
  fn close_resources(self) -> Result<()> {
    self.map_or(Ok(()), CloseResources::close_resources)
  }
}

impl<T, const N: usize> CloseResources for [T; N]
where
  T: CloseResources,
{
  fn close_resources(self) -> Result<()> {
    let mut failures = Vec::new();
    for resource in self {
      record_close_result!(resource.close_resources(), failures);
    }
    finish_close_result!(failures)
  }
}

impl<T> CloseResources for Vec<T>
where
  T: CloseResources,
{
  fn close_resources(self) -> Result<()> {
    let mut failures = Vec::new();
    for resource in self {
      record_close_result!(resource.close_resources(), failures);
    }
    finish_close_result!(failures)
  }
}

macro_rules! impl_close_resources_tuple {
  ($(($T:ident, $index:tt)),+) => {
    impl<$($T),+> CloseResources for ($($T,)+)
    where
      $($T: CloseResources),+
    {
      fn close_resources(self) -> Result<()> {
        let mut failures = Vec::new();
        $(
          record_close_result!(self.$index.close_resources(), failures);
        )+
        finish_close_result!(failures)
      }
    }
  };
}

impl_close_resources_tuple!((A, 0), (B, 1));
impl_close_resources_tuple!((A, 0), (B, 1), (C, 2));
impl_close_resources_tuple!((A, 0), (B, 1), (C, 2), (D, 3));
impl_close_resources_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_close_resources_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_close_resources_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_close_resources_tuple!(
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
#[doc(hidden)]
pub struct CloseWhileHandlingException {
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
      // Preserve returned errors and caught panics as suppressed failures.
      Ok(Err(error)) => self.suppressed_exceptions.push(error),
      // Remember the first panic and keep closing the remaining resources.
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

/// One or more resources that can be passed directly to
/// [`IOUtils::close_while_handling_exception`].
///
/// This trait is an implementation detail of [`IOUtils`](crate::core::util::io_utils::IOUtils); callers should use
/// [`IOUtils::close_while_handling_exception`] rather than invoking it
/// directly.
#[doc(hidden)]
pub trait CloseWhileHandlingResource {
  fn close_while_handling(self, failures: &mut CloseWhileHandlingException);
}

impl<T> CloseWhileHandlingResource for &mut T
where
  T: Closeable + ?Sized,
{
  fn close_while_handling(self, failures: &mut CloseWhileHandlingException) {
    failures.close(|| self.close());
  }
}

impl<T> CloseWhileHandlingResource for &T
where
  T: CloseableRef + ?Sized,
{
  fn close_while_handling(self, failures: &mut CloseWhileHandlingException) {
    failures.close(|| self.close());
  }
}

impl<T> CloseWhileHandlingResource for Option<T>
where
  T: CloseWhileHandlingResource,
{
  fn close_while_handling(self, failures: &mut CloseWhileHandlingException) {
    if let Some(resource) = self {
      resource.close_while_handling(failures);
    }
  }
}

impl<T, const N: usize> CloseWhileHandlingResource for [T; N]
where
  T: CloseWhileHandlingResource,
{
  fn close_while_handling(self, failures: &mut CloseWhileHandlingException) {
    for resource in self {
      resource.close_while_handling(failures);
    }
  }
}

impl<T> CloseWhileHandlingResource for Vec<T>
where
  T: CloseWhileHandlingResource,
{
  fn close_while_handling(self, failures: &mut CloseWhileHandlingException) {
    for resource in self {
      resource.close_while_handling(failures);
    }
  }
}

macro_rules! impl_close_while_handling_resource_tuple {
  ($(($T:ident, $index:tt)),+) => {
    impl<$($T),+> CloseWhileHandlingResource for ($($T,)+)
    where
      $($T: CloseWhileHandlingResource),+
    {
      fn close_while_handling(self, failures: &mut CloseWhileHandlingException) {
        $(self.$index.close_while_handling(failures);)+
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
  /// Closes one or more resources.
  ///
  /// Pass a resource directly, or pass an array, vector, or tuple when closing
  /// multiple resources. Tuple elements may have different concrete types and
  /// are closed from left to right. `None` resources are ignored.
  ///
  /// After everything is closed, the method returns or resumes the first
  /// failure and retains later failures as suppressed failures.
  pub fn close<T>(resources: T) -> Result<()>
  where
    T: CloseResources,
  {
    resources.close_resources()
  }

  /// Closes every item yielded by `objects` using the supplied `close`
  /// operation.
  ///
  /// This is the iterator/custom-operation form of Java's `IOUtils.close`. Use
  /// it for an arbitrary-length iterator or when the cleanup operation is not
  /// the resource's standard `close` method. For resources that can be passed
  /// directly, use [`Self::close`] instead.
  ///
  /// After everything is closed, the method returns or resumes the first
  /// failure and retains later failures as suppressed failures.
  pub fn close_with<I, F>(objects: I, mut close: F) -> Result<()>
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

  /// Closes one or more resources while preserving an error already in flight.
  ///
  /// Pass a resource directly, or pass an array, vector, or tuple when closing
  /// multiple resources. Tuple elements may have different concrete types and
  /// are closed from left to right. `None` resources are ignored. Use
  /// [`Self::close_while_handling_exception_with`]
  /// instead for an arbitrary iterator or a custom cleanup operation.
  ///
  /// All resources are attempted even if a returned error or panic occurs.
  /// Returned errors are suppressed. After every resource has been attempted,
  /// the first panic is resumed with later failures retained as suppressed
  /// failures.
  pub fn close_while_handling_exception<T>(resources: T)
  where
    T: CloseWhileHandlingResource,
  {
    let mut failures = CloseWhileHandlingException::new();
    resources.close_while_handling(&mut failures);
    failures.finish()
  }

  /// Closes every item yielded by `objects` using the supplied `close`
  /// operation, suppressing all returned errors.
  ///
  /// This is the iterator/custom-operation form of
  /// [`Self::close_while_handling_exception`]. Use it for an arbitrary-length
  /// collection whose items have one iterator item type, or when the cleanup
  /// operation is not the resource's standard `close` method. For resources
  /// that can be passed directly, use [`Self::close_while_handling_exception`]
  /// instead.
  ///
  /// All items are attempted even if a returned error or panic occurs. Returned
  /// errors are suppressed. After every item has been attempted, the first
  /// panic is resumed with later failures retained as suppressed failures.
  pub fn close_while_handling_exception_with<I, F>(objects: I, mut close: F)
  where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<()>,
  {
    let mut failures = CloseWhileHandlingException::new();
    for object in objects {
      failures.close(|| close(object));
    }
    failures.finish();
  }

  /// Deletes all given directory file names, suppressing all failures.
  pub fn delete_files_ignoring_exceptions<T, S, D>(dir: &D, files: T)
  where
    T: IntoIterator<Item = S>,
    S: AsRef<str>,
    D: Directory + ?Sized,
  {
    for name in files {
      let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dir.delete_file(name.as_ref())
      }));
    }
  }

  /// Deletes all given directory file names. `None` elements are ignored.
  /// The first failure is returned and later failures are retained as
  /// suppressed failures.
  pub fn delete_files<T, S, D>(dir: &D, names: T) -> Result<()>
  where
    T: IntoIterator<Item = Option<S>>,
    S: AsRef<str>,
    D: Directory + ?Sized,
  {
    #[cfg(test)]
    let _execution_scope =
      ExecutionScope::enter(ExecutionOwner::IOUtils, ExecutionMethod::DeleteFiles);
    Self::close_with(names, |name| {
      name.map_or(Ok(()), |name| dir.delete_file(name.as_ref()))
    })
  }

  /// Deletes all given filesystem paths, suppressing all returned errors.
  ///
  /// Missing paths and `None` elements are ignored.
  pub fn delete_paths_ignoring_exceptions<I, P>(files: I)
  where
    I: IntoIterator<Item = Option<P>>,
    P: AsRef<Path>,
  {
    for file in files.into_iter().flatten() {
      let path = file.as_ref();
      let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir()) {
          let _ = fs::remove_dir(path);
        } else {
          let _ = fs::remove_file(path);
        }
      }));
    }
  }

  /// Deletes all given filesystem paths if they exist. `None` elements are
  /// ignored.
  ///
  /// If more than one path cannot be deleted, the first error is returned and
  /// the following errors are added to it as suppressed errors.
  pub fn delete_files_if_exist<I, P>(files: I) -> Result<()>
  where
    I: IntoIterator<Item = Option<P>>,
    P: AsRef<Path>,
  {
    Self::close_with(files, |file| {
      let Some(file) = file else {
        return Ok(());
      };
      let path = file.as_ref();
      let result = if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
      {
        fs::remove_dir(path)
      } else {
        fs::remove_file(path)
      };
      match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(LuceneError::io_with_path(
          path.to_string_lossy().to_string(),
          error,
        )),
      }
    })
  }

  /// Deletes one or more files or directories and everything underneath
  /// them.
  ///
  /// All removal attempts are made. If anything cannot be removed, a single
  /// error lists the failed paths in attempt order.
  pub fn rm<I, P>(locations: I) -> Result<()>
  where
    I: IntoIterator<Item = Option<P>>,
    P: AsRef<Path>,
  {
    let mut unremoved = LinkedHashMap::new();
    for location in locations {
      let Some(location) = location else {
        continue;
      };
      let location = location.as_ref();
      // Keep Java's current leniency: missing locations, including broken
      // symbolic links, are ignored.
      if location.exists() {
        Self::rm_path(&mut unremoved, location);
      }
    }

    if unremoved.is_empty() {
      return Ok(());
    }

    let mut message =
      String::from("Could not remove the following files (in the order of attempts):\n");
    for (path, error) in unremoved {
      let absolute = std::path::absolute(&path).unwrap_or(path);
      message.push_str(&format!("   {}: {error}\n", absolute.display()));
    }
    Err(LuceneError::Io {
      source: io::Error::other(message),
      suppressed: None,
    })
  }

  fn rm_path(unremoved: &mut LinkedHashMap<PathBuf, io::Error>, location: &Path) {
    let metadata = match fs::symlink_metadata(location) {
      Ok(metadata) => metadata,
      Err(error) => {
        unremoved.insert(location.to_path_buf(), error);
        return;
      },
    };

    if metadata.file_type().is_dir() {
      let entries = match fs::read_dir(location) {
        Ok(entries) => entries,
        Err(error) => {
          unremoved.insert(location.to_path_buf(), error);
          return;
        },
      };
      for entry in entries {
        match entry {
          Ok(entry) => Self::rm_path(unremoved, &entry.path()),
          // `ReadDir` does not expose the failed entry's path. Record the
          // containing directory and keep visiting, like Java's
          // `visitFileFailed` callback.
          Err(error) => {
            unremoved.insert(location.to_path_buf(), error);
          },
        }
      }
      if let Err(error) = fs::remove_dir(location) {
        unremoved.insert(location.to_path_buf(), error);
      }
    } else if let Err(error) = fs::remove_file(location) {
      unremoved.insert(location.to_path_buf(), error);
    }
  }

  /// Returns a previously caught failure.
  ///
  /// This method never returns a successful value. A returned error is
  /// propagated as a [`Result::Err`], while a panic is resumed with its
  /// original payload. Passing a successful caught result is a programming
  /// error.
  pub fn rethrow_always<T, R>(result: CaughtResult<T>) -> Result<R> {
    match result {
      Ok(Err(error)) => Err(error),
      Err(payload) => std::panic::resume_unwind(payload),
      Ok(Ok(_)) => Err(LuceneError::illegal_state(
        "IOUtils::rethrow_always requires a previously caught failure",
      )),
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
  pub fn fsync(file_to_sync: &Path, is_dir: bool) -> Result<()> {
    if is_dir && cfg!(windows) {
      if !file_to_sync.exists() {
        return Err(LuceneError::not_such_file(format!(
          "Directory not found: {}",
          file_to_sync.display()
        )));
      }
      return Ok(());
    }

    let file = if is_dir {
      File::options()
        .read(true)
        .open(file_to_sync)
        .map_err(|e| match e.kind() {
          io::ErrorKind::NotFound => {
            LuceneError::not_such_file(format!("Directory not found: {}", file_to_sync.display()))
          },
          _ => LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e),
        })?
    } else {
      File::options()
        .write(true)
        .open(file_to_sync)
        .map_err(|e| LuceneError::io_with_path(file_to_sync.to_string_lossy().to_string(), e))?
    };

    let sync_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
      if let Err(error) = file.sync_all() {
        if !is_dir {
          return Err(LuceneError::io_with_path(
            file_to_sync.to_string_lossy().to_string(),
            error,
          ));
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        debug_assert!(
          false,
          "On Linux and macOS, syncing a directory should not return an error. Got: {error}"
        );
      }
      Ok(())
    }));
    let close_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| file.close()));

    Self::use_or_suppress_caught_result(sync_result, close_result)
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

  /// Applies the consumer to all non-`None` elements in the collection even
  /// if an error or panic occurs. The first failure is propagated and
  /// subsequent failures are suppressed.
  pub fn apply_to_all<I, T, F>(collection: I, mut consumer: F) -> Result<()>
  where
    I: IntoIterator<Item = Option<T>>,
    F: FnMut(T) -> Result<()>,
  {
    Self::close_with(collection, |element| element.map_or(Ok(()), &mut consumer))
  }
}
