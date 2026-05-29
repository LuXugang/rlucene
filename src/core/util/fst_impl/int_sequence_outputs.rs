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
use std::sync::Arc;
use std::sync::LazyLock;

use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::outputs::Outputs;
use crate::core::util::ints_ref::IntsRef;
use crate::core::util::{CoreHelper, SliceCopyOps};

/// Global NO_OUTPUT singleton shared by all threads, matching Java's
/// `private static final IntsRef NO_OUTPUT = new IntsRef()` semantics.
static NO_OUTPUT: LazyLock<IntsRef<Arc<Vec<i32>>>> = LazyLock::new(IntsRef::new);

pub static SINGLETON: LazyLock<IntSequenceOutputs> = LazyLock::new(|| IntSequenceOutputs);

/// An FST `Outputs` implementation where each output is a sequence of ints.
#[derive(Clone, Default)]
pub struct IntSequenceOutputs;

impl IntSequenceOutputs {
  pub fn get_singleton() -> &'static IntSequenceOutputs {
    &SINGLETON
  }

  fn valid(&self, o: &IntsRef<Rc<Vec<i32>>>) -> bool {
    o.offset + o.length <= o.ints.len()
  }
}

impl Outputs for IntSequenceOutputs {
  type V = IntsRef<Arc<Vec<i32>>>;

  fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V {
    let a = &output1.ints[output1.offset..output1.offset + output1.length];
    let b = &output2.ints[output2.offset..output2.offset + output2.length];

    let mismatch = CoreHelper::miss_match(a, b);

    match mismatch {
      -1 => output1.clone(),     // exactly equals
      0 => self.get_no_output(), // no common prefix
      n if n as usize == output1.length => output1.clone(),
      n if n as usize == output2.length => output2.clone(),
      n => IntsRef::from_slice(Arc::new(a[..n as usize].to_vec()), 0, n as usize),
    }
  }

  fn subtract(&self, output: &Self::V, inc: &Self::V) -> Self::V {
    let no_output_clone = NO_OUTPUT.clone();

    if IntsRef::equals(inc, &no_output_clone) {
      return output.clone();
    } else if inc.length == output.length {
      return self.get_no_output();
    }

    debug_assert!(inc.length < output.length);

    IntsRef::from_slice(
      output.ints.clone(),
      output.offset + inc.length,
      output.length - inc.length,
    )
  }

  fn add(&self, prefix: &Self::V, output: &Self::V) -> Self::V {
    let no_output = NO_OUTPUT.clone();

    if IntsRef::equals(prefix, &no_output) {
      return output.clone();
    } else if IntsRef::equals(output, &no_output) {
      return prefix.clone();
    }
    debug_assert!(prefix.length > 0);
    debug_assert!(output.length > 0);
    let mut buf = vec![0; prefix.length + output.length];
    buf.copy_from(
      &prefix.ints[prefix.offset..prefix.offset + prefix.length],
      0,
    );
    buf.copy_from(
      &output.ints[output.offset..output.offset + output.length],
      prefix.length,
    );

    IntsRef::from_slice(Arc::new(buf), 0, prefix.length + output.length)
  }

  fn write(&self, output: &Self::V, out: &mut impl DataOutput) -> Result<()> {
    out.write_vint(output.length as i32)?;
    for i in 0..output.length {
      out.write_vint(output.ints[output.offset + i])?;
    }
    Ok(())
  }

  fn read(&self, input: &mut impl DataInput) -> Result<Self::V> {
    let len = input.read_vint()?;
    if len == 0 {
      Ok(self.get_no_output())
    } else {
      let mut buf = vec![0; len as usize];
      for item in buf.iter_mut().take(len as usize) {
        *item = input.read_vint()?;
      }
      Ok(IntsRef::from_slice(Arc::new(buf), 0, len as usize))
    }
  }

  fn skip_output(&self, input: &mut impl DataInput) -> Result<()> {
    let len = input.read_vint()?;
    if len == 0 {
      return Ok(());
    }
    for _ in 0..len {
      input.read_vint()?;
    }
    Ok(())
  }

  fn get_no_output(&self) -> Self::V {
    NO_OUTPUT.clone()
  }

  fn output_to_string(&self, output: &Self::V) -> String {
    output.to_string()
  }

  fn ram_bytes_used(&self, _output: &Self::V) -> i64 {
    // TODO: memory calculation not implement
    0
  }
}

impl Display for IntSequenceOutputs {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}
