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
/// [`Directory`](crate::store::directory::Directory)
pub trait FSDirectory {}
