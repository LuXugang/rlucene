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
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::thread_local;

use once_cell::sync::Lazy;

use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::outputs::Outputs;

thread_local! {
    static NO_OUTPUT: Rc<i64> = Rc::new(0);
}

pub static SINGLETON: Lazy<PositiveIntOutputs> = Lazy::new(|| PositiveIntOutputs);
/// An FST [`Outputs`] implementation where each output is a non-negative long
/// value.
#[derive(Clone, Default)]
pub struct PositiveIntOutputs;

impl PositiveIntOutputs {
    pub fn get_singleton() -> &'static PositiveIntOutputs {
        &SINGLETON
    }

    fn valid(&self, o: &Rc<i64>) -> bool {
        debug_assert!(
            NO_OUTPUT.with(|rc| Rc::ptr_eq(o, rc)) || **o > 0,
            "o= {}",
            o
        );
        true
    }
}

impl Outputs for PositiveIntOutputs {
    type Outputs = Rc<i64>;

    fn common(&self, output1: &Self::Outputs, output2: &Self::Outputs) -> Self::Outputs {
        debug_assert!(self.valid(output1));
        debug_assert!(self.valid(output2));

        if Rc::ptr_eq(output1, &self.get_no_output()) || Rc::ptr_eq(output2, &self.get_no_output())
        {
            self.get_no_output()
        } else {
            debug_assert!(**output1 > 0);
            debug_assert!(**output2 > 0);
            Rc::new(std::cmp::min(**output1, **output2))
        }
    }

    fn subtract(&self, output: &Self::Outputs, inc: &Self::Outputs) -> Self::Outputs {
        debug_assert!(self.valid(output));
        debug_assert!(self.valid(inc));
        debug_assert!(**output >= **inc);

        if Rc::ptr_eq(inc, &self.get_no_output()) {
            output.clone()
        } else if **output == **inc {
            self.get_no_output()
        } else {
            Rc::new(**output - **inc)
        }
    }

    fn add(&self, prefix: &Self::Outputs, output: &Self::Outputs) -> Self::Outputs {
        debug_assert!(self.valid(prefix));
        debug_assert!(self.valid(output));

        if Rc::ptr_eq(prefix, &self.get_no_output()) {
            output.clone()
        } else if Rc::ptr_eq(output, &self.get_no_output()) {
            prefix.clone()
        } else {
            Rc::new(**prefix + **output)
        }
    }

    fn write(&self, output: &Self::Outputs, out: &mut impl DataOutput) -> Result<()> {
        debug_assert!(self.valid(output));
        out.write_vlong(**output)
    }

    fn read(&self, input: &mut impl DataInput) -> Result<Rc<i64>> {
        let v = input.read_vlong()?;
        if v == 0 {
            Ok(self.get_no_output())
        } else {
            Ok(Rc::new(v))
        }
    }

    fn get_no_output(&self) -> Self::Outputs {
        NO_OUTPUT.with(|rc| rc.clone())
    }

    fn output_to_string(&self, output: &Self::Outputs) -> String {
        output.to_string()
    }

    fn ram_bytes_used(&self, output: &Self::Outputs) -> i64 {
        // TODO
        0
    }
}

impl Display for PositiveIntOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PositiveIntOutputs")
    }
}
