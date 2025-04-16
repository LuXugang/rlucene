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
use std::cell::RefCell;
use std::fmt;
use std::fmt::Display;
use std::hash::Hash;
use std::rc::Rc;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::{Arc, InputType, FST};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::ints_ref::IntsRef;
use crate::util::ints_ref_builder::IntsRefBuilder;

pub struct Util;
impl Util {
    /// Looks up the output for this input, or null if the input is not accepted.
    pub fn get<T, O, F>(fst: &mut FST<T, O, F>, input: &IntsRef) -> Result<Option<T>>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
    {
        let mut arc = Arc::default();
        fst.get_first_arc(&mut arc);
        let mut fst_reader = fst.get_bytes_reader()?;
        let mut output = fst.outputs.borrow().get_no_output();

        for i in 0..input.length as usize{
            let label = input.ints.borrow()[input.offset as usize + i];
            let found = fst.find_target_arc(label, &arc.clone(), &mut arc, &mut fst_reader)?;
            if found.is_none() {
                return Ok(None);
            }
            output = fst.outputs.borrow().add(&output, &arc.output());
        }

        if arc.is_final() {
            let final_output = fst.outputs.borrow().add(&output, &arc.next_final_output());
            Ok(Some(final_output))
        } else {
            Ok(None)
        }
    }
    /// Looks up the output for this input, or `None` if the input is not accepted.
    pub fn get_bytes<T, O, F>(fst: &mut FST<T, O, F>, input: &BytesRef) -> Result<Option<T>>
    where
        T: OutputsBound,
        O: Outputs<T>,
        F: FstReader,
    {
        assert_eq!(fst.metadata.as_ref().unwrap().input_type, InputType::Byte1);

        let mut fst_reader = fst.get_bytes_reader()?;
        let mut arc = Arc::<T>::default();
        fst.get_first_arc(&mut arc);
        let mut output = fst.outputs.borrow().get_no_output();

        for i in 0..input.length as usize{
            let label = (input.bytes[input.offset as usize+ i] & 0xFF) as i32;
            let found = fst.find_target_arc(label, &arc.clone(), &mut arc, &mut fst_reader)?;
            if found.is_none() {
                return Ok(None);
            }
            output = fst.outputs.borrow().add(&output, &arc.output());
        }

        if arc.is_final() {
            let final_output = fst.outputs.borrow().add(&output, &arc.next_final_output());
            Ok(Some(final_output))
        } else {
            Ok(None)
        }
    }

    pub fn binary_search<T, O, F>(
        fst: &mut FST<T, O, F>,
        arc: &Arc<T>,
        target_label: i32,
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
    pub input: IntsRefBuilder,
    pub boost: f32,
    pub context: String,
    // Custom int payload for consumers; the NRT suggester uses this to record if this path has
    // already enumerated a surface form
    pub payload: i32,
}

impl<T> FSTPath<T>
where
    T: Clone + Hash + Default,
{
    pub fn new(
        output: T,
        other: &Arc<T>,
        input: IntsRefBuilder,
        boost: f32,
        context: String,
        payload: i32,
    ) -> Self {
        let mut arc =  Arc::default();
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
    pub fn new_path(&self, output: T, input: IntsRefBuilder) -> Self {
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