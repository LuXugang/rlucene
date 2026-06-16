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
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::core::store::fs_directory::FSDirectory;
use crate::core::store::fs_directory_base::FSDirectoryBase;
use crate::core::store::index_input::get_full_slice_description;
use crate::core::store::lock_factory::LockFactory;
use crate::core::store::native_fs_lock_factory::NativeFSLockFactory;
use crate::core::store::{
  BUFFER_SIZE, BufferedIndexInput, BufferedIndexInputBase, IOContext, fs_lock_factory,
};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{ReadableCursorExt, TryIntoInt};

/// An implementation of
/// `FSDirectory` that uses
/// `std::fs::File` for positional reads, allowing multiple threads to read from
/// the same file without synchronization.
///
/// # Read and Write Modes
///
/// This struct uses `std::fs::File` for reading, enabling thread-safe
/// concurrent reads. Writing is achieved using
/// [`OutputStreamIndexOutput`](crate::core::store::output_stream_index_output).
pub struct NIOFSDirectory;

impl Default for NIOFSDirectory {
  fn default() -> Self {
    Self
  }
}

impl NIOFSDirectory {
  /// Creates a new NIOFS-backed [`FSDirectory`] for the named location using
  /// the default lock factory.
  ///
  /// The directory is created at the named location if it does not yet exist.
  pub fn new(directory: PathBuf) -> Result<FSDirectory<NativeFSLockFactory, Self>> {
    Self::with_lock_factory(directory, fs_lock_factory::get_default())
  }

  /// Creates a new NIOFS-backed [`FSDirectory`] for the named location using
  /// the provided lock factory.
  ///
  /// The directory is created at the named location if it does not yet exist.
  pub fn with_lock_factory<D>(directory: PathBuf, lock_factory: D) -> Result<FSDirectory<D, Self>>
  where
    D: LockFactory,
  {
    FSDirectory::with_lock_factory(directory, lock_factory, Self)
  }
}

impl Display for NIOFSDirectory {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "NIOFSDirectory")
  }
}

/// this method should only be called in
/// [`FSDirectory::open_input`](crate::core::store::fs_directory::FSDirectory), which
/// will first check whether file could be read
impl FSDirectoryBase for NIOFSDirectory {
  type Output = BufferedIndexInput<NIOFSIndexInput>;
  fn open_input(&self, name: &str, context: &IOContext, path: &Path) -> Result<Self::Output> {
    let file_path = path.join(name);
    let file_name = file_path.to_string_lossy().to_string();
    let file = match File::open(file_path) {
      Ok(file) => file,
      Err(err) => {
        return Err(LuceneError::io_with_path(file_name, err));
      },
    };
    let resource_desc = format!("NIOFSIndexInput(path=\"{}\")", path.display());
    // let resource_desc_string = resource_desc.to_string();
    let index_input = NIOFSIndexInput::new(file, &resource_desc)?;
    BufferedIndexInput::with_io_context(index_input, &resource_desc, context)
  }
}

const CHUNK_SIZE: usize = 16384;

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], pos: u64) -> std::io::Result<usize> {
  use std::os::unix::fs::FileExt;
  file.read_at(buf, pos)
}

#[cfg(windows)]
fn read_at(file: &File, buf: &mut [u8], pos: u64) -> std::io::Result<usize> {
  use std::os::windows::fs::FileExt;
  file.seek_read(buf, pos)
}

pub struct NIOFSIndexInput {
  /// the file we will read from
  file: File,
  /// start offset: non-zero in the slice case
  off: usize,
  /// end offset (start+length)
  end: usize,
  resource_desc: String,
  buffer_size: usize,
}

impl NIOFSIndexInput {
  pub fn new(file: File, resource_desc: &str) -> Result<Self> {
    let metadata = file.metadata()?;
    let len = metadata.len().try_convert()?;
    Ok(Self {
      file,
      off: 0,
      end: len,
      resource_desc: resource_desc.to_string(),
      buffer_size: BUFFER_SIZE,
    })
  }
  pub fn with_range(
    file: File,
    off: usize,
    length: usize,
    resource_desc: &str,
    buffer_size: usize,
  ) -> Self {
    Self {
      file,
      off,
      end: off + length,
      resource_desc: resource_desc.to_string(),
      buffer_size,
    }
  }
  pub fn get_buffer_size(&self) -> usize {
    self.buffer_size
  }
}

impl crate::core::util::clone::TryClone for NIOFSIndexInput {
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(Self {
      file: self.file.try_clone()?,
      off: self.off,
      end: self.end,
      resource_desc: self.resource_desc.clone(),
      buffer_size: self.buffer_size,
    })
  }
}

