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
use crate::core::store::data_input::DataInput;
use crate::core::store::directory::Directory;
use crate::core::store::index_input::IndexInput;
use crate::core::store::read_advice::ReadAdvice;
use crate::core::util::clone::TryClone;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::support::core::store::mock_directory_wrapper::MockDirectoryWrapper;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, ThreadId};
use std::time::Duration;

static NEXT_HANDLE_ID: AtomicUsize = AtomicUsize::new(0);

/// An IndexInput wrapper that tracks whether the input has been closed.
pub(crate) struct MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  dir: MockDirectoryWrapper<D>,
  pub(crate) name: String,
  in_: I,
  closed: Arc<AtomicBool>,

  // The closed state of the original input, or None if this is the original.
  parent: Option<Arc<AtomicBool>>,
  confined: bool,
  thread: ThreadId,
  read_advice: Mutex<ReadAdvice>,
  pub(crate) handle_id: usize,
}

impl<D, I> MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  pub(crate) fn new(
    dir: MockDirectoryWrapper<D>,
    name: impl Into<String>,
    delegate: I,
    parent: Option<Arc<AtomicBool>>,
    read_advice: ReadAdvice,
    confined: bool,
  ) -> Self {
    Self {
      dir,
      name: name.into(),
      in_: delegate,
      closed: Arc::new(AtomicBool::new(false)),
      parent,
      confined,
      thread: thread::current().id(),
      read_advice: Mutex::new(read_advice),
      handle_id: NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst),
    }
  }

  fn ensure_open(&self) -> Result<()> {
    if self.closed.load(Ordering::SeqCst) {
      return Err(LuceneError::already_closed("Abusing closed IndexInput!"));
    }
    if self
      .parent
      .as_ref()
      .is_some_and(|parent| parent.load(Ordering::SeqCst))
    {
      return Err(LuceneError::already_closed(
        "Abusing clone of a closed IndexInput!",
      ));
    }
    Ok(())
  }

  fn ensure_accessible(&self) -> Result<()> {
    if self.confined && self.thread != thread::current().id() {
      return Err(LuceneError::illegal_state("Abusing from another thread!"));
    }
    Ok(())
  }

  fn original_closed_state(&self) -> Arc<AtomicBool> {
    self
      .parent
      .as_ref()
      .cloned()
      .unwrap_or_else(|| Arc::clone(&self.closed))
  }
}

impl<D, I> Display for MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "MockIndexInputWrapper({})", self.in_)
  }
}

impl<D, I> TryClone for MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn try_clone(&self) -> Result<Self> {
    self.ensure_open()?;
    if self.dir.state.verbose_clone.load(Ordering::SeqCst) {
      eprintln!("clone: {self}");
    }
    self
      .dir
      .state
      .input_clone_count
      .fetch_add(1, Ordering::SeqCst);
    Ok(Self::new(
      self.dir.clone(),
      self.name.clone(),
      self.in_.try_clone()?,
      Some(self.original_closed_state()),
      *self.read_advice.lock(),
      self.confined,
    ))
  }
}

impl<D, I> Closeable for MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn close(&mut self) -> Result<()> {
    if self.closed.swap(true, Ordering::SeqCst) {
      self.in_.close()?;
      return Ok(());
    }

    if self.parent.is_none() {
      self.dir.remove_index_input(self.handle_id, &self.name);
    }
    let deterministic_result = self.dir.maybe_throw_deterministic_exception();
    let close_result = self.in_.close();
    deterministic_result?;
    close_result
  }
}

impl<D, I> Drop for MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn drop(&mut self) {
    if !self.closed.load(Ordering::SeqCst) {
      let _ = self.close();
    }
  }
}

