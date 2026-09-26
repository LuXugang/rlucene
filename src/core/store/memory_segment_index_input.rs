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

use crate::core::store::data_input_ext::DataInputExt;
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::store::{DataInput, IndexInput, ReadAdvice};
#[cfg(unix)]
use crate::core::store::{native_access::NativeAccess, posix_native_access::PosixNativeAccess};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::clone::TryClone;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::group_vint_util::{GroupVIntUtil, IntReader};
use crate::core::util::{CoreHelper, TryIntoInt};
#[cfg(unix)]
use memmap2::Advice;
use memmap2::{Mmap, MmapOptions};
use parking_lot::Mutex;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::hint::black_box;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

/// Immutable mappings are owned once for the whole file. A borrowed random read
/// remains valid after logical close; the last owner releases the mappings.
struct MappedFile {
  resource_desc: Box<str>,
  segments: Box<[Arc<Mmap>]>,
  closed: AtomicBool,
}

/// The final virtual segment can be empty, just as in Lucene's segment array.
/// Single views trim both ends; multi views retain the first segment's prefix.
#[derive(Clone, Copy)]
enum SegmentRange {
  Single {
    index: usize,
    start: usize,
    length: usize,
  },
  Multi {
    first: usize,
    count: usize,
    offset: usize,
    last_length: usize,
  },
}

struct MappedView {
  file: Arc<MappedFile>,
  range: SegmentRange,
  length: usize,
  power: u32,
  mask: usize,
  // Multi clones share Java's segment directory. Closing a clone clears that
  // directory, but another input can still hold its previously loaded segment.
  directory_closed: Option<Arc<AtomicBool>>,
}

// Holds the loaded mapping independently of the directory, just as Java's
// curSegment. The visible range belongs to this input's slice.
struct CurrentSegment {
  mapping: Option<Arc<Mmap>>,
  base: usize,
  len: usize,
}

impl CurrentSegment {
  fn byte(&self, pos: usize) -> Option<u8> {
    if pos >= self.len {
      return None;
    }
    // Loaded views satisfy base + len <= mapping.len(), so the logical
    // bounds check also proves that this address addition cannot overflow.
    self.mapping.as_deref()?.get(self.base + pos).copied()
  }

  fn range(&self, pos: usize, len: usize) -> Option<&[u8]> {
    if len > self.len.checked_sub(pos)? {
      return None;
    }
    // Check the requested bytes directly, without rebuilding the entire view.
    // The loaded view and logical bounds above also bound both additions.
    let start = self.base + pos;
    let mapping = self.mapping.as_deref().map_or(&[][..], |m| &m[..]);
    mapping.get(start..start + len)
  }

  fn as_slice(&self) -> &[u8] {
    self
      .mapping
      .as_deref()
      .map_or(&[], |mapping| &mapping[self.base..self.base + self.len])
  }
}

struct SegmentState {
  index: usize,
  current: CurrentSegment,
  // Logical positions that can be handled without resolving the directory.
  seek_start: usize,
  seek_count: usize,
  seek_position: usize,
}

struct SegmentCursor<'a> {
  state: &'a mut SegmentState,
  position: &'a mut usize,
}

impl SegmentCursor<'_> {
  fn segment(&self) -> &[u8] {
    self.state.current.as_slice()
  }
}

struct InputCursor {
  // Exclusive reads borrow this directly; shared multi fallback serializes
  // position changes with state. Single pointer queries need no lock.
  position: AtomicUsize,
  state: Mutex<SegmentState>,
}

impl InputCursor {
  fn new(view: &MappedView) -> Self {
    let (physical_index, start, end) = match view.range {
      SegmentRange::Single {
        index,
        start,
        length,
      } => (index, start, start + length),
      SegmentRange::Multi {
        first,
        count,
        last_length,
        ..
      } => (
        first,
        0,
        if count == 1 {
          last_length
        } else {
          view.file.segments.get(first).map_or(0, |m| m.len())
        },
      ),
    };
    let mut state = SegmentState {
      index: 0,
      current: CurrentSegment {
        mapping: view.file.segments.get(physical_index).cloned(),
        base: start,
        len: end - start,
      },
      seek_start: 0,
      seek_count: 0,
      seek_position: 0,
    };
    view.set_seek_window(&mut state, 0);
    Self {
      position: AtomicUsize::new(view.offset()),
      state: Mutex::new(state),
    }
  }
  fn get_mut(&mut self) -> SegmentCursor<'_> {
    SegmentCursor {
      state: self.state.get_mut(),
      position: self.position.get_mut(),
    }
  }
  fn file_pointer(&self, view: &MappedView) -> usize {
    match view.range {
      SegmentRange::Single { .. } => self.position.load(Ordering::Relaxed),
      SegmentRange::Multi { offset, .. } => {
        let state = self.state.lock();
        ((state.index << view.power) + self.position.load(Ordering::Relaxed)).wrapping_sub(offset)
      },
    }
  }
}

