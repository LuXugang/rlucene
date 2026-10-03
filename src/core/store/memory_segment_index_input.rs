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
use crate::core::util::error::Eof;
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
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

/// Immutable mappings are owned once for the whole file. A borrowed random read
/// stays valid while an input retains its owner. The last owner releases the mappings.
struct MappedFile {
  resource_desc: Box<str>,
  segments: Box<[Arc<Mmap>]>,
}

struct SegmentState {
  index: usize,
  // Private stable view retained by the input's immutable MappedFile.
  current: &'static [u8],
}

impl SegmentState {
  fn current_byte(&self, pos: usize) -> Option<u8> {
    self.current.get(pos).copied()
  }

  fn current_range(&self, pos: usize, len: usize) -> Option<&[u8]> {
    if len > self.current.len().checked_sub(pos)? {
      return None;
    }
    self.current.get(pos..pos + len)
  }

  fn current_slice(&self) -> &[u8] {
    self.current
  }

  fn new(file: &MappedFile, range: (usize, usize, u32)) -> Self {
    let (start, length, power) = range;
    let mask = (1usize << power) - 1;
    let first = start >> power;
    let offset = start & mask;
    let single = (offset + length) >> power == 0;
    let begin = if single { offset } else { 0 };
    let end = if single {
      begin + length
    } else {
      file.segments.get(first).map_or(0, |m| m.len())
    };
    let current_slice = file
      .segments
      .get(first)
      .map_or(&[][..], |mapping| &mapping[begin..end]);
    // SAFETY: callers retain file in the enclosing input. Mmap allocations are
    // immutable and stable; the private view is not exposed with its extended
    // lifetime. Public/current_slice borrows stay tied to the retaining input.
    let current = unsafe { &*(current_slice as *const [u8]) };
    SegmentState { index: 0, current }
  }
}

/// A mapped input that retains its backing independently of other views.
///
/// `close()` is a compatibility no-op. Clones and slices remain readable after
/// another view is dropped; the mappings are released with their last owner.
// Keep frequently accessed cursor fields together instead of allowing the
// default field reordering to spread them across the object.
#[repr(C)]
pub struct MemorySegmentIndexInput {
  // Keep the written position apart from the immutable current range so that
  // a paired load does not span the position store on sequential reads.
  position: AtomicUsize,
  state: Mutex<SegmentState>,
  single_segment: bool,
  consecutive_prefetch_hit_count: AtomicI32,
  // Immutable mapping and visible-range information need no cursor lock.
  file: Arc<MappedFile>,
  start: usize,
  length: usize,
  power: u32,
  resource_desc_suffix: ResourceDescriptionSuffix,
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

impl MemorySegmentIndexInput {
  #[cold]
  fn read_single_scalar_eof(&mut self) -> Eof {
    let state = self.state.get_mut();
    let position = self.position.get_mut();
    // Consume the visible tail before the failed directory advance, exactly
    // as the byte-wise scalar fallback does for a partial Single element.
    if *position < state.current.len() {
      *position = state.current.len();
    }
    Self::advance_segment(state);
    Self::scalar_eof()
  }
  #[cold]
  fn scalar_eof() -> Eof {
    Eof::new("read past EOF")
  }
  fn advance_segment(state: &mut SegmentState) {
    // Failed advances still retain the old data and position.
    state.index += 1;
  }

  fn load_segment(
    file: &MappedFile,
    range: (usize, usize, u32),
    state: &mut SegmentState,
    index: usize,
  ) -> Result<()> {
    let (start, length, power) = range;
    let mask = (1usize << power) - 1;
    let first = start >> power;
    let offset = start & mask;
    let last = (offset + length) >> power;
    if index > last {
      return Err(Self::eof());
    }
    let physical = first + index;
    let start = if last == 0 { offset } else { 0 };
    let end = if last == 0 {
      start + length
    } else if index == last {
      (offset + length) & mask
    } else {
      file.segments.get(physical).map_or(0, |m| m.len())
    };
    // Resolve first: a failed directory advance must retain the old mapping.
    let current_slice = file
      .segments
      .get(physical)
      .map_or(&[][..], |mapping| &mapping[start..end]);
    // SAFETY: the input or local file borrow retains every mapping used by this
    // private cursor. Replacing the view never changes the backing allocation;
    // any returned public slice has the lifetime of the retaining input borrow.
    state.current = unsafe { &*(current_slice as *const [u8]) };
    Ok(())
  }

  fn count(&self) -> usize {
    (((self.start & ((1usize << self.power) - 1)) + self.length) >> self.power) + 1
  }
  fn eof() -> LuceneError {
    LuceneError::eof("read past EOF")
  }

