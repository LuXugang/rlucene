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
use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::outputs::Outputs;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::LazyLock;

static NO_OUTPUT: LazyLock<Arc<i64>> = LazyLock::new(|| Arc::new(0));

pub static SINGLETON: LazyLock<NoOutputs> = LazyLock::new(|| NoOutputs);

/// A null FST Outputs implementation; use this if you just want to build an FSA.
///
/// lucene.experimental
#[derive(Default, Clone)]
pub struct NoOutputs;

impl NoOutputs {
  pub fn get_singleton() -> &'static NoOutputs {
    &SINGLETON
  }

  fn valid(&self, o: &Arc<i64>) -> bool {
    debug_assert!(Arc::ptr_eq(o, &NO_OUTPUT), "got {o}");
    true
  }
}

impl Display for NoOutputs {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl Outputs for NoOutputs {
  type V = Arc<i64>;

  fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V {
    debug_assert!(Arc::ptr_eq(output1, &NO_OUTPUT));
    debug_assert!(Arc::ptr_eq(output2, &NO_OUTPUT));
    NO_OUTPUT.clone()
  }

  fn subtract(&self, output: &Self::V, inc: &Self::V) -> Self::V {
    debug_assert!(Arc::ptr_eq(output, &NO_OUTPUT));
    debug_assert!(Arc::ptr_eq(inc, &NO_OUTPUT));
    NO_OUTPUT.clone()
  }

  fn add(&self, prefix: &Self::V, output: &Self::V) -> Self::V {
    debug_assert!(Arc::ptr_eq(prefix, &NO_OUTPUT), "got {prefix}");
    debug_assert!(Arc::ptr_eq(output, &NO_OUTPUT));
    NO_OUTPUT.clone()
  }

  fn write(&self, _output: &Self::V, _out: &mut impl DataOutput) -> Result<()> {
    Ok(())
  }

  fn read(&self, _input: &mut impl DataInput) -> Result<Self::V> {
    Ok(NO_OUTPUT.clone())
  }

  fn get_no_output(&self) -> Self::V {
    NO_OUTPUT.clone()
  }

  fn output_to_string(&self, _output: &Self::V) -> String {
    String::new()
  }

  fn merge(&self, first: &Self::V, second: &Self::V) -> Result<Self::V> {
    debug_assert!(Arc::ptr_eq(first, &NO_OUTPUT));
    debug_assert!(Arc::ptr_eq(second, &NO_OUTPUT));
    Ok(NO_OUTPUT.clone())
  }

  fn ram_bytes_used(&self, _output: &Self::V) -> i64 {
    0
  }
}
