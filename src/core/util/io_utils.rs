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

use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::core::store::directory::Directory;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};

macro_rules! close_objects {
  ($objects:expr, $close:expr) => {{
    let mut error = None;
    for object in $objects {
      if let Err(e) = $close(object) {
        error = Some(IOUtils::use_or_suppress(error, e));
      }
    }

    if let Some(error) = error {
      Err(error)
    } else {
      Ok(())
    }
  }};
}

pub struct IOUtils;
impl IOUtils {
  /// Deletes all given files, suppressing all thrown errors.
  ///
  /// Note: The `files` collection should not be empty or contain `None`.
  pub fn delete_files_ignoring_exceptions<'a, T>(dir: &impl Directory, files: T)
  where
    T: IntoIterator<Item = &'a String>,
  {
    for name in files {
      if dir.delete_file(name).is_err() {
        // Ignore the error and continue with the next file.
      }
    }
  }
  pub fn delete_files<'a, T>(dir: &impl Directory, names: T) -> Result<()>
  where
    T: IntoIterator<Item = &'a String>,
  {
    for name in names {
      dir.delete_file(name)?;
    }
    Ok(())
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
  /// After everything is closed, the method either returns the first error it hit
  /// while closing, or completes normally if there were no errors.
  pub fn close<I, F>(objects: I, mut close: F) -> Result<()>
  where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<()>,
  {
    close_objects!(objects, close)
  }

  /// Closes the given object by shared reference.
  pub fn close_one_ref<T>(object: &T) -> Result<()>
  where
    T: CloseableRef,
  {
    Self::close_refs(std::slice::from_ref(object))
  }

  /// Closes all given objects by shared reference.
  ///
  /// After everything is closed, the method either returns the first error it hit
  /// while closing, or completes normally if there were no errors.
  pub fn close_refs<T>(objects: &[T]) -> Result<()>
  where
    T: CloseableRef,
  {
    Self::close(objects, CloseableRef::close)
  }

  /// Closes all given objects, suppressing all returned errors.
  ///
  /// Even if a panic is raised, all given closeables are closed before the
  /// first panic is returned as an error.
  pub fn close_while_handling_error<I, F>(objects: I, mut close: F) -> Result<()>
  where
    I: IntoIterator,
    F: FnMut(I::Item) -> Result<()>,
  {
    let mut first_error = None;
    let mut first_panic = None;
    for object in objects {
      match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| close(object))) {
        Ok(Ok(())) => {},
        Ok(Err(e)) => {
          first_error = Some(IOUtils::use_or_suppress(first_error, e));
        },
        Err(e) => {
          first_panic = Some(IOUtils::use_or_suppress(
            first_panic,
            LuceneError::tragedy_from_panic("panic while closing", e.as_ref()),
          ));
        },
      }
    }

    if let Some(mut first_error) = first_error {
      if let Some(panic) = first_panic {
        first_error = IOUtils::use_or_suppress(Some(first_error), panic);
      }
      Err(first_error)
    } else {
      Ok(())
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

      file.sync_all().map_err(|e| {
        LuceneError::io_with_path(
          file_to_sync.to_string_lossy().to_string(),
          io::Error::new(e.kind(), format!("Failed to sync file: {e}")),
        )
      })?;
    }

    Ok(())
  }

  /// Returns the second error if the first is [`None`], otherwise adds the second
  /// as suppressed to the first and returns it.
  pub fn use_or_suppress(first: Option<LuceneError>, second: LuceneError) -> LuceneError {
    match first {
      None => second,
      Some(mut first) => {
        if let Err(e) = first.add_suppressed(second) {
          e
        } else {
          first
        }
      },
    }
  }

  /// Applies the consumer to all elements in the collection even if an error is returned.
  /// The first error returned by the consumer is returned and subsequent errors are suppressed.
  pub fn apply_to_all<T, F>(collection: &[T], consumer: F) -> Result<()>
  where
    F: FnMut(&T) -> Result<()>,
  {
    Self::close(collection, consumer)
  }
}
