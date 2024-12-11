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
use crate::store::directory::Directory;
use crate::store::index_input::IndexInput;
use crate::store::lock::{FSLockEnum, Lock};
use crate::store::lock_factory::LockFactory;
use crate::store::{IOContext, IndexOutput, NativeFSLockFactory, OutputStreamIndexOutput};
use crate::util::error::data_io_error_enum::DataIOError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64};

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
pub struct FSDirectory<D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    directory: PathBuf,
    /// Maps files that we are trying to delete (or we tried already but failed) before attempting to
    /// delete that key.
    pending_deletes: HashSet<String>,
    ops_since_last_delete: AtomicU32,
    /** Used to generate temp file names in [`createTempOutput`](Directory::create_temp_output). */
    next_temp_file_counter: AtomicU64,
    lock_factory: D,
    sub_fs_directory: T,
}
impl<D, T> FSDirectory<D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    pub fn new_with_lock_factory(
        directory: PathBuf,
        lock_factory: D,
        sub_fs_directory: T,
    ) -> Result<FSDirectory<D, T>, DataIOError> {
        if !directory.is_dir() {
            fs::create_dir(&directory)?; // create directory, if it doesn't exist
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
}
// impl<T> FSDirectory<NativeFSLockFactory,T> where  T: BaseDirectory{
//     pub fn new(directory: PathBuf,sub_fs_directory: T ) -> Result<FSDirectory<NativeFSLockFactory, T>, DataIOError> {
//         Self::new_with_lock_factory(directory, NativeFSLockFactory::new(), sub_fs_directory)
//     }
// }

impl<D, T> Directory for FSDirectory<D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    fn list_all(&self) -> Vec<String> {
        todo!()
    }

    fn delete_file(&self, name: &str) -> Result<(), DataIOError> {
        todo!()
    }

    fn file_length(&self, name: &str) -> u64 {
        todo!()
    }

    fn create_output(&self, name: &str, context: IOContext) -> OutputStreamIndexOutput<File> {
        todo!()
    }

    fn create_temp_output(
        &self,
        prefix: &str,
        suffix: &str,
        context: IOContext,
    ) -> Result<OutputStreamIndexOutput<File>, DataIOError> {
        todo!()
    }

    fn sync(&self, names: Vec<&str>) {
        todo!()
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

    fn obtain_lock(&mut self, name: &str) -> Result<FSLockEnum, DataIOError> {
        self.lock_factory.obtain_lock(&self.directory, name)
    }

    fn get_pending_deletions(&self) -> HashSet<String> {
        todo!()
    }
}

impl<D, T> Display for FSDirectory<D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<D, T> BaseDirectory for FSDirectory<D, T>
where
    D: LockFactory,
    T: BaseDirectory,
{
    fn obtain_lock(&mut self, name: &str) -> Result<FSLockEnum, DataIOError> {
        Directory::obtain_lock(self, name)
    }
}
