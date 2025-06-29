/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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
    type V = Rc<i64>;

    fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V {
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

    fn subtract(&self, output: &Self::V, inc: &Self::V) -> Self::V {
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

    fn add(&self, prefix: &Self::V, output: &Self::V) -> Self::V {
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

    fn write(&self, output: &Self::V, out: &mut impl DataOutput) -> Result<()> {
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

    fn get_no_output(&self) -> Self::V {
        NO_OUTPUT.with(|rc| rc.clone())
    }

    fn output_to_string(&self, output: &Self::V) -> String {
        output.to_string()
    }

    fn ram_bytes_used(&self, _output: &Self::V) -> i64 {
        // TODO
        0
    }
}

impl Display for PositiveIntOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PositiveIntOutputs")
    }
}
