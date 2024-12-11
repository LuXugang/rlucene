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
use crate::store::base_directory::BaseDirectory;
use crate::store::directory::{get_temp_file_name, Directory};
use crate::store::index_input::IndexInput;
use crate::store::lock::{FSLockEnum, Lock};
use crate::store::lock_factory::LockFactory;
use crate::store::{IOContext, IndexOutput, NativeFSLockFactory, OutputStreamIndexOutput};
use crate::util::error::data_io_error_enum::DataIOError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::Path;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::{fs, io};

/// Base trait for `Directory` implementations that store index files in the file system.
/// There are currently two core implementations:
///
/// - [`MMapDirectory`](crate::store::mmap_directory::MMapDirectory): Uses memory-mapped IO when reading.
///   This is a good choice if you have plenty of virtual memory relative to your index size.
///   It works well on 64-bit systems or on 32-bit systems with small enough index sizes. This implementation
///   utilizes the modern `MemorySegment` API available since Rust 21, allowing safe unmapping of previously
///   memory-mapped files after closing `IndexInput`s. No need to enable the "preview feature" of your Java version.
/// - [`NIOFSDirectory`](crate::store::nio_fs_directory::NIOFSDirectory): Uses `java.nio`'s `FileChannel`'s positional IO
///   to avoid synchronization when reading from the same file. This is the preferred choice on all platforms except
///   Windows, where a bug in the Sun JRE causes performance issues.
///   Applications using thread interruption or future cancellation should use `RAFDirectory` instead.
///
/// # Note
/// Accessing one of the above subclasses directly or indirectly from a thread while it's interrupted can cause the
/// underlying channel to close immediately, leading to subsequent `ClosedChannelException` errors. If your application
/// uses `Thread::interrupt()` or `Future::cancel()`, it's recommended to use the legacy `RAFDirectory` from the `misc` module.
///
/// The default locking implementation is [`NativeFSLockFactory`](crate::store::native_fs_lock_factory::NativeFSLockFactory),
/// but it can be replaced with a custom `LockFactory`.
///
/// # See Also
/// [`Directory`](Directory)
pub struct FSDirectory<'a, D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    directory: &'a Path,
    /// Maps files that we are trying to delete (or we tried already but failed) before attempting to
    /// delete that key.
    pending_deletes: HashSet<String>,
    ops_since_last_delete: AtomicU32,
    /** Used to generate temp file names in [`createTempOutput`](Directory::create_temp_output). */
    next_temp_file_counter: AtomicU64,
    lock_factory: D,
    sub_fs_directory: T,
}
impl<D, T> FSDirectory<'_, D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    pub fn new_with_lock_factory(
        directory: &Path,
        lock_factory: D,
        sub_fs_directory: T,
    ) -> Result<FSDirectory<D, T>, DataIOError> {
        if !directory.is_dir() {
            fs::create_dir(directory)?;
        }
        Ok(FSDirectory {
            directory,
            pending_deletes: HashSet::new(),
            ops_since_last_delete: AtomicU32::new(0),
            next_temp_file_counter: AtomicU64::new(0),
            sub_fs_directory,
            lock_factory,
        })
    }

    fn list_all(
        dir: &Path,
        skip_names: Option<&HashSet<String>>,
    ) -> Result<Vec<String>, DataIOError> {
        let mut entries = Vec::new();

        for entry in dir.read_dir()? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            if let Some(skip) = &skip_names {
                if skip.contains(&name) {
                    continue;
                }
            }

            entries.push(name);
        }

        entries.sort();
        Ok(entries)
    }
    pub fn maybe_delete_pending_files(&mut self) -> Result<(), DataIOError> {
        if !self.pending_deletes.is_empty() {
            let count = self
                .ops_since_last_delete
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;

            if count as usize >= self.pending_deletes.len() {
                self.ops_since_last_delete
                    .fetch_sub(count, std::sync::atomic::Ordering::SeqCst);
                self.delete_pending_files()?;
            }
        }
        Ok(())
    }
    /// Try to delete any pending files that we had previously tried to delete but failed because we
    /// are on Windows and the files were still held open.
    pub fn delete_pending_files(&mut self) -> Result<(), DataIOError> {
        if !self.pending_deletes.is_empty() {
            // TODO: we could fix IndexInputs from FSDirectory subclasses to call this when they are
            // closed?

            // Clone the set since we mutate it in privateDeleteFile:
            let files_to_delete: Vec<String> = self.pending_deletes.clone().into_iter().collect();

            for name in files_to_delete {
                self.private_delete_file(&name, true)?;
            }
        }
        Ok(())
    }

    fn private_delete_file(
        &mut self,
        name: &str,
        is_pending_delete: bool,
    ) -> Result<(), DataIOError> {
        let file_path = self.directory.join(name);

        match fs::remove_file(file_path) {
            Ok(_) => {
                self.pending_deletes.remove(name);
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.pending_deletes.remove(name);

                if is_pending_delete && cfg!(windows) {
                    // TODO: can we remove this OS-specific hacky logic?  If windows deleteFile is buggy, we
                    // should instead contain this workaround in
                    // a WindowsFSDirectory ...
                    // LUCENE-6684: we suppress this check for Windows, since a file could be in a confusing
                    // "pending delete" state, failing the first
                    // delete attempt with access denied and then apparently falsely failing here when we try ot
                    // delete it again, with NSFE/FNFE
                    Ok(())
                } else {
                    Err(DataIOError::io(e))
                }
            }
            Err(e) => {
                // On windows, a file delete can fail because there's still an open
                // file handle against it.  We record this in pendingDeletes and
                // try again later.

                // TODO: this is hacky/lenient (we don't know which IOException this is), and
                // it should only happen on filesystems that can do this, so really we should
                // move this logic to WindowsDirectory or something

                // TODO: can/should we do if (Constants.WINDOWS) here, else throw the exc?
                // but what about a Linux box with a CIFS mount?
                if cfg!(windows) {
                    self.pending_deletes.insert(name.to_string());
                    Ok(())
                } else {
                    Err(DataIOError::io(e))
                }
            }
        }
    }
}
impl<'a, T> FSDirectory<'_, NativeFSLockFactory, T>
where
    T: BaseDirectory,
{
    pub fn new(
        directory: &'a Path,
        sub_fs_directory: T,
    ) -> Result<FSDirectory<NativeFSLockFactory, T>, DataIOError> {
        Self::new_with_lock_factory(directory, NativeFSLockFactory::new(), sub_fs_directory)
    }
}

