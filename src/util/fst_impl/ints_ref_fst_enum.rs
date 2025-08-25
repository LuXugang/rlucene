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
use std::rc::Rc;

use crate::util::OptionTakeExt;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::{FST, fst_util};
use crate::util::fst_impl::fst_enum::{FSTEnum, FSTEnumBase, InputOutput};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::outputs::Outputs;
use crate::util::ints_ref::IntsRef;

/// Enumerates all input (`IntsRef`) + output pairs in an FST.
pub struct IntsRefFSTEnum<O, F>
where
    O: Outputs,
    F: FstReader,
{
    pub(crate) current: RcIntsRef,
    pub(crate) result: InputOutput<O::V, RcIntsRef>,
    pub(crate) target: IntsRef<Rc<RefCell<Vec<i32>>>>,
    pub base: Option<FSTEnum<O, F>>,
}

impl<O, F> IntsRefFSTEnum<O, F>
where
    O: Outputs,
    F: FstReader,
{
    /// `do_floor` controls the behavior of advance: if it's true,
    /// `advance` positions to the biggest term before target.
    pub fn new(fst: FST<O, F>) -> Result<Self> {
        let mut current: IntsRef<Rc<RefCell<Vec<i32>>>> = IntsRef::with_capacity(10);
        current.offset = 1;
        let result_input =
            IntsRef::from_slice(current.ints.clone(), current.offset, current.length);
        let base = FSTEnum::new(fst)?;
        Ok(Self {
            current,
            result: InputOutput {
                input: result_input,
                output: O::V::default(),
            },
            target: IntsRef::default(),
            base: Some(base),
        })
    }

    pub fn current(&self) -> &InputOutput<O::V, RcIntsRef> {
        &self.result
    }

    pub fn next_value(&mut self) -> Result<Option<&InputOutput<O::V, RcIntsRef>>> {
        debug_assert!(self.base.is_some());
        let mut base = self.base.take().unwrap();
        base.do_next(self)?;
        self.base = Some(base);
        self.set_result()
    }
    /// Seeks to smallest term that's &gt;= target.
    pub fn seek_ceil(
        &mut self,
        target: IntsRef<Rc<RefCell<Vec<i32>>>>,
    ) -> Result<Option<&InputOutput<O::V, RcIntsRef>>> {
        self.target = target;
        debug_assert!(self.base.is_some());
        let mut base = self.base.take().unwrap();
        debug_assert!(self.target.length <= i32::MAX as usize);
        base.target_length = self.target.length as i32;
        base.do_seek_ceil(self)?;
        self.base = Some(base);

        self.set_result()
    }

    ///  Seeks to biggest term that's &lt;= target.
    pub fn seek_floor(
        &mut self,
        target: IntsRef<Rc<RefCell<Vec<i32>>>>,
    ) -> Result<Option<&InputOutput<O::V, RcIntsRef>>> {
        self.target = target;
        debug_assert!(self.base.is_some());
        let mut base = self.base.take().unwrap();
        debug_assert!(self.target.length <= i32::MAX as usize);
        base.target_length = self.target.length as i32;
        base.do_seek_floor(self)?;
        self.base = Some(base);
        self.set_result()
    }
    /// Seeks to the exact target term and returns `None` if the term does not
    /// exist. This is faster than using [`Self::seek_floor`] or
    /// [`Self::seek_ceil`] because it short-circuits as soon as a mismatch
    /// is detected.
    pub fn seek_exact(
        &mut self,
        target: IntsRef<Rc<RefCell<Vec<i32>>>>,
    ) -> Result<Option<&InputOutput<O::V, RcIntsRef>>> {
        self.target = target;
        debug_assert!(self.base.is_some());
        let mut base = self.base.take().unwrap();
        debug_assert!(self.target.length <= i32::MAX as usize);
        base.target_length = self.target.length as i32;

        if base.do_seek_exact(self)? {
            debug_assert_eq!(base.upto, 1 + self.target.length);
            self.base = Some(base);
            self.set_result()
        } else {
            self.base = Some(base);
            Ok(None)
        }
    }

    fn set_result(&mut self) -> Result<Option<&InputOutput<O::V, RcIntsRef>>> {
        self.base.take_do_return(|base| {
            if base.upto == 0 {
                Ok(None)
            } else {
                self.current.length = base.upto - 1;
                self.result.input.length = base.upto - 1;
                self.result.output = base.output[base.upto].clone();
                Ok(Some(&self.result))
            }
        })
    }
}

impl<O, F> FSTEnumBase<O, F> for IntsRefFSTEnum<O, F>
where
    O: Outputs,
    F: FstReader,
{
    fn get_target_label(&mut self, base: &mut FSTEnum<O, F>) -> Result<i32> {
        if base.upto - 1 == self.target.length {
            Ok(fst_util::END_LABEL)
        } else {
            Ok(self.target.ints.borrow()[self.target.offset + base.upto - 1])
        }
    }

    fn get_current_label(&mut self, base: &mut FSTEnum<O, F>) -> Result<i32> {
        Ok(self.current.ints.borrow()[base.upto])
    }

    fn set_current_label(&mut self, label: i32, base: &mut FSTEnum<O, F>) -> Result<()> {
        self.current.ints.borrow_mut()[base.upto] = label;
        Ok(())
    }

    fn grow(&mut self, base: &mut FSTEnum<O, F>) -> Result<()> {
        ArrayUtil::grow_with_len(&mut *self.current.ints.borrow_mut(), base.upto + 1);
        Ok(())
    }
}
pub type RcIntsRef = IntsRef<Rc<RefCell<Vec<i32>>>>;
