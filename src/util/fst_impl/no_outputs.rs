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
    type Outputs = Rc<i64>;

    fn common(&self, _output1: &Self::Outputs, _output2: &Self::Outputs) -> Self::Outputs {
        todo!()
    }

    fn subtract(&self, _output: &Self::Outputs, _inc: &Self::Outputs) -> Self::Outputs {
        todo!()
    }

    fn add(&self, _prefix: &Self::Outputs, _output: &Self::Outputs) -> Self::Outputs {
        todo!()
    }

    fn write(&self, _output: &Self::Outputs, _out: &mut impl DataOutput) -> Result<()> {
        todo!()
    }

    fn read(&self, _input: &mut impl DataInput) -> Result<Self::Outputs> {
        todo!()
    }

    fn get_no_output(&self) -> Self::Outputs {
        todo!()
    }

    fn output_to_string(&self, _output: &Self::Outputs) -> String {
        todo!()
    }

    fn ram_bytes_used(&self, _output: &Self::Outputs) -> i64 {
        todo!()
    }
}