pub struct MemorySegmentIndexInput {
  resource_desc_suffix: ResourceDescriptionSuffix,
  view: MappedView,
  cursor: InputCursor,
  consecutive_prefetch_hit_count: AtomicI32,
  closed: AtomicBool,
  owns_file: bool,
  #[cfg(unix)]
  native_access: PosixNativeAccess,
}

/// A random slice owns an independent input, including the private cursor that
/// Lucene's multi-segment absolute scalar fallback updates.
pub struct MemorySegmentRandomAccessInput {
  input: MemorySegmentIndexInput,
}

#[derive(Clone)]
enum ResourceDescriptionSuffix {
  Inline { bytes: [u8; 22], len: u8 },
  Heap(Box<str>),
}

impl ResourceDescriptionSuffix {
  fn new() -> Self {
    Self::Inline {
      bytes: [0; 22],
      len: 0,
    }
  }

  fn as_bytes(&self) -> &[u8] {
    match self {
      Self::Inline { bytes, len } => &bytes[..*len as usize],
      Self::Heap(value) => value.as_bytes(),
    }
  }

  fn extend(&self, slice_description: &str) -> Result<Self> {
    let suffix = self.as_bytes();
    let new_len = suffix.len() + " [slice=]".len() + slice_description.len();
    if new_len <= 22 {
      let mut bytes = [0; 22];
      let mut offset = suffix.len();
      bytes[..offset].copy_from_slice(suffix);
      bytes[offset..offset + " [slice=".len()].copy_from_slice(b" [slice=");
      offset += " [slice=".len();
      bytes[offset..offset + slice_description.len()].copy_from_slice(slice_description.as_bytes());
      bytes[new_len - 1] = b']';
      Ok(Self::Inline {
        bytes,
        len: new_len as u8,
      })
    } else {
      let suffix = std::str::from_utf8(suffix)
        .map_err(|_| LuceneError::illegal_state("invalid resource description suffix"))?;
      let mut value = String::with_capacity(new_len);
      value.push_str(suffix);
      value.push_str(" [slice=");
      value.push_str(slice_description);
      value.push(']');
      Ok(Self::Heap(value.into_boxed_str()))
    }
  }
}

impl MappedView {
  fn set_seek_window(&self, state: &mut SegmentState, index: usize) {
    match self.range {
      SegmentRange::Single { length, .. } => {
        state.seek_start = 0;
        state.seek_count = length + 1;
        state.seek_position = 0;
      },
      SegmentRange::Multi { offset, .. } => {
        let base = index << self.power;
        let prefix = offset.saturating_sub(base);
        state.seek_start = base.saturating_sub(offset);
        state.seek_position = prefix;
        // A full segment's end belongs to the next segment for seek.
        state.seek_count = (state.current.len.min(self.mask) + 1).saturating_sub(prefix);
      },
    }
  }
  fn advance_segment(&self, cursor: &mut SegmentCursor<'_>) {
    cursor.state.index += 1;
    if matches!(self.range, SegmentRange::Multi { .. }) {
      // On failure Java retains curSegment but has already advanced its index.
      cursor.state.seek_count = 0;
    }
  }

  fn load_segment(&self, cursor: &mut SegmentCursor<'_>, index: usize) -> Result<()> {
    let (physical, start, end) = match self.range {
      SegmentRange::Single {
        index: physical,
        start,
        length,
      } => {
        if index != 0 {
          return Err(Self::eof());
        }
        (physical, start, start + length)
      },
      SegmentRange::Multi {
        first,
        count,
        last_length,
        ..
      } => {
        if index >= count {
          return Err(Self::eof());
        }
        let physical = first + index;
        let end = if index + 1 == count {
          last_length
        } else {
          self.file.segments.get(physical).map_or(0, |m| m.len())
        };
        (physical, 0, end)
      },
    };
    // Resolve first: a failed directory advance must retain the old mapping.
    cursor.state.current = CurrentSegment {
      mapping: self.file.segments.get(physical).cloned(),
      base: start,
      len: end - start,
    };
    self.set_seek_window(cursor.state, index);
    Ok(())
  }

  fn offset(&self) -> usize {
    match self.range {
      SegmentRange::Single { .. } => 0,
      SegmentRange::Multi { offset, .. } => offset,
    }
  }
  fn count(&self) -> usize {
    match self.range {
      SegmentRange::Single { .. } => 1,
      SegmentRange::Multi { count, .. } => count,
    }
  }
  fn directory_open(&self) -> Result<()> {
    if self
      .directory_closed
      .as_ref()
      .is_some_and(|c| c.load(Ordering::Relaxed))
    {
      return Err(LuceneError::already_closed(
        "mmap segment directory is closed",
      ));
    }
    Ok(())
  }
  fn eof() -> LuceneError {
    LuceneError::eof("read past EOF")
  }

