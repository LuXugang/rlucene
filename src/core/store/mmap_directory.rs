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
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

use crate::core::store::fs_directory::FSDirectory;
use crate::core::store::fs_directory_base::FSDirectoryBase;
use crate::core::store::lock_factory::LockFactory;
use crate::core::store::memory_segment_index_input::MemorySegmentIndexInput;
use crate::core::store::native_fs_lock_factory::NativeFSLockFactory;
use crate::core::store::{IOContext, ReadAdvice, fs_lock_factory};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Default maximum mmap chunk size.
///
/// This matches Java Lucene's defaults: 16 GiB on 64-bit targets and 256 MiB
/// otherwise.
#[cfg(target_pointer_width = "64")]
pub const DEFAULT_MAX_CHUNK_SIZE: u64 = 1u64 << 34;
/// Default maximum mmap chunk size.
///
/// This matches Java Lucene's defaults: 16 GiB on 64-bit targets and 256 MiB
/// otherwise.
#[cfg(not(target_pointer_width = "64"))]
pub const DEFAULT_MAX_CHUNK_SIZE: u64 = 1u64 << 28;

/// Predicate used by [`MMapPreload::Custom`] to decide whether a file should
/// be preloaded.
pub type MMapPreloadPredicate = dyn Fn(&str, &IOContext) -> bool + Send + Sync;

/// Configures which files should be preloaded into physical memory when they
/// are opened.
pub enum MMapPreload {
  /// Preload every file that is opened.
  AllFiles,
  /// Do not preload files.
  NoFiles,
  /// Preload files whose [`IOContext`] uses [`ReadAdvice::RandomPreload`].
  BasedOnLoadIOContext,
  /// Use a custom predicate whose first argument is the file name and second
  /// argument is the [`IOContext`] used to open the file.
  Custom(Box<MMapPreloadPredicate>),
}

impl Debug for MMapPreload {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::AllFiles => f.write_str("AllFiles"),
      Self::NoFiles => f.write_str("NoFiles"),
      Self::BasedOnLoadIOContext => f.write_str("BasedOnLoadIOContext"),
      Self::Custom(_) => f.write_str("Custom"),
    }
  }
}

impl MMapPreload {
  /// Creates a custom preload predicate.
  pub fn custom<F>(preload: F) -> Self
  where
    F: Fn(&str, &IOContext) -> bool + Send + Sync + 'static,
  {
    Self::Custom(Box::new(preload))
  }

  fn test(&self, filename: &str, context: &IOContext) -> bool {
    match self {
      Self::AllFiles => true,
      Self::NoFiles => false,
      Self::BasedOnLoadIOContext => *context.get_read_advice() == ReadAdvice::RandomPreload,
      Self::Custom(preload) => preload(filename, context),
    }
  }
}

/// File-system directory implementation that uses mmap for reading.
pub struct MMapDirectory {
  preload: MMapPreload,
  chunk_size_power: u32,
}

impl Default for MMapDirectory {
  fn default() -> Self {
    Self {
      preload: MMapPreload::NoFiles,
      chunk_size_power: Self::chunk_size_power(DEFAULT_MAX_CHUNK_SIZE).expect("valid default"),
    }
  }
}

impl MMapDirectory {
  /// Configures all files to be preloaded upon opening them.
  pub const ALL_FILES: MMapPreload = MMapPreload::AllFiles;
  /// Configures no files to be preloaded upon opening them.
  pub const NO_FILES: MMapPreload = MMapPreload::NoFiles;
  /// Configures files to be preloaded when they use
  /// [`ReadAdvice::RandomPreload`].
  pub const BASED_ON_LOAD_IO_CONTEXT: MMapPreload = MMapPreload::BasedOnLoadIOContext;

  /// Creates a new mmap-backed [`FSDirectory`] for the named location using
  /// the default lock factory.
  ///
  /// The directory is created at the named location if it does not yet exist.
  pub fn new(directory: PathBuf) -> Result<FSDirectory<NativeFSLockFactory, Self>> {
    Self::with_lock_factory(directory, fs_lock_factory::get_default())
  }

