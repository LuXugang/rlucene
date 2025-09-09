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

use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::outputs::Outputs;

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