impl<D, I> DataInput for MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn read_byte(&mut self) -> Result<u8> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_byte()
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_bytes(b, offset, len)
  }

  fn read_bytes_with_buffer(
    &mut self,
    b: &mut [u8],
    offset: usize,
    len: usize,
    use_buffer: bool,
  ) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_bytes_with_buffer(b, offset, len, use_buffer)
  }

  fn read_short(&mut self) -> Result<i16> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_short()
  }

  fn read_int(&mut self) -> Result<i32> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_int()
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_group_vint(dst, offset)
  }

  fn read_vint(&mut self) -> Result<i32> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_vint()
  }

  fn read_zint(&mut self) -> Result<i32> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_zint()
  }

  fn read_long(&mut self) -> Result<i64> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_long()
  }

  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_longs(dst, offset, len)
  }

  fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_ints(dst, offset, len)
  }

  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_floats(dst, offset, len)
  }

  fn read_vlong(&mut self) -> Result<i64> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_vlong()
  }

  fn read_zlong(&mut self) -> Result<i64> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_zlong()
  }

  fn read_string(&mut self) -> Result<String> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_string()
  }

  fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_map_of_strings()
  }

  fn read_set_of_strings(&mut self) -> Result<HashSet<String>> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.read_set_of_strings()
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    DataInput::skip_bytes(&mut self.in_, num_bytes)
  }

  fn is_index_input(&self) -> bool {
    true
  }

  fn seek_in_data_input(&mut self, pos: usize) -> Result<()> {
    self.seek(pos)
  }

  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    self.get_file_pointer()
  }
}

impl<D, I> IndexInput for MockIndexInputWrapper<D, I>
where
  D: Directory,
  I: IndexInput,
{
  type IndexInput = MockIndexInputWrapper<D, I::IndexInput>;

  fn get_file_pointer(&self) -> Result<usize> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.get_file_pointer()
  }

  fn seek(&mut self, pos: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.seek(pos)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    IndexInput::skip_bytes(&mut self.in_, num_bytes)
  }

  fn length(&self) -> Result<usize> {
    self.ensure_open()?;
    self.in_.length()
  }

  fn slice(
    &self,
    slice_description: &str,
    offset: usize,
    length: usize,
  ) -> Result<Self::IndexInput> {
    self.ensure_open()?;
    if self.dir.state.verbose_clone.load(Ordering::SeqCst) {
      eprintln!("slice: {self}");
    }
    self
      .dir
      .state
      .input_clone_count
      .fetch_add(1, Ordering::SeqCst);
    Ok(MockIndexInputWrapper::new(
      self.dir.clone(),
      slice_description,
      self.in_.slice(slice_description, offset, length)?,
      Some(self.original_closed_state()),
      *self.read_advice.lock(),
      self.confined,
    ))
  }

  fn slice_with_read_advice(
    &self,
    description: &str,
    offset: usize,
    length: usize,
    read_advice: &ReadAdvice,
  ) -> Result<Self::IndexInput> {
    if *self.read_advice.lock() != ReadAdvice::Normal {
      return Err(LuceneError::illegal_state(
        "slice() may only be called with a custom read advice on inputs that have been opened with ReadAdvice::Normal",
      ));
    }
    self.ensure_open()?;
    if self.dir.state.verbose_clone.load(Ordering::SeqCst) {
      eprintln!("slice: {self}");
    }
    self
      .dir
      .state
      .input_clone_count
      .fetch_add(1, Ordering::SeqCst);
    Ok(MockIndexInputWrapper::new(
      self.dir.clone(),
      description,
      self
        .in_
        .slice_with_read_advice(description, offset, length, read_advice)?,
      Some(self.original_closed_state()),
      *read_advice,
      self.confined,
    ))
  }

  type RandomAccessSlice = I::RandomAccessSlice;

  fn random_access_slice(&self, offset: usize, length: usize) -> Result<Self::RandomAccessSlice> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.random_access_slice(offset, length)
  }

  fn prefetch(&mut self, pos: usize, len: usize) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    self.in_.prefetch(pos, len)
  }

  fn update_read_advice(&self, read_advice: ReadAdvice) -> Result<()> {
    self.ensure_open()?;
    self.ensure_accessible()?;
    *self.read_advice.lock() = read_advice;
    self.in_.update_read_advice(read_advice)
  }
}

pub(crate) enum MockDirectoryIndexInput<D, I>
where
  D: Directory,
  I: IndexInput,
{
  Mock(MockIndexInputWrapper<D, I>),
  SlowClosing(MockIndexInputWrapper<D, I>),
  SlowOpening(MockIndexInputWrapper<D, I>),
}

impl<D, I> Display for MockDirectoryIndexInput<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.fmt(f),
    }
  }
}