  /// Returns the physical mapping and its visible byte range. An empty trailing
  /// segment has no mapping, so empty files need no OS mapping or dummy buffer.
  #[cfg(unix)]
  fn mapped_segment(&self, index: usize) -> Result<Option<(&Mmap, usize, usize)>> {
    match self.range {
      SegmentRange::Single {
        index: physical,
        start,
        length,
      } => {
        if index != 0 {
          return Err(Self::eof());
        }
        Ok(
          self
            .file
            .segments
            .get(physical)
            .map(|m| (m.as_ref(), start, length)),
        )
      },
      SegmentRange::Multi {
        first,
        count,
        last_length,
        ..
      } => {
        if index >= count {
          return Err(Self::eof());
        }
        Ok(self.file.segments.get(first + index).map(|m| {
          (
            m.as_ref(),
            0,
            if index + 1 == count {
              last_length
            } else {
              m.len()
            },
          )
        }))
      },
    }
  }
  fn segment(&self, index: usize) -> Option<&[u8]> {
    match self.range {
      SegmentRange::Single {
        index: physical,
        start,
        length,
      } => {
        if index != 0 {
          return None;
        }
        Some(
          self
            .file
            .segments
            .get(physical)
            .map_or(&[], |m| &m[start..start + length]),
        )
      },
      SegmentRange::Multi {
        first,
        count,
        last_length,
        ..
      } => {
        if index >= count {
          return None;
        }
        Some(self.file.segments.get(first + index).map_or(&[], |m| {
          if index + 1 == count {
            &m[..last_length]
          } else {
            &m[..]
          }
        }))
      },
    }
  }
  fn coordinates(&self, pos: usize) -> Result<(usize, usize)> {
    match self.range {
      SegmentRange::Single { .. } => Ok((0, pos)),
      SegmentRange::Multi { offset, .. } => {
        let absolute = pos.checked_add(offset).ok_or_else(Self::eof)?;
        Ok((absolute >> self.power, absolute & self.mask))
      },
    }
  }
  fn seek(&self, cursor: &mut SegmentCursor<'_>, pos: usize) -> Result<()> {
    let (index, offset) = self.coordinates(pos)?;
    if index != cursor.state.index {
      self.directory_open()?;
      self.load_segment(cursor, index)?;
      cursor.state.index = index;
    }
    if offset > cursor.state.current.len {
      return Err(Self::eof());
    }
    *cursor.position = offset;
    Ok(())
  }
  fn read_byte(&self, cursor: &mut SegmentCursor<'_>) -> Result<u8> {
    if let Some(value) = cursor.state.current.byte(*cursor.position) {
      *cursor.position += 1;
      return Ok(value);
    }
    self.read_byte_boundary(cursor.state, cursor.position)
  }
  #[cold]
  fn read_byte_boundary(&self, state: &mut SegmentState, position: &mut usize) -> Result<u8> {
    let mut cursor = SegmentCursor { state, position };
    loop {
      self.advance_segment(&mut cursor);
      if cursor.state.index >= self.count() {
        return Err(Self::eof());
      }
      self.directory_open()?;
      let index = cursor.state.index;
      self.load_segment(&mut cursor, index)?;
      *cursor.position = 0;
      if let Some(value) = cursor.state.current.byte(0) {
        *cursor.position = 1;
        return Ok(value);
      }
    }
  }
  fn read_scalar<const N: usize>(&self, cursor: &mut SegmentCursor<'_>) -> Result<[u8; N]> {
    if let Some(bytes) = cursor
      .state
      .current
      .range(*cursor.position, N)
      .and_then(|s| s.first_chunk::<N>())
    {
      let bytes = *bytes;
      *cursor.position += N;
      return Ok(bytes);
    }
    self.read_scalar_boundary(cursor)
  }
  #[cold]
  fn read_scalar_boundary<const N: usize>(
    &self,
    cursor: &mut SegmentCursor<'_>,
  ) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    let mut filled = 0;
    while filled < N {
      let segment = cursor.segment();
      if let Some(remaining) = segment.get(*cursor.position..)
        && !remaining.is_empty()
      {
        let count = remaining.len().min(N - filled);
        bytes[filled..filled + count].copy_from_slice(&remaining[..count]);
        *cursor.position += count;
        filled += count;
      } else {
        // Keep readByte's directory/EOF transition, including the old loaded
        // segment when advancing beyond the directory fails.
        bytes[filled] = self.read_byte_boundary(cursor.state, cursor.position)?;
        filled += 1;
      }
    }
    Ok(bytes)
  }
  fn copy(
    segment: &CurrentSegment,
    pos: usize,
    b: &mut [u8],
    offset: usize,
    len: usize,
  ) -> Result<()> {
    let source = segment.range(pos, len).ok_or_else(Self::eof)?;
    let target = b
      .get_mut(offset..)
      .and_then(|s| s.get_mut(..len))
      .ok_or_else(|| LuceneError::illegal_argument("destination range out of bounds"))?;
    target.copy_from_slice(source);
    Ok(())
  }
  fn read_bytes(
    &self,
    cursor: &mut SegmentCursor<'_>,
    b: &mut [u8],
    mut offset: usize,
    mut len: usize,
  ) -> Result<()> {
    loop {
      let segment = &cursor.state.current;
      let available = segment
        .len
        .checked_sub(*cursor.position)
        .ok_or_else(|| LuceneError::array_index_out_of_bounds("source position out of bounds"))?;
      if len <= available {
        Self::copy(segment, *cursor.position, b, offset, len)?;
        *cursor.position += len;
        return Ok(());
      }
      Self::copy(segment, *cursor.position, b, offset, available)?;
      len -= available;
      offset += available;
      self.advance_segment(cursor);
      if cursor.state.index >= self.count() {
        return Err(Self::eof());
      }
      self.directory_open()?;
      self.load_segment(cursor, cursor.state.index)?;
      *cursor.position = 0;
    }
  }
  fn slice(&self, offset: usize, length: usize) -> Result<Self> {
    match offset.checked_add(length) {
      Some(end) if end <= self.length => {},
      _ => return Err(LuceneError::illegal_argument("slice out of bounds")),
    }
    self.directory_open()?;
    let (range, shared_directory) = match self.range {
      SegmentRange::Single { index, start, .. } => (
        SegmentRange::Single {
          index,
          start: start + offset,
          length,
        },
        false,
      ),
      SegmentRange::Multi {
        first,
        offset: base,
        ..
      } => {
        let start = base + offset;
        let end = start + length;
        let first_index = start >> self.power;
        let last_index = end >> self.power;
        if first_index == last_index {
          (
            SegmentRange::Single {
              index: first + first_index,
              start: start & self.mask,
              length,
            },
            false,
          )
        } else {
          (
            SegmentRange::Multi {
              first: first + first_index,
              count: last_index - first_index + 1,
              offset: start & self.mask,
              last_length: end & self.mask,
            },
            start == 0 && length == self.length,
          )
        }
      },
    };
    let directory_closed = match range {
      SegmentRange::Single { .. } => None,
      SegmentRange::Multi { .. } if shared_directory => self.directory_closed.clone(),
      SegmentRange::Multi { .. } => Some(Arc::new(AtomicBool::new(false))),
    };
    Ok(Self {
      file: self.file.clone(),
      range,
      length,
      power: self.power,
      mask: self.mask,
      directory_closed,
    })
  }
}

