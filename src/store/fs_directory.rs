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
/// Base class for directory implementations that store index files in the file system.
///
/// There are currently two core implementations:
///
/// - **MMapDirectory**:
///   Uses memory-mapped IO when reading. This is a good choice if you have plenty of virtual
///   memory relative to your index size, such as running on a 64-bit system or on a 32-bit system
///   with small enough indexes. This class uses the modern `MemorySegment` API available in Java 21
///   to safely unmap previously memory-mapped files after closing the index inputs. For more
///   details about the foreign memory API, refer to the documentation of the `java.lang.foreign`
///   package or [Uwe's blog post](https://blog.thetaphi.de/2012/07/use-lucenes-mmapdirectory-on-64bit.html).
///
/// - **NIOFSDirectory**:
///   Uses `FileChannel`'s positional IO when reading to avoid synchronization when reading from the
///   same file. On non-Windows platforms, this is the preferred choice. However, due to a
///   Windows-specific [Sun JRE bug](http://bugs.sun.com/bugdatabase/view_bug.do?bug_id=6265734),
///   it is less suitable for Windows. Applications using `Thread::interrupt` or futures that cancel
///   with interruption should prefer `RAFDirectory` (provided in the `misc` module). See the
///   documentation for `NIOFSDirectory` for more details.
///
/// **Note:**
/// Due to system peculiarities, there is no single overall best implementation. To choose the
/// optimal implementation for your environment, you can use the `open` method to let Lucene select
/// the most appropriate FSDirectory implementation. If you have specific requirements, you can
/// directly instantiate the desired implementation.
///
/// **Important:**
/// Accessing one of these implementations from a thread that is interrupted during a blocked IO
/// operation can immediately close the underlying channel. The channel will remain closed, and
/// subsequent access to the index will throw a `ClosedChannelException`. Applications using
/// `Thread::interrupt` or futures with cancellation should use the legacy `RAFDirectory` from the
/// `misc` module.
///
/// By default, the locking implementation is `NativeFSLockFactory`, but you can change it by
/// providing a custom `LockFactory` instance.
///
/// # See Also
/// - `Directory`
pub trait FSDirectory{

}