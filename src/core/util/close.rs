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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

pub trait Closeable {
  fn close(&mut self) -> Result<()> {
    Ok(())
  }
}

/// A writable stream whose close operation can be invoked explicitly.
pub trait CloseableWrite: Write {
  fn close(self) -> Result<()>;
}

#[cfg(unix)]
impl CloseableWrite for File {
  fn close(self) -> Result<()> {
    use std::os::fd::IntoRawFd;

    let fd = self.into_raw_fd();
    if unsafe { libc::close(fd) } == -1 {
      Err(std::io::Error::last_os_error().into())
    } else {
      Ok(())
    }
  }
}

#[cfg(windows)]
impl CloseableWrite for File {
  fn close(self) -> Result<()> {
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::Foundation::CloseHandle;

    let handle = self.into_raw_handle();
    if unsafe { CloseHandle(handle) } == 0 {
      Err(std::io::Error::last_os_error().into())
    } else {
      Ok(())
    }
  }
}

#[cfg(not(any(unix, windows)))]
impl CloseableWrite for File {
  fn close(self) -> Result<()> {
    drop(self);
    Ok(())
  }
}

impl<W> CloseableWrite for &mut W
where
  W: Write + ?Sized,
{
  fn close(self) -> Result<()> {
    Ok(())
  }
}

impl<T: ?Sized + Closeable> Closeable for Arc<T> {
  fn close(&mut self) -> Result<()> {
    Err(LuceneError::unsupported_operation(
      "Closeable::close is unsupported for Arc; use CloseableRef for shared resources",
    ))
  }
}

impl<T: ?Sized + Closeable> Closeable for &mut T {
  fn close(&mut self) -> Result<()> {
    (**self).close()
  }
}

pub trait CloseableRef {
  fn close(&self) -> Result<()> {
    Ok(())
  }
}

impl<T: ?Sized + CloseableRef> CloseableRef for Arc<T> {
  fn close(&self) -> Result<()> {
    (**self).close()
  }
}

impl<T: ?Sized + CloseableRef> CloseableRef for &T {
  fn close(&self) -> Result<()> {
    (**self).close()
  }
}
