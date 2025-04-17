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
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::bytes_rc::BytesRc;
use crate::util::fst_impl::fst::fst_util;
use crate::util::fst_impl::fst_enum::{FSTEnumBase, InputOutput};
use crate::util::fst_impl::outputs::OutputsBound;

/// Enumerates all input (`BytesRc`) + output pairs in an FST.
pub struct BytesRefFSTEnum<T>
where
    T: OutputsBound,
{
    pub(crate) current: BytesRc,
    pub(crate) result: InputOutput<T, BytesRc>,
    pub(crate) target: BytesRc,
}

impl<T> Default for BytesRefFSTEnum<T>
where
    T: OutputsBound,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BytesRefFSTEnum<T>
where
    T: OutputsBound,
{
    /// `do_floor` controls the behavior of advance: if it's true,
    /// `advance` positions to the biggest term before target.
    pub fn new() -> Self {
        let mut current = BytesRc::with_capacity(10);
        current.offset = 1;
        let result_input = BytesRc::from_vec(current.bytes.clone(), current.offset, current.length);
        BytesRefFSTEnum {
            current,
            result: InputOutput {
                input: result_input,
                output: T::default(),
            },
            target: BytesRc::new(),
        }
    }
}

impl<T> FSTEnumBase<BytesRc, T> for BytesRefFSTEnum<T>
where
    T: OutputsBound,
{
    fn current(&self) -> &InputOutput<T, BytesRc> {
        &self.result
    }

    fn get_target_label(&self, upto: usize) -> Result<i32> {
        if upto - 1 == self.target.length as usize {
            Ok(fst_util::END_LABEL)
        } else {
            let b = self.target.bytes.borrow()[self.target.offset as usize + upto - 1];
            Ok(b as i32)
        }
    }

    fn get_current_label(&self, upto: usize) -> Result<i32> {
        let b = self.current.bytes.borrow()[upto];
        Ok(b as i32)
    }

    fn set_current_label(&mut self, label: i32, upto: usize) -> Result<()> {
        self.current.bytes.borrow_mut()[upto] = label as u8;
        Ok(())
    }

    fn grow(&mut self, upto: usize) -> Result<()> {
        ArrayUtil::grow_with_len(&mut self.current.bytes.borrow_mut(), upto as i32 + 1)
    }

    fn set_results(&mut self, upto: usize, output: T) -> Result<Option<&InputOutput<T, BytesRc>>> {
        self.current.length = upto as i32 - 1;
        self.result.output = output;
        Ok(Some(&self.result))
    }

    fn set_target(&mut self, target: BytesRc) -> Result<i32> {
        self.target = target;
        Ok(self.target.length)
    }
}
