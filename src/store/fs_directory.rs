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
use crate::store::fs_directory_base::FSDirectoryBase;
use crate::store::lock::Lock;
use crate::store::lock_factory::LockFactory;
use crate::store::{
    BufferedIndexInput, BufferedIndexInputBase, IOContext, IndexOutput, NativeFSLockFactory,
    OutputStreamIndexOutput,
};
use crate::util::error::data_io_error_enum::RuntimeError;
use crate::util::IOUtils;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};
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
/// The default locking implementation is [`NativeFSLockFactory`],
/// but it can be replaced with a custom `LockFactory`.
///
/// # See Also
/// [`Directory`]
pub struct FSDirectory<D, T, B>
where
    D: LockFactory,
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    directory: PathBuf,
    /// Maps files that we are trying to delete (or we tried already but failed) before attempting to
    /// delete that key.
    pending_deletes: Arc<Mutex<HashSet<String>>>,
    ops_since_last_delete: AtomicU32,
    /// Used to generate temp file names in [`createTempOutput`](Directory::create_temp_output).
    next_temp_file_counter: AtomicU64,
    lock_factory: D,
    sub_fs_directory: T,
}
impl<D, T, B> FSDirectory<D, T, B>
where
    D: LockFactory,
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    pub fn new_with_lock_factory(
        directory: PathBuf,
        lock_factory: D,
        sub_fs_directory: T,
    ) -> Result<FSDirectory<D, T, B>, RuntimeError> {
        if !directory.is_dir() {
            fs::create_dir(&directory)?;
        }
        Ok(FSDirectory {
            directory,
            pending_deletes: Arc::new(Mutex::new(HashSet::new())),
            ops_since_last_delete: AtomicU32::new(0),
            next_temp_file_counter: AtomicU64::new(0),
            sub_fs_directory,
            lock_factory,
        })
    }

    fn list_all(
        dir: &Path,
        skip_names: Option<&HashSet<String>>,
    ) -> Result<Vec<String>, RuntimeError> {
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
    pub fn maybe_delete_pending_files(
        directory: &Path,
        pending_deletes: &mut HashSet<String>,
        ops_since_last_delete: &mut AtomicU32,
    ) -> Result<(), RuntimeError> {
        if !pending_deletes.is_empty() {
            let count = ops_since_last_delete.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

            if count as usize >= pending_deletes.len() {
                ops_since_last_delete.fetch_sub(count, std::sync::atomic::Ordering::SeqCst);
                Self::delete_pending_files(directory, pending_deletes)?;
            }
        }
        Ok(())
    }

    /// Ensure that the given file is synchronized to the storage device.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the file to sync.
    ///
    /// # Errors
    ///
    /// Returns a `DataIOError` if the file cannot be found or synchronized.
    pub fn fsync(&self, name: &str) -> Result<(), RuntimeError> {
        IOUtils::fsync(&self.directory.join(name), false)
    }

    /// Try to delete any pending files that we had previously tried to delete but failed because we
    /// are on Windows and the files were still held open.
    pub fn delete_pending_files(
        directory: &Path,
        pending_deletes: &mut HashSet<String>,
    ) -> Result<(), RuntimeError> {
        if !pending_deletes.is_empty() {
            // TODO: we could fix IndexInputs from FSDirectory subclasses to call this when they are
            // closed?

            // Clone the set since we mutate it in privateDeleteFile:
            let files_to_delete: Vec<String> = pending_deletes.clone().into_iter().collect();

            for name in files_to_delete {
                Self::private_delete_file(directory, &name, true, pending_deletes)?;
            }
        }
        Ok(())
    }

    fn private_delete_file(
        directory: &Path,
        name: &str,
        is_pending_delete: bool,
        pending_deletes: &mut HashSet<String>,
    ) -> Result<(), RuntimeError> {
        let file_path = directory.join(name);
        let file_name = file_path.to_string_lossy().to_string();
        match fs::remove_file(file_path) {
            Ok(_) => {
                pending_deletes.remove(name);
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                pending_deletes.remove(name);

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
                    Err(RuntimeError::io_with_path(file_name, e))
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
                    pending_deletes.insert(name.to_string());
                    Ok(())
                } else {
                    Err(RuntimeError::io_with_path(file_name, e))
                }
            }
        }
    }
    fn ensure_can_read(&self, name: &str) -> Result<(), RuntimeError> {
        let pending_deletes = self.pending_deletes.lock().unwrap();
        if pending_deletes.contains(name) {
            return Err(RuntimeError::not_found(format!(
                "file \"{}\" is pending delete and cannot be opened for read",
                name
            )));
        }
        Ok(())
    }
}
impl<T, B> FSDirectory<NativeFSLockFactory, T, B>
where
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    pub fn new(
        directory: PathBuf,
        sub_fs_directory: T,
    ) -> Result<FSDirectory<NativeFSLockFactory, T, B>, RuntimeError> {
        Self::new_with_lock_factory(directory, NativeFSLockFactory::new(), sub_fs_directory)
    }
}

