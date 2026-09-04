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
#[cfg(unix)]
use std::io;

#[cfg(unix)]
use memmap2::{Advice, Mmap};

#[cfg(unix)]
use crate::core::store::ReadAdvice;
#[cfg(unix)]
use crate::core::store::native_access::NativeAccess;
#[cfg(unix)]
use crate::core::util::error::lucene_error::{LuceneError, Result};

#[cfg(unix)]
unsafe extern "C" {
  fn getpagesize() -> libc::c_int;
}

#[cfg(unix)]
#[derive(Clone, Copy)]
pub struct PosixNativeAccess {
  page_size: usize,
}

#[cfg(unix)]
impl PosixNativeAccess {
  pub fn new() -> Result<Self> {
    let page_size = unsafe { getpagesize() };
    if page_size <= 0 {
      return Err(LuceneError::illegal_state(format!(
        "getpagesize returned {page_size}"
      )));
    }
    let page_size = page_size as usize;
    Ok(Self { page_size })
  }

  fn to_advice(read_advice: &ReadAdvice) -> Option<Advice> {
    match read_advice {
      ReadAdvice::Normal | ReadAdvice::RandomPreload => Some(Advice::Normal),
      ReadAdvice::Random => Some(Advice::Random),
      ReadAdvice::Sequential => Some(Advice::Sequential),
    }
  }
}

#[cfg(unix)]
impl NativeAccess for PosixNativeAccess {
  fn map_read_advice(&self, read_advice: &ReadAdvice) -> Option<Advice> {
    Self::to_advice(read_advice)
  }

  fn madvise(&self, segment: &Mmap, read_advice: &ReadAdvice) -> io::Result<()> {
    if let Some(advice) = self.map_read_advice(read_advice) {
      segment.advise(advice)?;
    }
    Ok(())
  }

  fn madvise_will_need(&self, segment: &Mmap) -> io::Result<()> {
    segment.advise(Advice::WillNeed)
  }

  fn is_loaded(&self, segment: &Mmap, offset: usize, length: usize) -> io::Result<bool> {
    if length == 0 {
      return Ok(true);
    }

    let start = (segment.as_ptr() as usize)
      .checked_add(offset)
      .ok_or_else(|| io::Error::other("mmap range start overflow"))?;
    let end = start
      .checked_add(length)
      .ok_or_else(|| io::Error::other("mmap range end overflow"))?;
    let aligned_start = start - start % self.page_size;
    let aligned_length = end - aligned_start;
    let page_count = aligned_length.div_ceil(self.page_size);
    let mut residency = vec![0u8; page_count];

    // SAFETY: `aligned_start` is page-aligned and covers the mapped range represented by
    // `segment`; `residency` has one initialized byte for every page that `mincore` writes.
    let result = unsafe {
      libc::mincore(
        aligned_start as *mut libc::c_void,
        aligned_length,
        residency.as_mut_ptr().cast(),
      )
    };
    if result != 0 {
      return Err(io::Error::last_os_error());
    }
    Ok(residency.iter().all(|value| value & 1 != 0))
  }

  fn get_page_size(&self) -> usize {
    self.page_size
  }
}
