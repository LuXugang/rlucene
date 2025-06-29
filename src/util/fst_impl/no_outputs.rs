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

use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::outputs::Outputs;

pub struct NoOutputs;
impl NoOutputs {
    pub fn get_singleton(&self) -> Self {
        todo!()
    }
}

impl Display for NoOutputs {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Clone for NoOutputs {
    fn clone(&self) -> Self {
        NoOutputs
    }
}

impl Outputs for NoOutputs {
    // TODO: 未完成
    type V = Rc<i64>;

    fn common(&self, _output1: &Self::V, _output2: &Self::V) -> Self::V {
        todo!()
    }

    fn subtract(&self, _output: &Self::V, _inc: &Self::V) -> Self::V {
        todo!()
    }

    fn add(&self, _prefix: &Self::V, _output: &Self::V) -> Self::V {
        todo!()
    }

    fn write(&self, _output: &Self::V, _out: &mut impl DataOutput) -> Result<()> {
        todo!()
    }

    fn read(&self, _input: &mut impl DataInput) -> Result<Self::V> {
        todo!()
    }

    fn get_no_output(&self) -> Self::V {
        todo!()
    }

    fn output_to_string(&self, _output: &Self::V) -> String {
        todo!()
    }

    fn ram_bytes_used(&self, _output: &Self::V) -> i64 {
        todo!()
    }
}
