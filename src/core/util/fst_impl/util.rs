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
use std::fmt;
use std::fmt::Display;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::store::DataInput;
use crate::core::util::access::{SharedAccessVec, WritableVec};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::fst::{
  ARCS_FOR_BINARY_SEARCH, ARCS_FOR_CONTINUOUS, ARCS_FOR_DIRECT_ADDRESSING, Arc, BitTable,
  BytesReader, END_LABEL, FST, InputType, read_end_arc, target_has_arcs,
};
use crate::core::util::fst_impl::fst_reader::FstReader;
use crate::core::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::ints_ref_builder::IntsRefBuilder;

pub struct Util;
impl Util {
  /// Looks up the output for this input, or null if the input is not
  /// accepted.
  pub fn get_ints<O, F, AV>(fst: &FST<O, F>, input: &IntsRef<AV>) -> Result<Option<O::V>>
  where
    O: Outputs,
    F: FstReader,
    AV: SharedAccessVec<i32>,
  {
    let mut arc = Arc::default();
    fst.get_first_arc(&mut arc);
    let mut fst_reader = fst.get_bytes_reader()?;
    let mut output = fst.outputs.get_no_output();

    for i in 0..input.length {
      let label = input.ints.access(|ints| ints[input.offset + i]);
      let found = fst.find_target_arc(label, &arc.clone(), &mut arc, &mut fst_reader)?;
      if found.is_none() {
        return Ok(None);
      }
      output = fst.outputs.add(&output, &arc.output());
    }

    if arc.is_final() {
      let final_output = fst.outputs.add(&output, &arc.next_final_output());
      Ok(Some(final_output))
    } else {
      Ok(None)
    }
  }
  /// Looks up the output for this input, or `None` if the input is not
  /// accepted.
  pub fn get_bytes<O, F, AV>(fst: &FST<O, F>, input: &BytesRef<AV>) -> Result<Option<O::V>>
  where
    O: Outputs,
    F: FstReader,
    AV: SharedAccessVec<u8>,
  {
    debug_assert_eq!(fst.metadata.input_type, InputType::Byte1);

    let mut fst_reader = fst.get_bytes_reader()?;
    let mut arc = Arc::<O::V>::default();
    fst.get_first_arc(&mut arc);
    let mut output = fst.outputs.get_no_output();

    for i in 0..input.length {
      let label = input.bytes.access(|bytes| bytes[input.offset + i] as i32);
      let found = fst.find_target_arc(label, &arc.clone(), &mut arc, &mut fst_reader)?;
      if found.is_none() {
        return Ok(None);
      }
      output = fst.outputs.add(&output, &arc.output());
    }

    if arc.is_final() {
      let final_output = fst.outputs.add(&output, &arc.next_final_output());
      Ok(Some(final_output))
    } else {
      Ok(None)
    }
  }
  pub fn get_utf32<AV>(s: &str, scratch: &mut IntsRefBuilder<AV>)
  where
    AV: SharedAccessVec<i32> + WritableVec<i32>,
  {
    let len = s.len();
    Self::get_utf32_with_slice(s, 0, len, scratch);
  }
  /// Decodes the Unicode codepoints from the provided `char[]` and places
  /// them into the provided scratch `IntsRef`, which must not be `None`,
  /// and returns it.
  pub fn get_utf32_with_slice<AV>(
    s: &str,
    offset: usize,
    length: usize,
    scratch: &mut IntsRefBuilder<AV>,
  ) where
    AV: SharedAccessVec<i32> + WritableVec<i32>,
  {
    let mut int_idx = 0;
    for c in s[offset..offset + length].chars() {
      scratch.grow(int_idx + 1);
      scratch.set_int_at(int_idx, c as i32);
      int_idx += 1;
    }
    scratch.set_length(int_idx);
  }
  /// Just takes unsigned byte values from the BytesRef and converts into an
  /// IntsRef.
  pub fn get_ints_ref<AV1, AV2>(input: &BytesRef<AV1>, scratch: &mut IntsRefBuilder<AV2>)
  where
    AV1: SharedAccessVec<u8>,
    AV2: SharedAccessVec<i32> + WritableVec<i32>,
  {
    scratch.grow_no_copy(input.length);
    for i in 0..input.length {
      input.bytes.access(|bytes| {
        let byte = bytes[input.offset + i];
        scratch.set_int_at(i, byte as i32);
      })
    }
    scratch.set_length(input.length);
  }
  /// Just converts IntsRef to BytesRef; you must ensure the int values fit
  /// into a byte.
  pub fn get_bytes_ref<AV1, AV2>(
    input: &IntsRef<AV1>,
    scratch: &mut BytesRefBuilder<AV2>,
  ) -> Result<BytesRef<AV2>>
  where
    AV1: SharedAccessVec<i32>,
    AV2: SharedAccessVec<u8> + WritableVec<u8>,
  {
    scratch.grow(input.length);
    for i in 0..input.length {
      input.ints.access(|v| {
        let value = v[i + input.offset];
        debug_assert!(value >= u8::MIN as i32 && value <= u8::MAX as i32);
        scratch.set_byte_at(i, value as u8);
      })
    }
    scratch.set_length(input.length);
    Ok(scratch.get_bytes_owner())
  }