  /// Returns the physical mapping and its visible byte range. An empty trailing
  /// segment has no mapping, so empty files need no OS mapping or dummy buffer.
  #[cfg(unix)]
  fn mapped_segment(&self, index: usize) -> Result<Option<(&Mmap, usize, usize)>> {
    let mask = (1usize << self.power) - 1;
    let offset = self.start & mask;
    let last = (offset + self.length) >> self.power;
    if index > last {
      return Err(Self::eof());
    }
    let start = if self.single_segment { offset } else { 0 };
    let length = if self.single_segment {
      self.length
    } else if index == last {
      (offset + self.length) & mask
    } else {
      self
        .file
        .segments
        .get((self.start >> self.power) + index)
        .map_or(0, |m| m.len())
    };
    Ok(
      self
        .file
        .segments
        .get((self.start >> self.power) + index)
        .map(|m| (m.as_ref(), start, length)),
    )
  }
  fn segment(&self, index: usize) -> Option<&[u8]> {
    let mask = (1usize << self.power) - 1;
    let offset = self.start & mask;
    let last = (offset + self.length) >> self.power;
    if index > last {
      return None;
    }
    Some(
      self
        .file
        .segments
        .get((self.start >> self.power) + index)
        .map_or(&[], |m| {
          if self.single_segment {
            &m[offset..offset + self.length]
          } else if index == last {
            &m[..(offset + self.length) & mask]
          } else {
            &m[..]
          }
        }),
    )
  }
  fn coordinates(&self, pos: usize) -> Result<(usize, usize)> {
    if self.single_segment {
      Ok((0, pos))
    } else {
      let mask = (1usize << self.power) - 1;
      let absolute = pos.checked_add(self.start & mask).ok_or_else(Self::eof)?;
      Ok((absolute >> self.power, absolute & mask))
    }
  }
  #[cold]
  fn read_byte_boundary(
    file: &MappedFile,
    range: (usize, usize, u32),
    state: &mut SegmentState,
    position: &mut usize,
  ) -> Result<u8> {
    let last = ((range.0 & ((1usize << range.2) - 1)) + range.1) >> range.2;
    loop {
      Self::advance_segment(state);
      if state.index > last {
        return Err(Self::eof());
      }
      let index = state.index;
      Self::load_segment(file, range, state, index)?;
      *position = 0;
      if let Some(value) = state.current_byte(0) {
        *position = 1;
        return Ok(value);
      }
    }
  }
  fn read_scalar<const N: usize>(
    file: &MappedFile,
    range: (usize, usize, u32),
    state: &mut SegmentState,
    position: &mut usize,
  ) -> Result<[u8; N]> {
    if let Some(bytes) = state
      .current_range(*position, N)
      .and_then(|s| s.first_chunk::<N>())
    {
      let bytes = *bytes;
      *position += N;
      return Ok(bytes);
    }
    let mut bytes = [0; N];
    let mut filled = 0;
    while filled < N {
      let segment = state.current_slice();
      if let Some(remaining) = segment.get(*position..)
        && !remaining.is_empty()
      {
        let count = remaining.len().min(N - filled);
        bytes[filled..filled + count].copy_from_slice(&remaining[..count]);
        *position += count;
        filled += count;
      } else {
        // Keep readByte's directory/EOF transition, including the old loaded
        // segment when advancing beyond the directory fails.
        bytes[filled] = Self::read_byte_boundary(file, range, state, position)?;
        filled += 1;
      }
    }
    Ok(bytes)
  }
  #[cold]
  fn read_scalar_boundary<const N: usize>(&mut self) -> Result<[u8; N]> {
    Self::read_scalar(
      &self.file,
      (self.start, self.length, self.power),
      self.state.get_mut(),
      self.position.get_mut(),
    )
  }
  fn copy(
    segment: &SegmentState,
    pos: usize,
    b: &mut [u8],
    offset: usize,
    len: usize,
  ) -> Result<()> {
    let source = segment.current_range(pos, len).ok_or_else(Self::eof)?;
    let target = b
      .get_mut(offset..)
      .and_then(|s| s.get_mut(..len))
      .ok_or_else(|| LuceneError::illegal_argument("destination range out of bounds"))?;
    target.copy_from_slice(source);
    Ok(())
  }
  #[cold]
  fn read_bytes_from_segments(
    file: &MappedFile,
    range: (usize, usize, u32),
    state: &mut SegmentState,
    position: &mut usize,
    destination: (&mut [u8], usize, usize),
  ) -> Result<()> {
    let (b, mut offset, mut len) = destination;
    let last = ((range.0 & ((1usize << range.2) - 1)) + range.1) >> range.2;
    loop {
      let available = state
        .current
        .len()
        .checked_sub(*position)
        .ok_or_else(|| LuceneError::array_index_out_of_bounds("source position out of bounds"))?;
      if len <= available {
        Self::copy(state, *position, b, offset, len)?;
        *position += len;
        return Ok(());
      }
      Self::copy(state, *position, b, offset, available)?;
      len -= available;
      offset += available;
      Self::advance_segment(state);
      if state.index > last {
        return Err(Self::eof());
      }
      Self::load_segment(file, range, state, state.index)?;
      *position = 0;
    }
  }
}

impl MemorySegmentIndexInput {
  #[cold]
  fn seek_outside_current(&mut self, pos: usize) -> Result<()> {
    let result = if self.single_segment {
      Err(Self::eof())
    } else {
      (|| {
        let (index, offset) = self.coordinates(pos)?;
        if index != self.state.get_mut().index {
          Self::load_segment(
            &self.file,
            (self.start, self.length, self.power),
            self.state.get_mut(),
            index,
          )?;
          let state = self.state.get_mut();
          state.index = index;
        }
        if offset > self.state.get_mut().current.len() {
          return Err(Self::eof());
        }
        *self.position.get_mut() = offset;
        Ok(())
      })()
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

    let last = length >> chunk_size_power;
    let file = Arc::new(MappedFile {
      resource_desc: resource_desc.into_boxed_str(),
      segments: segments.into_boxed_slice(),
    });
    let power = chunk_size_power;
    let single_segment = last == 0;
    let position = AtomicUsize::new(0);
    let state = Mutex::new(SegmentState::new(&file, (0, length, power)));
    Ok(Self {
      resource_desc_suffix: ResourceDescriptionSuffix::new(),
      file,
      start: 0,
      length,
      power,
      position,
      state,
      single_segment,
      consecutive_prefetch_hit_count: AtomicI32::new(0),
      #[cfg(unix)]
      native_access,
    })
  }
  fn file_pointer(&self) -> usize {
    if self.single_segment {
      self.position.load(Ordering::Relaxed)
    } else {
      let state = self.state.lock();
      ((state.index << self.power) + self.position.load(Ordering::Relaxed))
        .wrapping_sub(self.start & ((1usize << self.power) - 1))
    }
  }
  fn with_range(
    &self,
    first: usize,
    last: usize,
    offset: usize,
    length: usize,
    suffix: ResourceDescriptionSuffix,
  ) -> Self {
    let single_segment = last == 0;
    let position = AtomicUsize::new(if single_segment { 0 } else { offset });
    let start = (first << self.power) + offset;
    let state = Mutex::new(SegmentState::new(&self.file, (start, length, self.power)));
    Self {
      resource_desc_suffix: suffix,
      file: self.file.clone(),
      start,
      length,
      power: self.power,
      position,
      state,
      single_segment,
      consecutive_prefetch_hit_count: AtomicI32::new(0),
      #[cfg(unix)]
      native_access: self.native_access,
    }
  }
  // Byte copies preserve all integer and floating-point bit patterns on a
  // little-endian target. Small copies use constant widths to avoid a library
  // call; larger copies retain the platform memcpy implementation.
  #[cfg(target_endian = "little")]
  #[inline]
  unsafe fn copy_array_bytes(source: &[u8], dst: *mut u8) {
    let len = source.len();
    let src = source.as_ptr();
    // SAFETY: callers pass a disjoint, writable destination of exactly len
    // bytes. Each head/tail copy stays within both ranges; separate copies may
    // overlap each other in the destination, but never their own source.
    unsafe {
      match len {
        0 => {},
        4..=7 => {
          std::ptr::copy_nonoverlapping(src, dst, 4);
          std::ptr::copy_nonoverlapping(src.add(len - 4), dst.add(len - 4), 4);
        },
        8..=15 => {
          std::ptr::copy_nonoverlapping(src, dst, 8);
          std::ptr::copy_nonoverlapping(src.add(len - 8), dst.add(len - 8), 8);
        },
        16..=32 => {
          std::ptr::copy_nonoverlapping(src, dst, 16);
          std::ptr::copy_nonoverlapping(src.add(len - 16), dst.add(len - 16), 16);
        },
        33..=80 => {
          std::ptr::copy_nonoverlapping(src, dst, 32);
          std::ptr::copy_nonoverlapping(src.add(len - 32), dst.add(len - 32), 32);
          if len > 64 {
            // The middle fills the gap between the checked head and tail.
            std::ptr::copy_nonoverlapping(src.add(32), dst.add(32), 16);
          }
        },
        _ => std::ptr::copy_nonoverlapping(src, dst, len),
      }
    }
  }
  // SAFETY contract: only the three primitive callers below use this helper.
  // T is i32/f32 with N=4, or i64 with N=8; all bit patterns are valid and
  // decode is the corresponding from_le_bytes operation.
  #[cold]
  unsafe fn read_array_boundary<T, F, const N: usize>(
    &mut self,
    dst: &mut [T],
    offset: usize,
    len: usize,
    decode: F,
  ) -> Result<()>
  where
    F: Fn([u8; N]) -> T,
  {
    CoreHelper::check_from_index_size(offset, len, dst.len())?;
    let mut dst = &mut dst[offset..offset + len];
    while !dst.is_empty() {
      let state = self.state.get_mut();
      let position = self.position.get_mut();
      let segment = state.current_slice();
      let count = (segment.len().saturating_sub(*position) / N).min(dst.len());
      if count == 0 {
        if *position >= segment.len() {
          // A complete element in the next segment does not need byte-wise
          // decoding. Preserve readByte's order of directory and cursor updates.
          loop {
            Self::advance_segment(state);
            if state.index
              > (((self.start & ((1usize << self.power) - 1)) + self.length) >> self.power)
            {
              return Err(Self::eof());
            }
            let index = state.index;
            Self::load_segment(
              &self.file,
              (self.start, self.length, self.power),
              state,
              index,
            )?;
            *position = 0;
            if !state.current_slice().is_empty() {
              break;
            }
          }
          continue;
        }
        // Only the element crossing the boundary needs scalar fallback. If it
        // fails, earlier complete elements and its partial cursor progress stay.
        let bytes = Self::read_scalar(
          &self.file,
          (self.start, self.length, self.power),
          state,
          position,
        )?;
        dst[0] = decode(bytes);
        dst = &mut dst[1..];
      } else {
        let bytes = &segment[*position..*position + count * N];
        #[cfg(target_endian = "little")]
        // SAFETY: count complete elements fit both slices. The primitive type
        // contract above makes the byte copy valid, including every float bit
        // pattern. The retained read-only mapping is disjoint from dst.
        unsafe {
          let target = dst.as_mut_ptr().cast::<u8>();
          if (65..=128).contains(&bytes.len()) {
            // This range also covers a short long-array tail without a memcpy
            // call. Both 64-byte copies stay within the complete elements.
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, 64);
            std::ptr::copy_nonoverlapping(
              bytes.as_ptr().add(bytes.len() - 64),
              target.add(bytes.len() - 64),
              64,
            );
          } else {
            Self::copy_array_bytes(bytes, target);
          }
        }
        #[cfg(target_endian = "big")]
        for (value, chunk) in dst[..count].iter_mut().zip(bytes.as_chunks::<N>().0) {
          *value = decode(*chunk);
        }
        *position += count * N;
        dst = &mut dst[count..];
      }
    }
    Ok(())
  }
  #[inline]
  fn single_slice(&self) -> &[u8] {
    debug_assert!(self.single_segment);
    // SAFETY: on a Single input, shared methods never replace the current view.
    // All Single cursor/view updates require &mut self, excluding shared reads.
    // The two shared cursor-locking paths are restricted to Multi inputs.
    // The private slice remains backed by the input's Arc<MappedFile> and the
    // returned borrow cannot outlive this input. Reading it needs no mutex.
    unsafe { (&*self.state.data_ptr()).current_slice() }
  }
  #[inline(never)]
  fn read_short_multi(
    file: &MappedFile,
    range: (usize, usize, u32),
    cursor: (&Mutex<SegmentState>, &AtomicUsize),
    pos: usize,
  ) -> Result<i16> {
    Ok(i16::from_le_bytes(Self::read_absolute_multi::<2>(
      file, range, cursor, pos,
    )?))
  }
  #[inline(never)]
  fn read_int_multi(
    file: &MappedFile,
    range: (usize, usize, u32),
    cursor: (&Mutex<SegmentState>, &AtomicUsize),
    pos: usize,
  ) -> Result<i32> {
    Ok(i32::from_le_bytes(Self::read_absolute_multi::<4>(
      file, range, cursor, pos,
    )?))
  }
  #[inline(never)]
  fn read_long_multi(
    file: &MappedFile,
    range: (usize, usize, u32),
    cursor: (&Mutex<SegmentState>, &AtomicUsize),
    pos: usize,
  ) -> Result<i64> {
    Ok(i64::from_le_bytes(Self::read_absolute_multi::<8>(
      file, range, cursor, pos,
    )?))
  }
  fn read_absolute_multi<const N: usize>(
    file: &MappedFile,
    range: (usize, usize, u32),
    cursor: (&Mutex<SegmentState>, &AtomicUsize),
    pos: usize,
  ) -> Result<[u8; N]> {
    let (start, length, power) = range;
    let mask = (1usize << power) - 1;
    let absolute = pos.checked_add(start & mask).ok_or_else(Self::eof)?;
    let (index, offset) = (absolute >> power, absolute & mask);
    let last = ((start & mask) + length) >> power;
    if index > last {
      return Err(Self::eof());
    }
    let segment = file
      .segments
      .get((start >> power) + index)
      .map_or(&[][..], |m| {
        if index == last {
          &m[..((start & mask) + length) & mask]
        } else {
          &m[..]
        }
      });
    if let Some(bytes) = segment.get(offset..).and_then(|s| s.first_chunk::<N>()) {
      return Ok(*bytes);
    }
    let mut state = cursor.0.lock();
    let mut position = offset;
    let result = {
      Self::load_segment(file, range, &mut state, index)?;
      state.index = index;
      Self::read_scalar(file, range, &mut state, &mut position)
    };
    cursor.1.store(position, Ordering::Relaxed);
    result
  }
  #[cfg(unix)]
  fn advise_first<F>(&self, pos: usize, length: usize, advice: F) -> Result<()>
  where
    F: FnOnce(&Mmap, usize, usize) -> std::io::Result<()>,
  {
    CoreHelper::check_from_index_size(pos, length, self.length)?;
    // Advice uses segment-directory coordinates, even on a multi slice.
    let index = pos >> self.power;
    let offset = pos & ((1usize << self.power) - 1);
    let Some((map, start, visible)) = self.mapped_segment(index)? else {
      return Ok(());
    };
    if offset > visible {
      return Err(Self::eof());
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
    advice(map, physical_offset, length).map_err(LuceneError::io)
  }
  fn prefetch_impl(&self, pos: usize, len: usize) -> Result<()> {
    #[cfg(unix)]
    {
      CoreHelper::check_from_index_size(pos, len, self.length)?;
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
  #[inline]
  fn read_byte(&mut self) -> Result<u8> {
    if self.single_segment {
      let position = self.position.get_mut();
      let data = self.state.get_mut().current;
      if let Some(value) = data.get(*position).copied() {
        *position += 1;
        return Ok(value);
      }
      return Err(LuceneError::Eof(self.read_single_scalar_eof()));
    }
    let position = self.position.get_mut();
    let state = self.state.get_mut();
    if *position < state.current.len() {
      // SAFETY: the input's MappedFile retains this checked immutable range.
      let value = unsafe { *state.current.as_ptr().add(*position) };
      *position += 1;
      return Ok(value);
    }
    Self::read_byte_boundary(
      &self.file,
      (self.start, self.length, self.power),
      state,
      position,
    )
  }

  fn read_bytes(&mut self, b: &mut [u8], offset: usize, len: usize) -> Result<()> {
    let position = self.position.get_mut();
    let state = self.state.get_mut();
    if let Some(remaining) = state.current.len().checked_sub(*position)
      && len <= remaining
      && let Some(target) = b.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      // SAFETY: checked_sub and len <= remaining prove the source range lies
      // inside the immutable mapping retained by the input. target is a checked,
      // disjoint mutable range of the same length. Byte pointers need no
      // alignment; each constant-width head/tail copy stays in these ranges.
      // The overlapping head/tail destinations are separate copy operations,
      // while each operation's source and destination remain disjoint.
      unsafe {
        let src = state.current.as_ptr().add(*position);
        let dst = target.as_mut_ptr();
        if len <= 32 {
          match len {
            0 => {},
            1 => std::ptr::copy_nonoverlapping(src, dst, 1),
            2..=3 => {
              std::ptr::copy_nonoverlapping(src, dst, 2);
              std::ptr::copy_nonoverlapping(src.add(len - 2), dst.add(len - 2), 2);
            },
            4..=7 => {
              std::ptr::copy_nonoverlapping(src, dst, 4);
              std::ptr::copy_nonoverlapping(src.add(len - 4), dst.add(len - 4), 4);
            },
            8..=15 => {
              std::ptr::copy_nonoverlapping(src, dst, 8);
              std::ptr::copy_nonoverlapping(src.add(len - 8), dst.add(len - 8), 8);
            },
            _ => {
              std::ptr::copy_nonoverlapping(src, dst, 16);
              std::ptr::copy_nonoverlapping(src.add(len - 16), dst.add(len - 16), 16);
            },
          }
        } else {
          std::ptr::copy_nonoverlapping(src, dst, len);
        }
      }
      *position += len;
      return Ok(());
    }
    Self::read_bytes_from_segments(
      &self.file,
      (self.start, self.length, self.power),
      self.state.get_mut(),
      self.position.get_mut(),
      (b, offset, len),
    )
  }
  #[inline]
  fn read_short(&mut self) -> Result<i16> {
    if self.single_segment {
      let position = self.position.get_mut();
      let data = self.state.get_mut().current;
      if let Some(bytes) = data.get(*position..).and_then(|s| s.first_chunk::<2>()) {
        let value = i16::from_le_bytes(*bytes);
        *position += 2;
        return Ok(value);
      }
      return Err(LuceneError::Eof(self.read_single_scalar_eof()));
    }
    let position = self.position.get_mut();
    let state = self.state.get_mut();
    let segment = state.current_slice();
    if let Some(bytes) = segment.get(*position..).and_then(|s| s.first_chunk::<2>()) {
      let value = i16::from_le_bytes(*bytes);
      *position += 2;
      return Ok(value);
    }
    Ok(i16::from_le_bytes(self.read_scalar_boundary::<2>()?))
  }

  #[inline]
  fn read_int(&mut self) -> Result<i32> {
    if self.single_segment {
      let position = self.position.get_mut();
      let data = self.state.get_mut().current;
      if let Some(bytes) = data.get(*position..).and_then(|s| s.first_chunk::<4>()) {
        let value = i32::from_le_bytes(*bytes);
        *position += 4;
        return Ok(value);
      }
      return Err(LuceneError::Eof(self.read_single_scalar_eof()));
    }
    let position = self.position.get_mut();
    let state = self.state.get_mut();
    let segment = state.current_slice();
    if let Some(bytes) = segment.get(*position..).and_then(|s| s.first_chunk::<4>()) {
      let value = i32::from_le_bytes(*bytes);
      *position += 4;
      return Ok(value);
    }
    Ok(i32::from_le_bytes(self.read_scalar_boundary::<4>()?))
  }

  #[inline]
  fn read_long(&mut self) -> Result<i64> {
    if self.single_segment {
      let position = self.position.get_mut();
      let data = self.state.get_mut().current;
      if let Some(bytes) = data.get(*position..).and_then(|s| s.first_chunk::<8>()) {
        let value = i64::from_le_bytes(*bytes);
        *position += 8;
        return Ok(value);
      }
      return Err(LuceneError::Eof(self.read_single_scalar_eof()));
    }
    let position = self.position.get_mut();
    let state = self.state.get_mut();
    let segment = state.current_slice();
    if let Some(bytes) = segment.get(*position..).and_then(|s| s.first_chunk::<8>()) {
      let value = i64::from_le_bytes(*bytes);
      *position += 8;
      return Ok(value);
    }
    Ok(i64::from_le_bytes(self.read_scalar_boundary::<8>()?))
  }

  fn read_group_vint(&mut self, dst: &mut [i32], offset: usize) -> Result<()> {
    let state = self.state.get_mut();
    let position = self.position.get_mut();
    let remaining = state.current.len().saturating_sub(*position);
    let pos = *position;
    let len = GroupVIntUtil::read_group_vint_i32_with_reader(self, remaining, pos, dst, offset)?;
    *self.position.get_mut() += len;
    Ok(())
  }
  #[inline]
  fn read_ints(&mut self, dst: &mut [i32], offset: usize, len: usize) -> Result<()> {
    let state = self.state.get_mut();
    let position = self.position.get_mut();
    let Some(byte_len) = len.checked_mul(4) else {
      return Err(LuceneError::Eof(Self::scalar_eof()));
    };
    if let Some(bytes) = state.current_range(*position, byte_len)
      && let Some(target) = dst.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      #[cfg(target_endian = "little")]
      // SAFETY: both slices cover exactly byte_len bytes. The mapped source is
      // read-only and cannot overlap the caller's mutable destination. i32 has
      // no padding and every bit pattern is valid; byte alignment is sufficient.
      unsafe {
        Self::copy_array_bytes(bytes, target.as_mut_ptr().cast::<u8>());
      }
      #[cfg(target_endian = "big")]
      for (value, chunk) in target.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *value = i32::from_le_bytes(*chunk);
      }
      *position += byte_len;
      return Ok(());
    }
    // SAFETY: this primitive has the checked element width and all bit
    // patterns are valid; the boundary helper retains scalar EOF behavior.
    unsafe { self.read_array_boundary(dst, offset, len, i32::from_le_bytes) }
  }