impl MemorySegmentIndexInput {
  #[cold]
  fn seek_outside_current(&mut self, pos: usize) -> Result<()> {
    let result = if matches!(self.view.range, SegmentRange::Single { .. }) {
      Err(MappedView::eof())
    } else {
      self.view.seek(&mut self.cursor.get_mut(), pos)
    };
    result.map_err(|error| match error {
      LuceneError::Eof(_) => LuceneError::eof(format!("seek past EOF (pos={pos}): {self}")),
      error => error,
    })
  }

  pub fn new<S>(
    resource_desc: S,
    path: &Path,
    read_advice: ReadAdvice,
    chunk_size_power: u32,
    preload: bool,
  ) -> Result<Self>
  where
    S: Into<String>,
  {
    let resource_desc = resource_desc.into();
    let file =
      File::open(path).map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;
    let file_size_u64 = file
      .metadata()
      .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?
      .len();
    let length: usize = file_size_u64.try_convert()?;

    if chunk_size_power >= usize::BITS {
      return Err(LuceneError::illegal_argument(format!(
        "chunkSizePower {chunk_size_power} is too large for this platform"
      )));
    }
    if (file_size_u64 >> chunk_size_power) >= i32::MAX as u64 {
      return Err(LuceneError::illegal_argument(format!(
        "File too big for chunk size: {resource_desc}"
      )));
    }

    let chunk_size = 1usize << chunk_size_power;
    #[cfg(unix)]
    let native_access = PosixNativeAccess::new()?;
    #[cfg(not(unix))]
    let _ = read_advice;
    // Only non-empty chunks are mapped; a partial final chunk needs one slot.
    let mut segments = Vec::with_capacity(length.div_ceil(chunk_size));
    let mut start_offset = 0usize;
    while start_offset < length {
      let seg_size = chunk_size.min(length - start_offset);
      let mmap = unsafe {
        MmapOptions::new()
          .offset(start_offset.try_convert()?)
          .len(seg_size)
          .map(&file)
      }
      .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;

      #[cfg(unix)]
      {
        // Java's preload path uses MemorySegment::load and explicitly bypasses madvise.
        if !preload && read_advice != ReadAdvice::Normal {
          native_access
            .madvise(&mmap, &read_advice)
            .map_err(|e| LuceneError::io_with_path(path.display().to_string(), e))?;
        }
      }

      if preload {
        let mut value = 0u8;
        let mut pos = 0usize;
        while pos < mmap.len() {
          value ^= mmap[pos];
          pos += 4096;
        }
        black_box(value);
      }

      segments.push(Arc::new(mmap));
      start_offset += seg_size;
    }

    let range = if length < chunk_size {
      SegmentRange::Single {
        index: 0,
        start: 0,
        length,
      }
    } else {
      SegmentRange::Multi {
        first: 0,
        count: (length >> chunk_size_power) + 1,
        offset: 0,
        last_length: length & (chunk_size - 1),
      }
    };
    let view = MappedView {
      file: Arc::new(MappedFile {
        resource_desc: resource_desc.into_boxed_str(),
        segments: segments.into_boxed_slice(),
        closed: AtomicBool::new(false),
      }),
      range,
      length,
      power: chunk_size_power,
      mask: chunk_size - 1,
      directory_closed: if length < chunk_size {
        None
      } else {
        Some(Arc::new(AtomicBool::new(false)))
      },
    };
    let cursor = InputCursor::new(&view);
    Ok(Self {
      resource_desc_suffix: ResourceDescriptionSuffix::new(),
      view,
      cursor,
      consecutive_prefetch_hit_count: AtomicI32::new(0),
      closed: AtomicBool::new(false),
      owns_file: true,
      #[cfg(unix)]
      native_access,
    })
  }
  fn is_closed(&self) -> bool {
    self.closed.load(Ordering::Relaxed)
  }
  fn is_closed_for_reading(&self) -> bool {
    self.closed.load(Ordering::Relaxed) || self.view.file.closed.load(Ordering::SeqCst)
  }
  fn is_closed_for_exclusive_reading(&mut self) -> bool {
    // Only the root can close the file. An exclusive borrow of that root also
    // excludes concurrent close, so its local flag is sufficient. A clone must
    // still observe the root's shared lifetime flag.
    *self.closed.get_mut() || (!self.owns_file && self.view.file.closed.load(Ordering::SeqCst))
  }
  #[cold]
  #[inline(never)]
  fn closed_error(&self) -> LuceneError {
    LuceneError::already_closed(format!("Already closed: {self}"))
  }
  fn with_view(&self, view: MappedView, suffix: ResourceDescriptionSuffix) -> Self {
    let cursor = InputCursor::new(&view);
    Self {
      resource_desc_suffix: suffix,
      view,
      cursor,
      consecutive_prefetch_hit_count: AtomicI32::new(0),
      closed: AtomicBool::new(false),
      owns_file: false,
      #[cfg(unix)]
      native_access: self.native_access,
    }
  }
  fn read_array_boundary<T, F, const N: usize>(
    &mut self,
    mut dst: &mut [T],
    decode: F,
  ) -> Result<()>
  where
    F: Fn([u8; N]) -> T,
  {
    while !dst.is_empty() {
      if self.is_closed_for_exclusive_reading() {
        return Err(self.closed_error());
      }
      let mut cursor = self.cursor.get_mut();
      let segment = cursor.segment();
      let count = (segment.len().saturating_sub(*cursor.position) / N).min(dst.len());
      if count == 0 {
        if *cursor.position >= segment.len() {
          // A complete element in the next segment does not need byte-wise
          // decoding. Preserve readByte's order of directory and cursor updates.
          loop {
            self.view.advance_segment(&mut cursor);
            if cursor.state.index >= self.view.count() {
              return Err(MappedView::eof());
            }
            self.view.directory_open()?;
            let index = cursor.state.index;
            self.view.load_segment(&mut cursor, index)?;
            *cursor.position = 0;
            if !cursor.segment().is_empty() {
              break;
            }
          }
          continue;
        }
        // Only the element crossing the boundary needs scalar fallback. If it
        // fails, earlier complete elements and its partial cursor progress stay.
        let bytes = self.view.read_scalar(&mut cursor)?;
        dst[0] = decode(bytes);
        dst = &mut dst[1..];
      } else {
        let bytes = &segment[*cursor.position..*cursor.position + count * N];
        for (value, chunk) in dst[..count].iter_mut().zip(bytes.as_chunks::<N>().0) {
          *value = decode(*chunk);
        }
        *cursor.position += count * N;
        dst = &mut dst[count..];
      }
    }
    Ok(())
  }
  fn read_absolute<const N: usize>(&self, pos: usize) -> Result<[u8; N]> {
    if self.is_closed_for_reading() {
      return Err(self.closed_error());
    }
    if let SegmentRange::Single {
      index,
      start,
      length,
    } = self.view.range
    {
      if length
        .checked_sub(pos)
        .is_none_or(|remaining| remaining < N)
      {
        return Err(MappedView::eof());
      }
      let bytes = &self.view.file.segments[index][start + pos..start + pos + N];
      return bytes.try_into().map_err(|_| MappedView::eof());
    }
    self.view.directory_open()?;
    let (index, offset) = self.view.coordinates(pos)?;
    let segment = self.view.segment(index).ok_or_else(MappedView::eof)?;
    if let Some(bytes) = segment.get(offset..).and_then(|s| s.first_chunk::<N>()) {
      return Ok(*bytes);
    }
    let mut state = self.cursor.state.lock();
    let mut position = self.cursor.position.load(Ordering::Relaxed);
    let result = {
      let mut cursor = SegmentCursor {
        state: &mut state,
        position: &mut position,
      };
      self.view.load_segment(&mut cursor, index)?;
      cursor.state.index = index;
      *cursor.position = offset;
      self.view.read_scalar(&mut cursor)
    };
    self.cursor.position.store(position, Ordering::Relaxed);
    result
  }
  #[cfg(unix)]
  fn advise_first<F>(&self, pos: usize, length: usize, advice: F) -> Result<()>
  where
    F: FnOnce(&Mmap, usize, usize) -> std::io::Result<()>,
  {
    if self.is_closed() {
      return Err(self.closed_error());
    }
    self.view.directory_open()?;
    CoreHelper::check_from_index_size(pos, length, self.view.length)?;
    // Java advice uses segment-directory coordinates, even on a multi slice.
    let index = pos >> self.view.power;
    let offset = pos & self.view.mask;
    let Some((map, start, visible)) = self.view.mapped_segment(index)? else {
      return Ok(());
    };
    if offset > visible {
      return Err(MappedView::eof());
    }
    let mut length = length.min(visible - offset);
    let mut physical_offset = start + offset;
    let page = self.native_access.get_page_size();
    let in_page = (map.as_ptr() as usize + physical_offset) % page;
    if in_page <= offset {
      physical_offset -= in_page;
      length += in_page;
    } else {
      let skipped = page - in_page;
      if length <= skipped {
        return Ok(());
      }
      physical_offset += skipped;
      length -= skipped;
    }
    if self.is_closed_for_reading() {
      return Err(self.closed_error());
    }
    advice(map, physical_offset, length).map_err(LuceneError::io)
  }
  fn prefetch_impl(&self, pos: usize, len: usize) -> Result<()> {
    #[cfg(unix)]
    {
      if self.is_closed() {
        return Err(self.closed_error());
      }
      CoreHelper::check_from_index_size(pos, len, self.view.length)?;
      let hits = self
        .consecutive_prefetch_hit_count
        .fetch_add(1, Ordering::Relaxed);
      if !BitUtil::is_zero_or_power_of_two(hits) {
        return Ok(());
      }
      let mut miss = false;
      let result = self.advise_first(pos, len, |map, offset, length| {
        if !self.native_access.is_loaded(map, offset, length)? {
          miss = true;
          map.advise_range(Advice::WillNeed, offset, length)?;
        }
        Ok(())
      });
      if miss {
        self
          .consecutive_prefetch_hit_count
          .store(0, Ordering::Relaxed);
      }
      result
    }
    #[cfg(not(unix))]
    {
      let _ = (pos, len);
      Ok(())
    }
  }
}