impl<D, I> Closeable for MockDirectoryIndexInput<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowOpening(inner) => inner.close(),
      Self::SlowClosing(inner) => {
        thread::sleep(Duration::from_millis(50));
        inner.close()
      },
    }
  }
}

impl<D, I> TryClone for MockDirectoryIndexInput<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn try_clone(&self) -> Result<Self> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        Ok(Self::Mock(inner.try_clone()?))
      },
    }
  }
}

impl<D, I> DataInput for MockDirectoryIndexInput<D, I>
where
  D: Directory,
  I: IndexInput,
{
  fn read_byte(&mut self) -> Result<u8> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_byte(),
    }
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_bytes(b, offset, len)
      },
    }
  }

  fn read_bytes_with_buffer(
    &mut self,
    b: &mut [u8],
    offset: usize,
    len: usize,
    use_buffer: bool,
  ) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_bytes_with_buffer(b, offset, len, use_buffer)
      },
    }
  }

  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_floats(dst, offset, len)
      },
    }
  }

  fn read_short(&mut self) -> Result<i16> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_short(),
    }
  }

  fn read_int(&mut self) -> Result<i32> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_int(),
    }
  }

  fn read_long(&mut self) -> Result<i64> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_long(),
    }
  }

  fn read_string(&mut self) -> Result<String> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_string()
      },
    }
  }

  fn read_vint(&mut self) -> Result<i32> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_vint(),
    }
  }

  fn read_vlong(&mut self) -> Result<i64> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_vlong(),
    }
  }

  fn read_zint(&mut self) -> Result<i32> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_zint(),
    }
  }

  fn read_zlong(&mut self) -> Result<i64> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.read_zlong(),
    }
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        DataInput::skip_bytes(inner, num_bytes)
      },
    }
  }

  fn read_map_of_strings(&mut self) -> Result<HashMap<String, String>> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_map_of_strings()
      },
    }
  }

  fn read_set_of_strings(&mut self) -> Result<HashSet<String>> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_set_of_strings()
      },
    }
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_group_vint(dst, offset)
      },
    }
  }

  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_longs(dst, offset, len)
      },
    }
  }

  fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.read_ints(dst, offset, len)
      },
    }
  }

  fn is_index_input(&self) -> bool {
    true
  }

  fn seek_in_data_input(&mut self, pos: usize) -> Result<()> {
    self.seek(pos)
  }

  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    self.get_file_pointer()
  }
}

impl<D, I> IndexInput for MockDirectoryIndexInput<D, I>
where
  D: Directory,
  I: IndexInput,
{
  type IndexInput = MockDirectoryIndexInput<D, I::IndexInput>;

  fn get_file_pointer(&self) -> Result<usize> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.get_file_pointer()
      },
    }
  }

  fn seek(&mut self, pos: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.seek(pos),
    }
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        IndexInput::skip_bytes(inner, num_bytes)
      },
    }
  }

  fn length(&self) -> Result<usize> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => inner.length(),
    }
  }

  fn slice(
    &self,
    slice_description: &str,
    offset: usize,
    length: usize,
  ) -> Result<Self::IndexInput> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => Ok(
        MockDirectoryIndexInput::Mock(inner.slice(slice_description, offset, length)?),
      ),
    }
  }

  fn slice_with_read_advice(
    &self,
    description: &str,
    offset: usize,
    length: usize,
    read_advice: &ReadAdvice,
  ) -> Result<Self::IndexInput> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        Ok(MockDirectoryIndexInput::Mock(
          inner.slice_with_read_advice(description, offset, length, read_advice)?,
        ))
      },
    }
  }

  type RandomAccessSlice = I::RandomAccessSlice;

  fn random_access_slice(&self, offset: usize, length: usize) -> Result<Self::RandomAccessSlice> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.random_access_slice(offset, length)
      },
    }
  }

  fn prefetch(&mut self, pos: usize, len: usize) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.prefetch(pos, len)
      },
    }
  }

  fn update_read_advice(&self, read_advice: ReadAdvice) -> Result<()> {
    match self {
      Self::Mock(inner) | Self::SlowClosing(inner) | Self::SlowOpening(inner) => {
        inner.update_read_advice(read_advice)
      },
    }
  }
}