  #[inline]
  fn read_longs(&mut self, dst: &mut [i64], offset: usize, len: usize) -> Result<()> {
    let state = self.state.get_mut();
    let position = self.position.get_mut();
    let Some(byte_len) = len.checked_mul(8) else {
      return Err(LuceneError::Eof(Self::scalar_eof()));
    };
    if let Some(bytes) = state.current_range(*position, byte_len)
      && let Some(target) = dst.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      #[cfg(target_endian = "little")]
      // SAFETY: both slices cover exactly byte_len bytes. The mapped source is
      // read-only and cannot overlap the caller's mutable destination. i64 has
      // no padding and every bit pattern is valid; byte alignment is sufficient.
      unsafe {
        Self::copy_array_bytes(bytes, target.as_mut_ptr().cast::<u8>());
      }
      #[cfg(target_endian = "big")]
      for (value, chunk) in target.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *value = i64::from_le_bytes(*chunk);
      }
      *position += byte_len;
      return Ok(());
    }
    // SAFETY: this primitive has the checked element width and all bit
    // patterns are valid; the boundary helper retains scalar EOF behavior.
    unsafe { self.read_array_boundary(dst, offset, len, i64::from_le_bytes) }
  }

  #[inline]
  fn read_floats(&mut self, dst: &mut [f32], offset: usize, len: usize) -> Result<()> {
    let state = self.state.get_mut();
    let position = self.position.get_mut();
    let Some(byte_len) = len.checked_mul(4) else {
      return Err(LuceneError::Eof(Self::scalar_eof()));
    };
    if let Some(bytes) = state.current_range(*position, byte_len)
      && let Some(target) = dst.get_mut(offset..).and_then(|s| s.get_mut(..len))
    {
      #[cfg(target_endian = "little")]
      // SAFETY: both slices cover exactly byte_len bytes. The mapped source is
      // read-only and cannot overlap the caller's mutable destination. f32 has
      // no padding and every bit pattern is valid; byte alignment is sufficient.
      unsafe {
        Self::copy_array_bytes(bytes, target.as_mut_ptr().cast::<u8>());
      }
      #[cfg(target_endian = "big")]
      for (value, chunk) in target.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *value = f32::from_le_bytes(*chunk);
      }
      *position += byte_len;
      return Ok(());
    }
    // SAFETY: this primitive has the checked element width and all bit
    // patterns are valid; the boundary helper retains scalar EOF behavior.
    unsafe { self.read_array_boundary(dst, offset, len, f32::from_le_bytes) }
  }

  #[inline]
  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    IndexInput::skip_bytes(self, num_bytes)
  }
}
impl DataInputExt for MemorySegmentIndexInput {
  crate::core::store::data_input_ext::impl_index_input_ext!();
}
impl IntReader for MemorySegmentIndexInput {
  fn read(&mut self, pos: usize) -> Result<i32> {
    // Every input already caches its visible mapping range.
    let state = self.state.get_mut();
    let bytes = state
      .current_range(pos, 4)
      .and_then(|s| s.first_chunk::<4>())
      .ok_or_else(Self::eof)?;
    Ok(i32::from_le_bytes(*bytes))
  }
}
impl Display for MemorySegmentIndexInput {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.file.resource_desc)?;
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
// Mapped inputs retain their backing through Arc. Closing one view does not
// invalidate another; the last owner drops and unmaps the backing.
impl CloseableRef for MemorySegmentIndexInput {}