impl<D, T> Directory for FSDirectory<'_, D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    fn list_all(&self) -> Vec<String> {
        Self::list_all(self.directory, Some(&self.pending_deletes)).unwrap()
    }

    fn delete_file(&self, name: &str) -> Result<(), DataIOError> {
        todo!()
    }

    fn file_length(&self, name: &str) -> Result<u64, DataIOError> {
        if self.pending_deletes.contains(name) {
            return Err(DataIOError::not_found(format!(
                "file \"{}\" is pending delete",
                name
            )));
        }

        let file_path = self.directory.join(name);
        let metadata = fs::metadata(file_path).map_err(DataIOError::io)?;
        Ok(metadata.len())
    }
    #[allow(refining_impl_trait)]
    fn create_output(
        &mut self,
        name: &str,
        _context: IOContext,
    ) -> Result<OutputStreamIndexOutput<File>, DataIOError> {
        self.maybe_delete_pending_files()?;

        if self.pending_deletes.remove(name) {
            self.private_delete_file(name, true)?;
            self.pending_deletes.remove(name);
        }

        let file_path = self.directory.join(name);
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&file_path)?;

        Ok(OutputStreamIndexOutput::new(
            format!("FSIndexOutput(path=\"{}\")", file_path.display()).as_str(),
            name,
            file,
            CHUNK_SIZE,
        )?)
    }
    #[allow(refining_impl_trait)]
    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        _context: IOContext,
    ) -> Result<OutputStreamIndexOutput<File>, DataIOError> {
        self.maybe_delete_pending_files()?;

        loop {
            let counter = self.next_temp_file_counter.fetch_add(1, SeqCst);
            let name = get_temp_file_name(prefix, suffix, counter);

            if self.pending_deletes.contains(&name) {
                continue;
            }

            let file_path = self.directory.join(&name);
            match File::options()
                .write(true)
                .create_new(true)
                .open(&file_path)
            {
                Ok(file) => {
                    return Ok(OutputStreamIndexOutput::new(
                        format!("FSIndexOutput(path=\"{}\")", file_path.display()).as_str(),
                        &name,
                        file,
                        CHUNK_SIZE,
                    )?);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    continue;
                }
                Err(e) => {
                    return Err(DataIOError::io(e));
                }
            }
        }
    }

    fn sync(&mut self, names: &[&str]) -> Result<(), DataIOError> {
        for &name in names {
            // self.fsync(name)?;
        }

        self.maybe_delete_pending_files()?;
        Ok(())
    }

    fn sync_metadata(&self) {
        todo!()
    }

    fn rename(&self, source: &str, dest: &str) -> Result<(), DataIOError> {
        todo!()
    }

    fn open_input(&self, name: &str, context: IOContext) -> Result<impl IndexInput, DataIOError> {
        self.sub_fs_directory.open_input(name, context)
    }
    #[allow(refining_impl_trait)]
    fn obtain_lock(&mut self, name: &str) -> Result<FSLockEnum, DataIOError> {
        self.lock_factory.obtain_lock(self.directory, name)
    }

    fn get_pending_deletions(&self) -> HashSet<String> {
        todo!()
    }
}

impl<D, T> Display for FSDirectory<'_, D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<D, T> BaseDirectory for FSDirectory<'_, D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    fn obtain_lock(&mut self, name: &str) -> Result<FSLockEnum, DataIOError> {
        Directory::obtain_lock(self, name)
    }
}
/// The maximum chunk size is 8192 bytes in the original Java implementation because:
/// - On certain platforms, Java's FileChannel or native I/O layers allocate a native buffer
///   (outside the heap) for each write operation if the write size exceeds 8192 bytes.
/// - Limiting the chunk size avoids unnecessary native memory allocation and improves performance.
///
/// In Rust, this restriction is not necessary when using `BufWriter`, because:
/// - `BufWriter` internally manages a buffer with a default size of 8192 bytes, which optimizes
///   the write operations by batching smaller writes into a single larger write.
/// - There is no native memory allocation overhead similar to Java's FileChannel behavior.
///
/// As a result, in Rust, we can safely rely on `BufWriter` for efficient buffered writes without
/// manually enforcing a chunk size limit.
const CHUNK_SIZE: u32 = 8192;
