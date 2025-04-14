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
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::outputs::Outputs;
use std::fmt::{Display, Formatter};

pub struct DummyOutputs;

impl Display for DummyOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DummyOutputs")
    }
}

impl Outputs<i32> for DummyOutputs {
    fn common(&self, _output1: &i32, _output2: &i32) -> i32 {
        debug_assert!(false, "this method should not be called");
        0
    }

    fn subtract(&self, _output: &i32, _inc: &i32) -> i32 {
        debug_assert!(false, "this method should not be called");
        0
    }

    fn add(&self, _prefix: &i32, _output: &i32) -> i32 {
        debug_assert!(false, "this method should not be called");
        0
    }

    fn write(&self, _output: &i32, _out: &mut impl DataOutput) -> Result<()> {
        Err(LuceneError::unreachable("this method should not be called"))
    }

    fn read(&self, _input: &mut impl DataInput) -> Result<i32> {
        Err(LuceneError::unreachable("this method should not be called"))
    }

    fn get_no_output(&self) -> i32 {
        debug_assert!(false, "this method should not be called");
        0
    }

    fn output_to_string(&self, _output: &i32) -> String {
        debug_assert!(false, "this method should not be called");
        "".to_string()
    }

    fn ram_bytes_used(&self, _output: &i32) -> i64 {
        debug_assert!(false, "this method should not be called");
        0
    }
}