impl DataInput for MemorySegmentIndexInput {
  fn read_byte(&mut self) -> Result<u8> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    self.view.read_byte(&mut self.cursor.get_mut())
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    self
      .view
      .read_bytes(&mut self.cursor.get_mut(), b, offset, len)
  }
  fn read_short(&mut self) -> Result<i16> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    Ok(i16::from_le_bytes(
      self.view.read_scalar(&mut self.cursor.get_mut())?,
    ))
  }

  fn read_int(&mut self) -> Result<i32> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    Ok(i32::from_le_bytes(
      self.view.read_scalar(&mut self.cursor.get_mut())?,
    ))
  }

  fn read_long(&mut self) -> Result<i64> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    Ok(i64::from_le_bytes(
      self.view.read_scalar(&mut self.cursor.get_mut())?,
    ))
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    let cursor = self.cursor.get_mut();
    let remaining = cursor.state.current.len.saturating_sub(*cursor.position);
    let pos = *cursor.position;
    let len = GroupVIntUtil::read_group_vint_i32_with_reader(self, remaining, pos, dst, offset)?;
    *self.cursor.get_mut().position += len;
    Ok(())
  }
  fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<()> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    let cursor = self.cursor.get_mut();
    let byte_len = len.checked_mul(4).ok_or_else(MappedView::eof)?;
    if let Some(bytes) = cursor.state.current.range(*cursor.position, byte_len)
      && let Some(target) = dst.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      for (value, chunk) in target.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *value = i32::from_le_bytes(*chunk);
      }
      *cursor.position += byte_len;
      return Ok(());
    }
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    self.read_array_boundary(&mut dst[offset..offset + len], i32::from_le_bytes)
  }

  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    let cursor = self.cursor.get_mut();
    let byte_len = len.checked_mul(8).ok_or_else(MappedView::eof)?;
    if let Some(bytes) = cursor.state.current.range(*cursor.position, byte_len)
      && let Some(target) = dst.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      for (value, chunk) in target.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *value = i64::from_le_bytes(*chunk);
      }
      *cursor.position += byte_len;
      return Ok(());
    }
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    self.read_array_boundary(&mut dst[offset..offset + len], i64::from_le_bytes)
  }

  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    let cursor = self.cursor.get_mut();
    let byte_len = len.checked_mul(4).ok_or_else(MappedView::eof)?;
    if let Some(bytes) = cursor.state.current.range(*cursor.position, byte_len)
      && let Some(target) = dst.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      for (value, chunk) in target.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *value = f32::from_le_bytes(*chunk);
      }
      *cursor.position += byte_len;
      return Ok(());
    }
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    self.read_array_boundary(&mut dst[offset..offset + len], f32::from_le_bytes)
  }

  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    IndexInput::skip_bytes(self, num_bytes)
  }
}
impl DataInputExt for MemorySegmentIndexInput {
  crate::core::store::data_input_ext::impl_index_input_ext!();
}
impl IntReader for MemorySegmentIndexInput {
  fn read(&mut self, pos: usize) -> Result<i32> {
    if self.is_closed_for_exclusive_reading() {
      return Err(self.closed_error());
    }
    // Both Single slices and Multi cursors already cache their visible range.
    let cursor = self.cursor.get_mut();
    let bytes = cursor
      .state
      .current
      .range(pos, 4)
      .and_then(|s| s.first_chunk::<4>())
      .ok_or_else(MappedView::eof)?;
    Ok(i32::from_le_bytes(*bytes))
  }
}
impl Display for MemorySegmentIndexInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.view.file.resource_desc)?;
    let suffix =
      std::str::from_utf8(self.resource_desc_suffix.as_bytes()).map_err(|_| std::fmt::Error)?;
    f.write_str(suffix)
  }
}
impl Display for MemorySegmentRandomAccessInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.input.fmt(f)
  }
}
impl CloseableRef for MemorySegmentIndexInput {
  fn close(&self) -> Result<()> {
    if self.closed.load(Ordering::Relaxed) {
      return Ok(());
    }
    if !self.closed.swap(true, Ordering::Relaxed) {
      if self.owns_file {
        self.view.file.closed.store(true, Ordering::SeqCst);
      }
      if let Some(closed) = &self.view.directory_closed {
        closed.store(true, Ordering::Relaxed);
      }
    }
    Ok(())
  }
}
impl Drop for MemorySegmentIndexInput {
  fn drop(&mut self) {
    // Java collection of a clone is not an explicit close of its shared array.
    if self.owns_file {
      let _ = CloseableRef::close(self);
    }
  }
}
impl TryClone for MemorySegmentIndexInput {
  fn try_clone(&self) -> Result<Self> {
    if self.is_closed() {
      return Err(self.closed_error());
    }
    self.view.directory_open()?;
    let view = MappedView {
      file: self.view.file.clone(),
      range: self.view.range,
      length: self.view.length,
      power: self.view.power,
      mask: self.view.mask,
      directory_closed: self.view.directory_closed.clone(),
    };
    let mut clone = self.with_view(view, self.resource_desc_suffix.clone());
    clone.seek(self.get_file_pointer()?)?;
    Ok(clone)
  }
}
impl IndexInput for MemorySegmentIndexInput {
  type IndexInput = Self;
  type RandomAccessSlice = MemorySegmentRandomAccessInput;
  fn get_file_pointer(&self) -> Result<usize> {
    if self.is_closed() {
      return Err(self.closed_error());
    }
    Ok(self.cursor.file_pointer(&self.view))
  }
  #[inline]
  fn seek(&mut self, pos: usize) -> Result<()> {
    if *self.closed.get_mut() {
      return Err(self.closed_error());
    }
    let cursor = self.cursor.get_mut();
    let relative = pos.wrapping_sub(cursor.state.seek_start);
    if relative < cursor.state.seek_count {
      *cursor.position = relative + cursor.state.seek_position;
      return Ok(());
    }
    self.seek_outside_current(pos)
  }
  fn length(&self) -> Result<usize> {
    Ok(self.view.length)
  }
  fn slice(&self, description: &str, offset: usize, length: usize) -> Result<Self> {
    if offset
      .checked_add(length)
      .is_none_or(|end| end > self.view.length)
    {
      return Err(LuceneError::illegal_argument(format!(
        "slice() {description} out of bounds: offset={offset},length={length},fileLength={}: {self}",
        self.view.length
      )));
    }
    if self.is_closed() {
      return Err(self.closed_error());
    }
    Ok(self.with_view(
      self.view.slice(offset, length)?,
      self.resource_desc_suffix.extend(description)?,
    ))
  }
  fn slice_with_read_advice(
    &self,
    description: &str,
    offset: usize,
    length: usize,
    advice: &ReadAdvice,
  ) -> Result<Self> {
    let slice = self.slice(description, offset, length)?;
    #[cfg(unix)]
    {
      if advice != &ReadAdvice::Normal
        && length >= self.native_access.get_page_size()
        && let Some(advice) = self.native_access.map_read_advice(advice)
      {
        slice.advise_first(0, length, |map, offset, length| {
          map.advise_range(advice, offset, length)
        })?;
      }
    }
    #[cfg(not(unix))]
    {
      let _ = advice;
    }
    Ok(slice)
  }
  fn random_access_slice(&self, offset: usize, length: usize) -> Result<Self::RandomAccessSlice> {
    Ok(MemorySegmentRandomAccessInput {
      input: self.slice("randomaccess", offset, length)?,
    })
  }
  fn prefetch(&self, pos: usize, len: usize) -> Result<()> {
    self.prefetch_impl(pos, len)
  }
  fn update_read_advice(&self, advice: ReadAdvice) -> Result<()> {
    #[cfg(unix)]
    {
      if self.is_closed() {
        return Err(self.closed_error());
      }
      self.view.directory_open()?;
      if let Some(advice) = self.native_access.map_read_advice(&advice) {
        let mut offset = 0;
        for index in 0..self.view.count() {
          let len = self.view.segment(index).ok_or_else(MappedView::eof)?.len();
          self.advise_first(offset, len, |map, offset, length| {
            map.advise_range(advice, offset, length)
          })?;
          offset += len;
        }
      }
    }
    #[cfg(not(unix))]
    {
      let _ = advice;
    }
    Ok(())
  }
  fn is_loaded(&self) -> Result<Option<bool>> {
    if self.is_closed_for_reading() {
      return Err(self.closed_error());
    }
    self.view.directory_open()?;
    #[cfg(unix)]
    {
      for index in 0..self.view.count() {
        if let Some((map, start, length)) = self.view.mapped_segment(index)?
          && !self
            .native_access
            .is_loaded(map, start, length)
            .map_err(LuceneError::io)?
        {
          return Ok(Some(false));
        }
      }
      Ok(Some(true))
    }
    #[cfg(not(unix))]
    {
      Ok(None)
    }
  }
}
impl RandomAccessInput for MemorySegmentIndexInput {
  fn length(&self) -> Result<usize> {
    Ok(self.view.length)
  }
  fn read_byte(&self, pos: usize) -> Result<u8> {
    if self.is_closed_for_reading() {
      return Err(self.closed_error());
    }
    if let SegmentRange::Single {
      index,
      start,
      length,
    } = self.view.range
    {
      if pos >= length {
        return Err(MappedView::eof());
      }
      return Ok(self.view.file.segments[index][start + pos]);
    }
    self.view.directory_open()?;
    let (index, offset) = self.view.coordinates(pos)?;
    self
      .view
      .segment(index)
      .ok_or_else(MappedView::eof)?
      .get(offset)
      .copied()
      .ok_or_else(MappedView::eof)
  }
  fn read_short(&self, pos: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(self.read_absolute(pos)?))
  }
  fn read_int(&self, pos: usize) -> Result<i32> {
    Ok(i32::from_le_bytes(self.read_absolute(pos)?))
  }
  fn read_long(&self, pos: usize) -> Result<i64> {
    Ok(i64::from_le_bytes(self.read_absolute(pos)?))
  }
  fn read_bytes(&self, pos: usize, len: usize) -> Result<Cow<'_, [u8]>> {
    if self.is_closed_for_reading() {
      return Err(self.closed_error());
    }
    self.view.directory_open()?;
    if pos
      .checked_add(len)
      .is_none_or(|end| end > self.view.length)
    {
      return Err(MappedView::eof());
    }
    let (index, offset) = self.view.coordinates(pos)?;
    let segment = self.view.segment(index).ok_or_else(MappedView::eof)?;
    if let Some(bytes) = segment.get(offset..).and_then(|s| s.get(..len)) {
      return Ok(Cow::Borrowed(bytes));
    }
    let mut bytes = vec![0; len];
    let mut state = SegmentState {
      index,
      current: CurrentSegment {
        mapping: None,
        base: 0,
        len: 0,
      },
      seek_start: 0,
      seek_count: 0,
      seek_position: 0,
    };
    let mut position = offset;
    let mut cursor = SegmentCursor {
      state: &mut state,
      position: &mut position,
    };
    self.view.load_segment(&mut cursor, index)?;
    self.view.read_bytes(&mut cursor, &mut bytes, 0, len)?;
    Ok(Cow::Owned(bytes))
  }
  fn prefetch(&self, pos: usize, len: usize) -> Result<()> {
    self.prefetch_impl(pos, len)
  }
  fn is_loaded(&self) -> Result<Option<bool>> {
    IndexInput::is_loaded(self)
  }
}
impl RandomAccessInput for MemorySegmentRandomAccessInput {
  fn length(&self) -> Result<usize> {
    RandomAccessInput::length(&self.input)
  }
  fn read_byte(&self, pos: usize) -> Result<u8> {
    RandomAccessInput::read_byte(&self.input, pos)
  }
  fn read_short(&self, pos: usize) -> Result<i16> {
    RandomAccessInput::read_short(&self.input, pos)
  }
  fn read_int(&self, pos: usize) -> Result<i32> {
    RandomAccessInput::read_int(&self.input, pos)
  }
  fn read_long(&self, pos: usize) -> Result<i64> {
    RandomAccessInput::read_long(&self.input, pos)
  }
  fn read_bytes(&self, pos: usize, len: usize) -> Result<Cow<'_, [u8]>> {
    RandomAccessInput::read_bytes(&self.input, pos, len)
  }
  fn prefetch(&self, pos: usize, len: usize) -> Result<()> {
    RandomAccessInput::prefetch(&self.input, pos, len)
  }
  fn is_loaded(&self) -> Result<Option<bool>> {
    RandomAccessInput::is_loaded(&self.input)
  }
}
