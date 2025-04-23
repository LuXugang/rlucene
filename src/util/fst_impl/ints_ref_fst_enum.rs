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

use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::{fst_util, FST};
use crate::util::fst_impl::fst_enum::{FSTEnum, FSTEnumBase, InputOutput};
use crate::util::fst_impl::fst_reader::FstReader;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use crate::util::ints_ref::IntsRef;
use crate::util::OptionTakeExt;

/// Enumerates all input (`IntsRef`) + output pairs in an FST.
pub struct IntsRefFSTEnum<T, O, F>
where
    T: OutputsBound,
    O: Outputs<T>,
    F: FstReader,
{
    pub(crate) current: RcIntsRef,
    pub(crate) result: InputOutput<T, RcIntsRef>,
    pub(crate) target: IntsRef<Vec<i32>>,
    base: Option<FSTEnum<T, O, F>>,
}
#[allow(unused)]
impl<T, O, F> IntsRefFSTEnum<T, O, F>
where
    T: OutputsBound,
    O: Outputs<T>,
    F: FstReader,
{
    /// `do_floor` controls the behavior of advance: if it's true,
    /// `advance` positions to the biggest term before target.
    pub fn new(fst: FST<T, O, F>) -> Result<Self> {
        let mut current = IntsRef::with_capacity(10)?;
        current.offset = 1;
        let current = Rc::new(RefCell::new(current));
        let base = FSTEnum::new(fst)?;
        Ok(Self {
            current: current.clone(),
            result: InputOutput {
                input: current,
                output: T::default(),
            },
            target: IntsRef::default(),
            base: Some(base),
        })
    }

    pub fn current(&self) -> &InputOutput<T, RcIntsRef> {
        &self.result
    }

    pub fn next(&mut self) -> Result<Option<&InputOutput<T, RcIntsRef>>> {
        self.base.take_do_return(|base| base.do_next())?;
        self.set_result()
    }
    /// Seeks to smallest term that's &gt;= target.
    pub fn seek_ceil(
        &mut self,
        target: IntsRef<Vec<i32>>,
    ) -> Result<Option<&InputOutput<T, RcIntsRef>>> {
        self.target = target;
        match self.base.take() {
            Some(mut base) => {
                base.target_length = self.target.length;
                base.do_seek_ceil(self)?;
                self.base = Some(base);
            },
            None => {
                return Err(LuceneError::illegal_state("base is None".to_string()));
            },
        }
        self.set_result()
    }

    ///  Seeks to biggest term that's &lt;= target.
    pub fn seek_floor(
        &mut self,
        target: IntsRef<Vec<i32>>,
    ) -> Result<Option<&InputOutput<T, RcIntsRef>>> {
        self.target = target;
        match self.base.take() {
            Some(mut base) => {
                base.target_length = self.target.length;
                base.do_seek_floor(self)?;
                self.base = Some(base);
            },
            None => {
                return Err(LuceneError::illegal_state("base is None".to_string()));
            },
        }
        self.set_result()
    }
    /// Seeks to the exact target term and returns `None` if the term does not
    /// exist. This is faster than using [`Self::seek_floor`] or
    /// [`Self::seek_ceil`] because it short-circuits as soon as a mismatch
    /// is detected.
    pub fn seek_exact(
        &mut self,
        target: IntsRef<Vec<i32>>,
    ) -> Result<Option<&InputOutput<T, RcIntsRef>>> {
        self.target = target;
        match self.base.take() {
            Some(mut base) => {
                base.target_length = self.target.length;
                if base.do_seek_exact(self)? {
                    debug_assert_eq!(base.upto, 1 + self.target.length as usize);
                    self.base = Some(base);
                    self.set_result()
                } else {
                    self.base = Some(base);
                    Ok(None)
                }
            },
            None => Err(LuceneError::illegal_state("base is None".to_string())),
        }
    }

    fn set_result(&mut self) -> Result<Option<&InputOutput<T, RcIntsRef>>> {
        self.base.take_do_return(|base| {
            if base.upto == 0 {
                Ok(None)
            } else {
                self.current.borrow_mut().length = base.upto as i32 - 1;
                self.result.output = base.output[base.upto].clone();
                Ok(Some(&self.result))
            }
        })
    }
}

impl<T, O, F> FSTEnumBase for IntsRefFSTEnum<T, O, F>
where
    T: OutputsBound,
    O: Outputs<T>,
    F: FstReader,
{
    fn get_target_label(&mut self) -> Result<i32> {
        self.base.take_do_return(|base| {
            if base.upto - 1 == self.target.length as usize {
                Ok(fst_util::END_LABEL)
            } else {
                Ok(self.target.ints[self.target.offset as usize + base.upto - 1])
            }
        })
    }

    fn get_current_label(&mut self) -> Result<i32> {
        self.base
            .take_do_return(|base| Ok(self.current.borrow().ints[base.upto]))
    }

    fn set_current_label(&mut self, label: i32) -> Result<()> {
        self.base.take_do_return(|base| {
            self.current.borrow_mut().ints[base.upto] = label;
            Ok(())
        })
    }

    fn grow(&mut self) -> Result<()> {
        self.base.take_do_return(|base| {
            ArrayUtil::grow_with_len(&mut self.current.borrow_mut().ints, base.upto + 1);
            Ok(())
        })
    }
}
pub type RcIntsRef = Rc<RefCell<IntsRef<Vec<i32>>>>;