  pub fn read_ceil_arc<O, F>(
    label: i32,
    fst: &FST<O, F>,
    follow: &Arc<O::V>,
    arc: &mut Arc<O::V>,
    in_reader: &mut F::FstBytesReader,
  ) -> Result<Option<()>>
  where
    O: Outputs,
    F: FstReader,
  {
    if label == END_LABEL {
      read_end_arc(follow, arc);
      return Ok(Some(()));
    }

    if !target_has_arcs(follow) {
      return Ok(None);
    }

    fst.read_first_target_arc(follow, arc, in_reader)?;

    if arc.bytes_per_arc() != 0 && arc.label() != END_LABEL {
      match arc.node_flags() {
        ARCS_FOR_DIRECT_ADDRESSING => {
          let target_index = label - arc.label();
          if target_index >= arc.num_arcs() {
            return Ok(None);
          } else if target_index < 0 {
            return Ok(Some(()));
          }

          if BitTable::is_bit_set(target_index, arc, in_reader)? {
            fst.read_arc_by_direct_addressing(arc, in_reader, target_index)?;
            debug_assert_eq!(arc.label(), label);
          } else {
            let ceil_index = BitTable::next_bit_set(target_index, arc, in_reader)?;
            debug_assert!(ceil_index != -1);
            fst.read_arc_by_direct_addressing(arc, in_reader, ceil_index)?;
            debug_assert!(arc.label() > label);
          }
          return Ok(Some(()));
        },

        ARCS_FOR_CONTINUOUS => {
          let target_index = label - arc.label();
          return if target_index >= arc.num_arcs() {
            Ok(None)
          } else if target_index < 0 {
            Ok(Some(()))
          } else {
            fst.read_arc_by_continuous(arc, in_reader, target_index)?;
            debug_assert_eq!(arc.label(), label);
            Ok(Some(()))
          };
        },

        _ => {
          // Fixed length arcs in a binary search node.
          let mut idx = Self::binary_search(fst, arc, label)?;
          if idx >= 0 {
            fst.read_arc_by_index(arc, in_reader, idx)?;
          }
          idx = -1 - idx;
          if idx == arc.num_arcs() {
            // DEAD END!
            return Ok(None);
          }
        },
      }
    }

    // Variable length arcs in a linear scan list,
    // or special arc with label == FST.END_LABEL.
    fst.read_first_real_target_arc(follow.target(), arc, in_reader)?;
    loop {
      if arc.label() >= label {
        return Ok(Some(()));
      } else if arc.is_last() {
        return Ok(None);
      }
      fst.read_next_real_arc(arc, in_reader)?;
    }
  }

  pub fn binary_search<O, F>(fst: &FST<O, F>, arc: &Arc<O::V>, target_label: i32) -> Result<i32>
  where
    O: Outputs,
    F: FstReader,
  {
    debug_assert!(
      arc.node_flags() == ARCS_FOR_BINARY_SEARCH,
      "Arc is not encoded as packed array for binary search (nodeFlags={})",
      arc.node_flags()
    );

    let mut in_reader = fst.get_bytes_reader()?;
    let mut low = arc.arc_idx();
    let mut high = arc.num_arcs() - 1;

    while low <= high {
      let mid = (low + high) >> 1;

      in_reader.set_position(arc.pos_arcs_start() as usize);
      in_reader.skip_bytes((arc.bytes_per_arc() * mid + 1) as i64)?;

      let mid_label = fst.read_label(&mut in_reader)?;
      let cmp = mid_label - target_label;

      if cmp < 0 {
        low = mid + 1;
      } else if cmp > 0 {
        high = mid - 1;
      } else {
        return Ok(mid);
      }
    }

    Ok(-1 - low)
  }
}
/// Represents a path in TopNSearcher.
pub struct FSTPath<T>
where
  T: OutputsBound,
{
  /// Holds the last arc appended to this path
  pub arc: Arc<T>,
  /// Holds cost plus any usage-specific output:
  pub output: T,
  pub input: IntsRefBuilder<Vec<i32>>,
  pub boost: f32,
  pub context: String,
  // Custom int payload for consumers; the NRT suggester uses this to record
  // if this path has already enumerated a surface form
  pub payload: i32,
}

impl<T> FSTPath<T>
where
  T: OutputsBound,
{
  pub fn new<V>(
    output: T,
    other: &Arc<T>,
    input: IntsRefBuilder<Vec<i32>>,
    boost: f32,
    context: V,
    payload: i32,
  ) -> Self
  where
    V: Into<String>,
  {
    let mut arc = Arc::default();
    arc.copy_from(other);
    FSTPath {
      arc,
      output,
      input,
      boost,
      context: context.into(),
      payload,
    }
  }
  pub fn new_path(&self, output: T, input: IntsRefBuilder<Vec<i32>>) -> Self {
    FSTPath {
      arc: self.arc.clone(),
      output,
      input,
      boost: self.boost,
      context: self.context.clone(),
      payload: self.payload,
    }
  }
}
impl<T> Display for FSTPath<T>
where
  T: OutputsBound,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "input={} output={} context={} boost={} payload={}",
      self.input.get(),
      self.output,
      self.context,
      self.boost,
      self.payload
    )
  }
}
