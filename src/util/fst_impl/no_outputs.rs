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
use crate::util::fst_impl::outputs::Outputs;
use std::fmt::{Display, Formatter};

pub struct NoOutputs;
impl NoOutputs {
    pub fn get_singleton(&self) -> Self {
        todo!()
    }
}

impl Display for NoOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T> Outputs<T> for NoOutputs
where
    T: Clone + PartialEq + Default,
{
    fn common(&self, output1: &T, output2: &T) -> T {
        todo!()
    }

    fn subtract(&self, output: &T, inc: &T) -> T {
        todo!()
    }

    fn add(&self, prefix: &T, output: &T) -> T {
        todo!()
    }

    fn write(
        &self,
        output: &T,
        out: &mut impl DataOutput,
    ) -> crate::util::error::lucene_error::Result<()> {
        todo!()
    }

    fn read(&self, input: &mut impl DataInput) -> crate::util::error::lucene_error::Result<T> {
        todo!()
    }

    fn get_no_output(&self) -> T {
        todo!()
    }

    fn output_to_string(&self, output: &T) -> String {
        todo!()
    }

    fn ram_bytes_used(&self, output: &T) -> i64 {
        todo!()
    }
}