impl BufferedIndexInputBase for NIOFSIndexInput {
  fn seek_internal(&mut self, pos: usize) -> Result<()> {
    if pos > self.length() {
      return Err(LuceneError::eof(format!(
        "read past EOF: pos={} vs length={} in {}",
        pos,
        self.length(),
        self,
      )));
    }
    Ok(())
  }

  /// Reads data from the file into the provided buffer, ensuring that the
  /// data is read in chunks of a configurable size and does not exceed
  /// the file's defined bounds.
  ///
  /// # Arguments
  ///
  /// * `buffer` - A mutable reference to a `Cursor<Vec<u8>>`, which acts as
  ///   the target buffer for storing the data. The position of the cursor is
  ///   updated after each read to reflect the amount of data written.
  /// * `len` - The number of bytes to read from the file. This must not
  ///   exceed the buffer's remaining
  /// * `file_pointer` - The initial position in the file from which to start
  ///   reading.
  ///
  /// # Errors
  ///
  /// This method returns a `LuceneError` in the following cases:
  ///
  /// * [`LuceneError::Eof`] - If the requested read range exceeds the file's
  ///   bounds or if the file unexpectedly reaches EOF during a read.
  /// * [`LuceneError::Io`] - For general I/O errors encountered while reading
  ///   or seeking the file.
  ///
  /// # Details
  ///
  /// This method reads data from the file in chunks of up to `CHUNK_SIZE`
  /// bytes to optimize performance for large reads. Each chunk is written
  /// into the buffer starting at the cursor's current position, and the
  /// cursor's position is incremented accordingly. The method ensures
  /// that:
  ///
  /// 1. The file's read position (`file_pointer`) is correctly advanced for
  ///    each chunk.
  /// 2. The buffer is not overrun, with proper validation of its capacity
  ///    before writing.
  /// 3. The read length is fully consumed or an appropriate error is
  ///    returned.
  ///
  /// Each chunk is read with positional I/O so cloned inputs do not share a
  /// mutable OS file cursor.
  fn read_internal(
    &mut self,
    buffer: &mut Cursor<Vec<u8>>,
    len: usize,
    file_pointer: usize,
  ) -> Result<()> {
    debug_assert!(buffer.remain()? >= len, "buffer overflow");
    let mut pos = file_pointer + self.off;

    // Check if the requested read exceeds the file's end
    if pos + len > self.end {
      return Err(LuceneError::eof(format!(
        "read past EOF: position={} len={} end={}",
        pos, len, self.end
      )));
    }

    let mut read_length = len;
    while read_length > 0 {
      // Determine the size of the current chunk to read
      let to_read = CHUNK_SIZE.min(read_length);

      // Prepare the buffer slice for writing
      let buffer_start = buffer.position() as usize;
      let buffer_end = buffer_start + to_read;
      let buffer_slice = &mut buffer.get_mut()[buffer_start..buffer_end];

      let bytes_read = read_at(&self.file, buffer_slice, pos as u64).map_err(LuceneError::io)?;

      if bytes_read == 0 {
        return Err(LuceneError::eof(format!(
          "read past EOF during chunk read: position={} chunk size={} end={}",
          pos, to_read, self.end
        )));
      }

      // Update the position and remaining length
      pos += bytes_read;
      read_length -= bytes_read;
      // Update the buffer cursor position for next read
      buffer.set_position(buffer.position() + bytes_read as u64);
    }

    // Ensure the entire requested length was read
    debug_assert_eq!(
      read_length, 0,
      "Unexpected remaining length after read: {read_length}"
    );
    Ok(())
  }

  type Slice = BufferedIndexInput<NIOFSIndexInput>;

  fn slice(&self, slice_description: &str, offset: usize, length: usize) -> Result<Self::Slice> {
    if offset + length > self.length() {
      return Err(LuceneError::illegal_argument(format!(
        "slice() {} out of bounds: offset={}, length={}, fileLength={}: {}",
        slice_description,
        offset,
        length,
        self.length(),
        self
      )));
    }

    let resource_desc = get_full_slice_description(slice_description);
    let sub_index_input = NIOFSIndexInput::with_range(
      // Clone the file handle to create a new `File` instance pointing
      // to the same file resource.
      self.file.try_clone().map_err(LuceneError::io)?,
      self.off + offset,
      length,
      &resource_desc,
      self.buffer_size,
    );
    BufferedIndexInput::with_buffer_size(sub_index_input, &resource_desc, self.buffer_size)
  }
  fn length(&self) -> usize {
    self.end - self.off
  }
}

impl Display for NIOFSIndexInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.resource_desc)
  }
}
