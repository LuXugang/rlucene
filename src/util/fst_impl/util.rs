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
use std::hash::Hash;

use crate::index::BytesRef;
use crate::util::access::AccessVec;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::{Arc, InputType, FST};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::ints_ref::IntsRef;
use crate::util::ints_ref_builder::IntsRefBuilder;

pub struct Util;
impl Util {
    /// Looks up the output for this input, or null if the input is not
    /// accepted.
    pub fn get<T, O, F>(fst: &mut FST<T, O, F>, input: &IntsRef<Vec<i32>>) -> Result<Option<T>>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
    {
        let mut arc = Arc::default();
        fst.get_first_arc(&mut arc);
        let mut fst_reader = fst.get_bytes_reader()?;
        let mut output = fst.outputs.get_no_output();

        for i in 0..input.length as usize {
            let label = input.ints[input.offset as usize + i];
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
    pub fn get_bytes<T, O, F>(
        fst: &mut FST<T, O, F>,
        input: &BytesRef<Vec<u8>>,
    ) -> Result<Option<T>>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
    {
        assert_eq!(fst.metadata.as_ref().unwrap().input_type, InputType::Byte1);

        let mut fst_reader = fst.get_bytes_reader()?;
        let mut arc = Arc::<T>::default();
        fst.get_first_arc(&mut arc);
        let mut output = fst.outputs.get_no_output();

        for i in 0..input.length {
            let label = input.bytes[input.offset + i] as i32;
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
    pub fn get_utf32<AV: AccessVec<i32>>(s: &str, scratch: &mut IntsRefBuilder<AV>) {
        let len = s.len();
        Self::get_utf32_with_slice(s, 0, len, scratch);
    }
    /// Decodes the Unicode codepoints from the provided `char[]` and places
    /// them into the provided scratch `IntsRef`, which must not be `None`,
    /// and returns it.
    pub fn get_utf32_with_slice<AV: AccessVec<i32>>(
        s: &str,
        offset: usize,
        length: usize,
        scratch: &mut IntsRefBuilder<AV>,
    ) {
        let mut int_idx = 0;
        for c in s[offset..offset + length].chars() {
            scratch.grow(int_idx + 1);
            scratch.set_int_at(int_idx, c as i32);
            int_idx += 1;
        }
        scratch.set_length(int_idx);
    }

    pub fn get_ints_ref<AV1: AccessVec<u8>, AV2: AccessVec<i32>>(
        input: &BytesRef<AV1>,
        scratch: &mut IntsRefBuilder<AV2>,
    ) {
        scratch.grow_no_copy(input.length as i32);
        for i in 0..input.length {
            input.bytes.access(|bytes| {
                let byte = bytes[input.offset + i];
                scratch.set_int_at(i as i32, byte as i32);
            })
        }
        scratch.set_length(input.length as i32);
    }
    pub fn binary_search<T, O, F>(
        _fst: &mut FST<T, O, F>,
        _arc: &Arc<T>,
        _target_label: i32,
    ) -> Result<i32>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
    {
        Ok(0)
    }
}
/// Represents a path in TopNSearcher.
pub struct FSTPath<T>
where
    T: Clone + Hash + Default,
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
    T: Clone + Hash + Default,
{
    pub fn new(
        output: T,
        other: &Arc<T>,
        input: IntsRefBuilder<Vec<i32>>,
        boost: f32,
        context: String,
        payload: i32,
    ) -> Self {
        let mut arc = Arc::default();
        arc.copy_from(other);
        FSTPath {
            arc,
            output,
            input,
            boost,
            context,
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
    T: Clone + Hash + Default + Display,
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
