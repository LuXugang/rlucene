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
use once_cell::sync::Lazy;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crate::core::index::BytesRef;
use crate::core::store::{DataInput, DataOutput};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fst_impl::outputs::Outputs;
use crate::core::util::{CoreHelper, SliceCopyOps, StringHelper, TryIntoInt};

thread_local! {
    static NO_OUTPUT:BytesRef<Arc<Vec<u8>>> = BytesRef::default();
}
pub static SINGLETON: Lazy<ByteSequenceOutputs> = Lazy::new(|| ByteSequenceOutputs);
/// An FST Outputs implementation where each output is a sequence of bytes.
///
/// lucene.experimental
pub struct ByteSequenceOutputs;
impl ByteSequenceOutputs {
    pub fn get_singleton() -> &'static ByteSequenceOutputs {
        &SINGLETON
    }
}

impl Clone for ByteSequenceOutputs {
    fn clone(&self) -> Self {
        ByteSequenceOutputs
    }
}

impl Outputs for ByteSequenceOutputs {
    type V = BytesRef<Arc<Vec<u8>>>;

    fn common(&self, output1: &Self::V, output2: &Self::V) -> Self::V {
        let a = &output1.bytes[output1.offset..output1.offset + output1.length];
        let b = &output2.bytes[output2.offset..output2.offset + output2.length];

        let mismatch_pos = CoreHelper::miss_match(a, b);

        match mismatch_pos {
            -1 => output1.clone(),
            0 => NO_OUTPUT.with(|rc| rc.clone()),
            n if n as usize == output1.length => output1.clone(),
            n if n as usize == output2.length => output2.clone(),
            n => BytesRef::from_slice(output1.bytes.clone(), output1.offset, n as usize),
        }
    }

    fn subtract(&self, output: &Self::V, inc: &Self::V) -> Self::V {
        if NO_OUTPUT.with(|rc| BytesRef::equals(inc, rc)) {
            // no prefix removed
            return output.clone();
        }

        debug_assert!(StringHelper::starts_with(
            &output.bytes,
            output.offset,
            output.length,
            &inc.bytes,
            inc.offset,
            inc.length
        ));
        if inc.length == output.length {
            NO_OUTPUT.with(|rc| rc.clone())
        } else {
            debug_assert!(
                inc.length < output.length,
                "inc.length={} vs output.length={}",
                inc.length,
                output.length
            );
            debug_assert!(inc.length > 0);
            BytesRef::from_slice(
                output.bytes.clone(),
                output.offset + inc.length,
                output.length - inc.length,
            )
        }
    }

    fn add(&self, prefix: &Self::V, output: &Self::V) -> Self::V {
        let no_output = NO_OUTPUT.with(|rc| rc.clone());
        if BytesRef::equals(prefix, &no_output) {
            return output.clone();
        }
        if BytesRef::equals(output, &no_output) {
            return prefix.clone();
        }
        debug_assert!(prefix.length > 0);
        debug_assert!(output.length > 0);
        let mut buf = vec![0u8; prefix.length + output.length];
        buf.copy_from(
            &prefix.bytes[prefix.offset..(prefix.offset + prefix.length)],
            0,
        );
        buf.copy_from(
            &output.bytes[output.offset..(output.offset + output.length)],
            prefix.length,
        );
        BytesRef::from_slice(Arc::new(buf), 0, prefix.length + output.length)
    }

    fn write(&self, output: &Self::V, out: &mut impl DataOutput) -> Result<()> {
        out.write_vint(output.length as i32)?;
        out.write_bytes_range(&output.bytes, output.offset, output.length)
    }

    fn read(&self, input: &mut impl DataInput) -> Result<Self::V> {
        let len = input.read_vint()?.try_convert()?;
        if len == 0 {
            Ok(NO_OUTPUT.with(|rc| rc.clone()))
        } else {
            let mut output = vec![0u8; len];
            input.read_bytes(&mut output, 0, len)?;
            Ok(BytesRef::from_slice(Arc::new(output), 0, len))
        }
    }

    fn skip_output(&self, input: &mut impl DataInput) -> Result<()> {
        let len = input.read_vint()?;
        if len != 0 {
            input.skip_bytes(len as i64)?;
        }
        Ok(())
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
impl Display for ByteSequenceOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", std::any::type_name::<Self>())
    }
}
