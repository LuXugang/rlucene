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
use std::mem;
use std::sync::LazyLock;

use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::outputs::Outputs;

static NO_OUTPUT: i64 = 0;

pub static SINGLETON: LazyLock<PositiveIntOutputs> = LazyLock::new(|| PositiveIntOutputs {
  no_output: &NO_OUTPUT,
});
/// An FST [`Outputs`] implementation where each output is a non-negative `i64`.
pub struct PositiveIntOutputs {
  no_output: &'static i64,
}

impl Clone for PositiveIntOutputs {
  fn clone(&self) -> Self {
    Self {
      no_output: self.no_output,
    }
  }
}

impl Default for PositiveIntOutputs {
  fn default() -> Self {
    Self {
      no_output: &NO_OUTPUT,
    }
  }
}

impl PositiveIntOutputs {
  pub fn get_singleton() -> &'static PositiveIntOutputs {
    &SINGLETON
  }

  #[cfg(debug_assertions)]
  fn valid(&self, o: &i64) -> bool {
    debug_assert!(*o == *self.no_output || *o > 0, "o= {o}");
    true
  }
}

impl Outputs for PositiveIntOutputs {
  type V = i64;

  fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V {
    #[cfg(debug_assertions)]
    {
      debug_assert!(self.valid(output1));
      debug_assert!(self.valid(output2));
    }

    if *output1 == *self.no_output || *output2 == *self.no_output {
      *self.no_output
    } else {
      debug_assert!(*output1 > 0);
      debug_assert!(*output2 > 0);
      if *output1 <= *output2 {
        *output1
      } else {
        *output2
      }
    }
  }

  fn subtract<'a>(&self, output: &'a Self::V, inc: &Self::V) -> std::borrow::Cow<'a, Self::V> {
    #[cfg(debug_assertions)]
    {
      debug_assert!(self.valid(output));
      debug_assert!(self.valid(inc));
    }
    debug_assert!(*output >= *inc);

    if *inc == *self.no_output {
      std::borrow::Cow::Borrowed(output)
    } else if *output == *inc {
      std::borrow::Cow::Owned(*self.no_output)
    } else {
      std::borrow::Cow::Owned(output.wrapping_sub(*inc))
    }
  }

  fn add<'a>(&self, prefix: &'a Self::V, output: &'a Self::V) -> std::borrow::Cow<'a, Self::V> {
    #[cfg(debug_assertions)]
    {
      debug_assert!(self.valid(prefix));
      debug_assert!(self.valid(output));
    }

    if *prefix == *self.no_output {
      std::borrow::Cow::Borrowed(output)
    } else if *output == *self.no_output {
      std::borrow::Cow::Borrowed(prefix)
    } else {
      // Java long arithmetic wraps even when debug assertions are enabled.
      std::borrow::Cow::Owned(prefix.wrapping_add(*output))
    }
  }

  fn write<DO>(&self, output: &Self::V, out: &mut DO) -> Result<()>
  where
    DO: DataOutput,
  {
    #[cfg(debug_assertions)]
    debug_assert!(self.valid(output));
    out.write_vlong(*output)
  }

  fn read<DI>(&self, input: &mut DI) -> Result<std::borrow::Cow<'_, Self::V>>
  where
    DI: DataInput,
  {
    let v = input.read_vlong()?;
    if v == 0 {
      Ok(std::borrow::Cow::Borrowed(self.no_output))
    } else {
      Ok(std::borrow::Cow::Owned(v))
    }
  }

  fn skip_output<DI>(&self, input: &mut DI) -> Result<()>
  where
    DI: DataInput,
  {
    input.read_vlong()?;
    Ok(())
  }

  fn get_no_output(&self) -> &Self::V {
    self.no_output
  }

  fn output_to_string(&self, output: &Self::V) -> String {
    output.to_string()
  }

  fn ram_bytes_used(&self, output: &Self::V) -> i64 {
    mem::size_of_val(output) as i64
  }
}

impl Display for PositiveIntOutputs {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