impl<D, T, B> Directory for FSDirectory<D, T, B>
where
    D: LockFactory,
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    fn list_all(&self) -> Result<Vec<String>, RuntimeError> {
        let pending_deletes = self.pending_deletes.lock().unwrap();
        Self::list_all(&self.directory, Some(&pending_deletes))
    }

    fn delete_file(&mut self, name: &str) -> Result<(), RuntimeError> {
        let mut pending_deletes = self.pending_deletes.lock().unwrap();
        if pending_deletes.contains(name) {
            return Err(RuntimeError::not_found(format!(
                "file \"{}\" is already pending delete",
                name
            )));
        }

        Self::private_delete_file(&self.directory, name, false, &mut pending_deletes)?;

        Self::maybe_delete_pending_files(
            &self.directory,
            &mut pending_deletes,
            &mut self.ops_since_last_delete,
        )?;

        Ok(())
    }

    fn file_length(&self, name: &str) -> Result<u64, RuntimeError> {
        if self.pending_deletes.lock().unwrap().contains(name) {
            return Err(RuntimeError::not_found(format!(
                "file \"{}\" is pending delete",
                name
            )));
        }

        let file_path = self.directory.join(name);
        let file_name = file_path.to_string_lossy().to_string();
        let metadata =
            fs::metadata(file_path).map_err(|e| RuntimeError::io_with_path(file_name, e))?;
        Ok(metadata.len())
    }
    fn create_output(
        &mut self,
        name: &str,
        _context: IOContext,
    ) -> Result<impl IndexOutput, RuntimeError> {
        let mut pending_deletes = self.pending_deletes.lock().unwrap();
        Self::maybe_delete_pending_files(
            &self.directory,
            &mut pending_deletes,
            &mut self.ops_since_last_delete,
        )?;

        if pending_deletes.remove(name) {
            Self::private_delete_file(&self.directory, name, true, &mut pending_deletes)?;
            pending_deletes.remove(name);
        }

        let file_path = self.directory.join(name);
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&file_path)
            .map_err(|err| {
                RuntimeError::io_with_path(file_path.to_string_lossy().to_string(), err)
            })?;

        OutputStreamIndexOutput::new(
            format!("FSIndexOutput(path=\"{}\")", file_path.display()).as_str(),
            name,
            file,
            CHUNK_SIZE,
        )
    }
    fn create_temp_output(
        &mut self,
        prefix: &str,
        suffix: &str,
        _context: IOContext,
    ) -> Result<impl IndexOutput, RuntimeError> {
        let mut pending_deletes = self.pending_deletes.lock().unwrap();
        Self::maybe_delete_pending_files(
            &self.directory,
            &mut pending_deletes,
            &mut self.ops_since_last_delete,
        )?;

        loop {
            let counter = self.next_temp_file_counter.fetch_add(1, SeqCst);
            let name = get_temp_file_name(prefix, suffix, counter);

            if pending_deletes.contains(&name) {
                continue;
            }

            let file_path = self.directory.join(&name);
            match File::options()
                .write(true)
                .create_new(true)
                .open(&file_path)
            {
                Ok(file) => {
                    return OutputStreamIndexOutput::new(
                        format!("FSIndexOutput(path=\"{}\")", file_path.display()).as_str(),
                        &name,
                        file,
                        CHUNK_SIZE,
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    continue;
                }
                Err(e) => {
                    return Err(RuntimeError::io_with_path(
                        file_path.to_string_lossy().to_string(),
                        e,
                    ));
                }
            }
        }
    }

    fn sync(&mut self, names: &[&str]) -> Result<(), RuntimeError> {
        for &name in names {
            self.fsync(name)?;
        }
        Self::maybe_delete_pending_files(
            &self.directory,
            &mut self.pending_deletes.lock().unwrap(),
            &mut self.ops_since_last_delete,
        )?;
        Ok(())
    }

    fn sync_metadata(&mut self) -> Result<(), RuntimeError> {
        // TODO: to improve listCommits(), IndexFileDeleter could call this after deleting segments_Ns
        IOUtils::fsync(&self.directory, true)?;
        Self::maybe_delete_pending_files(
            &self.directory,
            &mut self.pending_deletes.lock().unwrap(),
            &mut self.ops_since_last_delete,
        )?;
        Ok(())
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<(), RuntimeError> {
        let mut pending_deletes = self.pending_deletes.lock().unwrap();
        if pending_deletes.contains(source) {
            return Err(RuntimeError::not_found(format!(
                "File \"{}\" is pending delete and cannot be moved",
                source
            )));
        }
        Self::maybe_delete_pending_files(
            &self.directory,
            &mut pending_deletes,
            &mut self.ops_since_last_delete,
        )?;

        if pending_deletes.remove(dest) {
            Self::private_delete_file(&self.directory, dest, true, &mut pending_deletes)?; // try again to delete it - this is the best effort
            pending_deletes.remove(dest); // watch out if the delete fails, it's back in here
        }

        let source_path = self.directory.join(source);
        let dest_path = self.directory.join(dest);

        fs::rename(source_path, dest_path).map_err(RuntimeError::io)?;

        Ok(())
    }

    type Output = BufferedIndexInput<B>;
    fn open_input(&self, name: &str, context: IOContext) -> Result<Self::Output, RuntimeError> {
        self.ensure_can_read(name)?;
        self.sub_fs_directory
            .open_input(name, context, &self.directory)
    }

    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock, RuntimeError> {
        self.lock_factory.obtain_lock(&self.directory, name)
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>, RuntimeError> {
        let mut pending_deletes = self.pending_deletes.lock().unwrap();
        Self::delete_pending_files(&self.directory, &mut pending_deletes)?;
        if pending_deletes.is_empty() {
            Ok(HashSet::new())
        } else {
            Ok(pending_deletes.clone())
        }
    }

    #[cfg(feature = "test_only")]
    fn is_fs_directory(&self) -> bool {
        true
    }
}

impl<D, T, B> Display for FSDirectory<D, T, B>
where
    D: LockFactory,
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}@{} lockFactory={}",
            self.sub_fs_directory,
            self.directory.display(),
            self.lock_factory
        )
    }
}

impl<D, T, B> BaseDirectory for FSDirectory<D, T, B>
where
    D: LockFactory,
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    fn obtain_lock(&mut self, name: &str) -> Result<impl Lock, RuntimeError> {
        Directory::obtain_lock(self, name)
    }
}
impl<D, T, B> Drop for FSDirectory<D, T, B>
where
    D: LockFactory,
    B: BufferedIndexInputBase,
    T: FSDirectoryBase<Output = BufferedIndexInput<B>>,
{
    fn drop(&mut self) {
        let mut pending_deletes = self.pending_deletes.lock().unwrap();
        if let Err(e) = Self::maybe_delete_pending_files(
            &self.directory,
            &mut pending_deletes,
            &mut self.ops_since_last_delete,
        ) {
            eprintln!(
                "Error while deleting pending files during drop, ignoring: {:?}",
                e
            );
        }
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