impl TryClone for MemorySegmentIndexInput {
  fn try_clone(&self) -> Result<Self> {
    let mut clone = self.with_range(
      self.start >> self.power,
      ((self.start & ((1usize << self.power) - 1)) + self.length) >> self.power,
      self.start & ((1usize << self.power) - 1),
      self.length,
      self.resource_desc_suffix.clone(),
    );
    clone.seek(self.get_file_pointer()?)?;
    Ok(clone)
  }
}
impl IndexInput for MemorySegmentIndexInput {
  type IndexInput = Self;
  type RandomAccessSlice = MemorySegmentRandomAccessInput;
  #[inline]
  fn get_file_pointer(&self) -> Result<usize> {
    if self.single_segment {
      return Ok(self.position.load(Ordering::Relaxed));
    }
    Ok(self.file_pointer())
  }
  #[inline]
  fn seek(&mut self, pos: usize) -> Result<()> {
    if self.single_segment {
      if pos <= self.length {
        *self.position.get_mut() = pos;
        return Ok(());
      }
    } else {
      let mask = (1usize << self.power) - 1;
      if let Some(absolute) = pos.checked_add(self.start & mask) {
        let state = self.state.get_mut();
        let offset = absolute & mask;
        if absolute >> self.power == state.index && offset <= state.current.len() {
          *self.position.get_mut() = offset;
          return Ok(());
        }
      }
    }
    self.seek_outside_current(pos)
  }
  #[inline]
  fn skip_bytes(&mut self, num_bytes: i64) -> Result<()> {
    if num_bytes < 0 {
      return Err(LuceneError::illegal_argument(format!(
        "num_bytes must be >= 0, got {num_bytes}"
      )));
    }
    let num_bytes: usize = num_bytes.try_convert()?;
    let state = self.state.get_mut();
    let position = self.position.get_mut();
    // Every input uses the current segment's checked range here. At its end,
    // seek handles a partial EOF or advances to the next segment as required.
    if let Some(target) = position.checked_add(num_bytes)
      && target < state.current.len()
    {
      *position = target;
      return Ok(());
    }
    // An exclusive borrow already protects the segment index and position.
    let position = if self.single_segment {
      *position
    } else {
      ((state.index << self.power) + *position)
        .wrapping_sub(self.start & ((1usize << self.power) - 1))
    };
    self.seek(position + num_bytes)
  }
  fn length(&self) -> Result<usize> {
    Ok(self.length)
  }
  fn slice(&self, description: &str, offset: usize, length: usize) -> Result<Self> {
    if offset
      .checked_add(length)
      .is_none_or(|end| end > self.length)
    {
      return Err(LuceneError::illegal_argument(format!(
        "slice() {description} out of bounds: offset={offset},length={length},fileLength={}: {self}",
        self.length
      )));
    }
    let start = (self.start & ((1usize << self.power) - 1)) + offset;
    let suffix = self.resource_desc_suffix.extend(description)?;
    if self.single_segment {
      return Ok(self.with_range(self.start >> self.power, 0, start, length, suffix));
    }
    let end = start + length;
    let first_index = start >> self.power;
    let last_index = end >> self.power;
    let last = last_index - first_index;
    Ok(self.with_range(
      (self.start >> self.power) + first_index,
      last,
      start & ((1usize << self.power) - 1),
      length,
      suffix,
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
      if let Some(advice) = self.native_access.map_read_advice(&advice) {
        let mut offset = 0;
        for index in 0..self.count() {
          let len = self.segment(index).ok_or_else(Self::eof)?.len();
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
    #[cfg(unix)]
    {
      for index in 0..self.count() {
        if let Some((map, start, length)) = self.mapped_segment(index)?
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
    Ok(self.length)
  }
  fn read_byte(&self, pos: usize) -> Result<u8> {
    if self.single_segment {
      let index = self.start >> self.power;
      let start = self.start & ((1usize << self.power) - 1);
      let length = self.length;
      if pos >= length {
        return Err(Self::eof());
      }
      return Ok(self.file.segments[index][start + pos]);
    }
    let (index, offset) = self.coordinates(pos)?;
    self
      .segment(index)
      .ok_or_else(Self::eof)?
      .get(offset)
      .copied()
      .ok_or_else(Self::eof)
  }
  #[inline]
  fn read_short(&self, pos: usize) -> Result<i16> {
    if self.single_segment {
      if let Some(bytes) = self
        .single_slice()
        .get(pos..)
        .and_then(|s| s.first_chunk::<2>())
      {
        return Ok(i16::from_le_bytes(*bytes));
      }
      return Err(LuceneError::Eof(Self::scalar_eof()));
    }
    Self::read_short_multi(
      &self.file,
      (self.start, self.length, self.power),
      (&self.state, &self.position),
      pos,
    )
  }
  #[inline]
  fn read_int(&self, pos: usize) -> Result<i32> {
    if self.single_segment {
      if let Some(bytes) = self
        .single_slice()
        .get(pos..)
        .and_then(|s| s.first_chunk::<4>())
      {
        return Ok(i32::from_le_bytes(*bytes));
      }
      return Err(LuceneError::Eof(Self::scalar_eof()));
    }
    Self::read_int_multi(
      &self.file,
      (self.start, self.length, self.power),
      (&self.state, &self.position),
      pos,
    )
  }
  #[inline]
  fn read_long(&self, pos: usize) -> Result<i64> {
    if self.single_segment {
      if let Some(bytes) = self
        .single_slice()
        .get(pos..)
        .and_then(|s| s.first_chunk::<8>())
      {
        return Ok(i64::from_le_bytes(*bytes));
      }
      return Err(LuceneError::Eof(Self::scalar_eof()));
    }
    Self::read_long_multi(
      &self.file,
      (self.start, self.length, self.power),
      (&self.state, &self.position),
      pos,
    )
  }
  fn read_bytes(&self, pos: usize, len: usize) -> Result<Cow<'_, [u8]>> {
    if pos.checked_add(len).is_none_or(|end| end > self.length) {
      return Err(Self::eof());
    }
    let (index, offset) = self.coordinates(pos)?;
    let segment = self.segment(index).ok_or_else(Self::eof)?;
    if let Some(bytes) = segment.get(offset..).and_then(|s| s.get(..len)) {
      return Ok(Cow::Borrowed(bytes));
    }
    let mut bytes = vec![0; len];
    let mut state = SegmentState {
      index,
      current: &[],
    };
    let mut position = offset;
    Self::load_segment(
      &self.file,
      (self.start, self.length, self.power),
      &mut state,
      index,
    )?;
    Self::read_bytes_from_segments(
      &self.file,
      (self.start, self.length, self.power),
      &mut state,
      &mut position,
      (&mut bytes, 0, len),
    )?;
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
  #[inline]
  fn read_short(&self, pos: usize) -> Result<i16> {
    RandomAccessInput::read_short(&self.input, pos)
  }
  #[inline]
  fn read_int(&self, pos: usize) -> Result<i32> {
    RandomAccessInput::read_int(&self.input, pos)
  }
  #[inline]
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