  /// Creates a new mmap-backed [`FSDirectory`] for the named location using
  /// the provided lock factory.
  ///
  /// The directory is created at the named location if it does not yet exist.
  pub fn with_lock_factory<D>(directory: PathBuf, lock_factory: D) -> Result<FSDirectory<D, Self>>
  where
    D: LockFactory,
  {
    Self::with_lock_factory_and_max_chunk_size(directory, lock_factory, DEFAULT_MAX_CHUNK_SIZE)
  }

  /// Creates a new mmap-backed [`FSDirectory`] for the named location using
  /// the default lock factory and the provided maximum mmap chunk size.
  ///
  /// The chunk size is rounded down to a power of two.
  pub fn with_max_chunk_size(
    directory: PathBuf,
    max_chunk_size: u64,
  ) -> Result<FSDirectory<NativeFSLockFactory, Self>> {
    Self::with_lock_factory_and_max_chunk_size(
      directory,
      fs_lock_factory::get_default(),
      max_chunk_size,
    )
  }

  /// Creates a new mmap-backed [`FSDirectory`] for the named location,
  /// specifying both the lock factory and the maximum mmap chunk size.
  ///
  /// Using a smaller chunk size can help on address-space constrained
  /// platforms. The chunk size is rounded down to a power of two, matching
  /// Java Lucene's constructor behavior.
  pub fn with_lock_factory_and_max_chunk_size<D>(
    directory: PathBuf,
    lock_factory: D,
    max_chunk_size: u64,
  ) -> Result<FSDirectory<D, Self>>
  where
    D: LockFactory,
  {
    FSDirectory::with_lock_factory(
      directory,
      lock_factory,
      Self {
        preload: MMapPreload::NoFiles,
        chunk_size_power: Self::chunk_size_power(max_chunk_size)?,
      },
    )
  }

  /// Configures which files to preload in physical memory upon opening.
  ///
  /// The default is [`MMapPreload::NoFiles`]. The behavior is best effort and
  /// operating-system dependent.
  pub fn set_preload(&mut self, preload: MMapPreload) {
    self.preload = preload;
  }

  /// Returns the current mmap chunk size.
  pub fn get_max_chunk_size(&self) -> u64 {
    1u64 << self.chunk_size_power
  }

  /// Returns whether this platform supports advising the kernel with
  /// `madvise`.
  pub fn supports_madvise() -> bool {
    cfg!(unix)
  }

  fn chunk_size_power(max_chunk_size: u64) -> Result<u32> {
    if max_chunk_size == 0 {
      return Err(LuceneError::illegal_argument(
        "Maximum chunk size for mmap must be >0",
      ));
    }
    let chunk_size_power = u64::BITS - 1 - max_chunk_size.leading_zeros();
    debug_assert!((1u64 << chunk_size_power) <= max_chunk_size);
    debug_assert!((1u64 << chunk_size_power) > (max_chunk_size / 2));
    Ok(chunk_size_power)
  }
}

impl Display for MMapDirectory {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MMapDirectory")
  }
}

impl FSDirectoryBase for MMapDirectory {
  type Output = MemorySegmentIndexInput;

  /// Creates an [`IndexInput`](crate::core::store::index_input::IndexInput) for
  /// the file with the given name.
  fn open_input(&self, name: &str, context: &IOContext, path: &Path) -> Result<Self::Output> {
    let file_path = path.join(name);
    let preload = self.preload.test(name, context);
    MemorySegmentIndexInput::new(
      format!("MemorySegmentIndexInput(path=\"{}\")", file_path.display()),
      &file_path,
      context.get_read_advice().clone(),
      self.chunk_size_power,
      preload,
    )
  }
}

impl<D> FSDirectory<D, MMapDirectory>
where
  D: LockFactory,
{
  /// Configures which files to preload in physical memory upon opening.
  pub fn set_preload(&mut self, preload: MMapPreload) {
    self.sub_fs_directory.set_preload(preload);
  }

  /// Returns the current mmap chunk size.
  pub fn get_max_chunk_size(&self) -> u64 {
    self.sub_fs_directory.get_max_chunk_size()
  }
}
