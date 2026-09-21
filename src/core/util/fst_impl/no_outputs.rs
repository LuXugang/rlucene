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

pub static SINGLETON: LazyLock<NoOutputs> = LazyLock::new(|| NoOutputs {
  no_output: &NO_OUTPUT,
});

/// FST outputs implementation for automata that do not store output values.
///
/// lucene.experimental
pub struct NoOutputs {
  no_output: &'static Arc<i64>,
}

impl Clone for NoOutputs {
  fn clone(&self) -> Self {
    Self {
      no_output: self.no_output,
    }
  }
}

impl Default for NoOutputs {
  fn default() -> Self {
    Self {
      no_output: &NO_OUTPUT,
    }
  }
}

impl NoOutputs {
  pub fn get_singleton() -> &'static NoOutputs {
    &SINGLETON
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
    debug_assert!(Arc::ptr_eq(output1, self.no_output));
    debug_assert!(Arc::ptr_eq(output2, self.no_output));
    self.no_output.clone()
  }

  fn subtract<'a>(&self, output: &'a Self::V, inc: &Self::V) -> std::borrow::Cow<'a, Self::V> {
    debug_assert!(Arc::ptr_eq(output, self.no_output));
    debug_assert!(Arc::ptr_eq(inc, self.no_output));
    std::borrow::Cow::Borrowed(output)
  }

  fn add<'a>(&self, prefix: &'a Self::V, output: &'a Self::V) -> std::borrow::Cow<'a, Self::V> {
    debug_assert!(Arc::ptr_eq(prefix, self.no_output), "got {prefix}");
    debug_assert!(Arc::ptr_eq(output, self.no_output));
    std::borrow::Cow::Borrowed(prefix)
  }

  fn write<DO>(&self, _output: &Self::V, _out: &mut DO) -> Result<()>
  where
    DO: DataOutput,
  {
    Ok(())
  }

  fn read<DI>(&self, _input: &mut DI) -> Result<std::borrow::Cow<'_, Self::V>>
  where
    DI: DataInput,
  {
    Ok(std::borrow::Cow::Borrowed(self.no_output))
  }

  fn get_no_output(&self) -> &Self::V {
    self.no_output
  }

  fn output_to_string(&self, _output: &Self::V) -> String {
    String::new()
  }

  fn merge(&self, first: &Self::V, second: &Self::V) -> Result<Self::V> {
    debug_assert!(Arc::ptr_eq(first, self.no_output));
    debug_assert!(Arc::ptr_eq(second, self.no_output));
    Ok(self.no_output.clone())
  }

  fn ram_bytes_used(&self, _output: &Self::V) -> i64 {
    0
  }
}
