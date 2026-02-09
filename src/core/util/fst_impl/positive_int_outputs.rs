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
use std::sync::Arc;
use std::thread_local;

use once_cell::sync::Lazy;

use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::outputs::Outputs;

thread_local! {
    static NO_OUTPUT: Arc<i64> = Arc::new(0);
}

pub static SINGLETON: Lazy<PositiveIntOutputs> = Lazy::new(|| PositiveIntOutputs);
/// An FST `Outputs` implementation where each output is a non-negative long
/// value.
#[derive(Clone, Default)]
pub struct PositiveIntOutputs;

impl PositiveIntOutputs {
    pub fn get_singleton() -> &'static PositiveIntOutputs {
        &SINGLETON
    }

    fn valid(&self, o: &Arc<i64>) -> bool {
        debug_assert!(NO_OUTPUT.with(|rc| Arc::ptr_eq(o, rc)) || **o > 0, "o= {o}");
        true
    }
}

impl Outputs for PositiveIntOutputs {
    type V = Arc<i64>;

    fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V {
        debug_assert!(self.valid(output1));
        debug_assert!(self.valid(output2));

        if Arc::ptr_eq(output1, &self.get_no_output())
            || Arc::ptr_eq(output2, &self.get_no_output())
        {
            self.get_no_output()
        } else {
            debug_assert!(**output1 > 0);
            debug_assert!(**output2 > 0);
            Arc::new(std::cmp::min(**output1, **output2))
        }
    }

    fn subtract(&self, output: &Self::V, inc: &Self::V) -> Self::V {
        debug_assert!(self.valid(output));
        debug_assert!(self.valid(inc));
        debug_assert!(**output >= **inc);

        if Arc::ptr_eq(inc, &self.get_no_output()) {
            output.clone()
        } else if **output == **inc {
            self.get_no_output()
        } else {
            Arc::new(**output - **inc)
        }
    }

    fn add(&self, prefix: &Self::V, output: &Self::V) -> Self::V {
        debug_assert!(self.valid(prefix));
        debug_assert!(self.valid(output));

        if Arc::ptr_eq(prefix, &self.get_no_output()) {
            output.clone()
        } else if Arc::ptr_eq(output, &self.get_no_output()) {
            prefix.clone()
        } else {
            Arc::new(**prefix + **output)
        }
    }

    fn write(&self, output: &Self::V, out: &mut impl DataOutput) -> Result<()> {
        debug_assert!(self.valid(output));
        out.write_vlong(**output)
    }

    fn read(&self, input: &mut impl DataInput) -> Result<Arc<i64>> {
        let v = input.read_vlong()?;
        if v == 0 {
            Ok(self.get_no_output())
        } else {
            Ok(Arc::new(v))
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
        write!(f, "{}", std::any::type_name::<Self>())
    }
}
