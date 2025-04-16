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
use crate::util::fst_impl::bytes_rc::BytesRc;
use crate::util::fst_impl::outputs::Outputs;
use crate::util::{CommonUtil, SliceCopyOps, StringHelper};
use once_cell::sync::Lazy;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

thread_local! {
    static NO_OUTPUT: Rc<BytesRc> = Rc::new(BytesRc::default());
}
pub static SINGLETON: Lazy<ByteSequenceOutputs> = Lazy::new(|| ByteSequenceOutputs);
/// An FST Outputs implementation where each output is a sequence of bytes.
///
/// lucene.experimental
pub struct ByteSequenceOutputs;
impl ByteSequenceOutputs {
    pub fn get() -> &'static ByteSequenceOutputs {
        &SINGLETON
    }
}

impl Clone for ByteSequenceOutputs {
    fn clone(&self) -> Self {
        ByteSequenceOutputs
    }
}

impl Outputs<Rc<BytesRc>> for ByteSequenceOutputs {
    fn common(&self, output1: &Rc<BytesRc>, output2: &Rc<BytesRc>) -> Rc<BytesRc> {
        let mismatch_pos = CommonUtil::miss_match(
            &output1.bytes[output1.offset as usize..(output1.offset + output1.length) as usize],
            &output2.bytes[output2.offset as usize..(output2.offset + output2.length) as usize],
        );

        if mismatch_pos == 0 {
            // no common prefix
            NO_OUTPUT.with(|rc| rc.clone())
        } else if mismatch_pos == output1.length {
            // exactly equals
            output1.clone()
        } else if mismatch_pos == output2.length {
            // output1 is a prefix of output2
            output2.clone()
            // output2 is a prefix of output1
        } else {
            Rc::new(BytesRc::from_vec(
                output1.bytes.clone(),
                output1.offset,
                mismatch_pos,
            ))
        }
    }

    fn subtract(&self, output: &Rc<BytesRc>, inc: &Rc<BytesRc>) -> Rc<BytesRc> {
        if Rc::ptr_eq(inc, &NO_OUTPUT.with(|rc| rc.clone())) {
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
            Rc::new(BytesRc::from_vec(
                output.bytes.clone(),
                output.offset + inc.length,
                output.length - inc.length,
            ))
        }
    }

    fn add(&self, prefix: &Rc<BytesRc>, output: &Rc<BytesRc>) -> Rc<BytesRc> {
        let no_output_clone = NO_OUTPUT.with(|rc| rc.clone());
        if Rc::ptr_eq(prefix, &no_output_clone) {
            return output.clone();
        }
        if Rc::ptr_eq(output, &no_output_clone) {
            return prefix.clone();
        }
        debug_assert!(prefix.length > 0);
        debug_assert!(output.length > 0);
        let mut buf = Vec::with_capacity((prefix.length + output.length) as usize);
        buf.copy_from(
            &prefix.bytes[prefix.offset as usize..(prefix.offset + prefix.length) as usize],
            0,
        );
        buf.copy_from(
            &output.bytes[output.offset as usize..(output.offset + output.length) as usize],
            prefix.length as usize,
        );
        Rc::new(BytesRc::from_vec(
            Rc::new(buf),
            0,
            prefix.length + output.length,
        ))
    }

    fn write(&self, output: &Rc<BytesRc>, out: &mut impl DataOutput) -> Result<()> {
        out.write_vint(output.length)?;
        out.write_bytes_range(&output.bytes, output.offset, output.length)
    }

    fn read(&self, input: &mut impl DataInput) -> Result<Rc<BytesRc>> {
        let len = input.read_vint()?;
        if len == 0 {
            Ok(NO_OUTPUT.with(|rc| rc.clone()))
        } else {
            let mut output = vec![0u8; len as usize];
            input.read_bytes(&mut output, 0, len)?;
            Ok(Rc::new(BytesRc::from_vec(Rc::new(output), 0, len)))
        }
    }

    fn skip_output(&self, input: &mut impl DataInput) -> Result<()> {
        let len = input.read_vint()?;
        if len != 0 {
            input.skip_bytes(len as i64)?;
        }
        Ok(())
    }

    fn get_no_output(&self) -> Rc<BytesRc> {
        NO_OUTPUT.with(|rc| rc.clone())
    }

    fn output_to_string(&self, output: &Rc<BytesRc>) -> String {
        output.to_string()
    }

    fn ram_bytes_used(&self, _output: &Rc<BytesRc>) -> i64 {
        // TODO
        0
    }
}
impl Display for ByteSequenceOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ByteSequenceOutputs")
    }
}
