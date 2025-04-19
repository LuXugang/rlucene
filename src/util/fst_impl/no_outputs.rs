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
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::outputs::{Outputs, OutputsBound};
use std::fmt::{Display, Formatter};

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

impl<T> Outputs<T> for NoOutputs
where
    T: OutputsBound,
{
    fn common(&self, _output1: &T, _output2: &T) -> T {
        todo!()
    }

    fn subtract(&self, _output: &T, _inc: &T) -> T {
        todo!()
    }

    fn add(&self, _prefix: &T, _output: &T) -> T {
        todo!()
    }

    fn write(&self, _output: &T, _out: &mut impl DataOutput) -> Result<()> {
        todo!()
    }

    fn read(&self, _input: &mut impl DataInput) -> Result<T> {
        todo!()
    }

    fn get_no_output(&self) -> T {
        todo!()
    }

    fn output_to_string(&self, _output: &T) -> String {
        todo!()
    }

    fn ram_bytes_used(&self, _output: &T) -> i64 {
        todo!()
    }
}
