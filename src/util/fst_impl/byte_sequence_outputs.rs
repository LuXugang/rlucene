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
use crate::index::BytesRef;
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::outputs::Outputs;
use crate::util::{CoreHelper, SliceCopyOps, StringHelper};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

thread_local! {
    static NO_OUTPUT: Rc<BytesRef<Rc<RefCell<Vec<u8>>>>> = Rc::new(BytesRef::default());
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

impl Outputs<Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>> for ByteSequenceOutputs {
    fn common(
        &self,
        output1: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
        output2: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
    ) -> Rc<BytesRef<Rc<RefCell<Vec<u8>>>>> {
        let mismatch_pos = CoreHelper::miss_match(
            &output1.bytes.borrow()[output1.offset..(output1.offset + output1.length)],
            &output2.bytes.borrow()[output2.offset..(output2.offset + output2.length)],
        );

        if mismatch_pos == 0 {
            // no common prefix
            NO_OUTPUT.with(|rc| rc.clone())
        } else if mismatch_pos as usize == output1.length {
            // exactly equals
            output1.clone()
        } else if mismatch_pos as usize == output2.length {
            // output1 is a prefix of output2
            output2.clone()
            // output2 is a prefix of output1
        } else {
            Rc::new(BytesRef::from_slice(
                output1.bytes.clone(),
                output1.offset,
                mismatch_pos as usize,
            ))
        }
    }

    fn subtract(
        &self,
        output: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
        inc: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
    ) -> Rc<BytesRef<Rc<RefCell<Vec<u8>>>>> {
        if Rc::ptr_eq(inc, &NO_OUTPUT.with(|rc| rc.clone())) {
            // no prefix removed
            return output.clone();
        }

        debug_assert!(StringHelper::starts_with(
            &output.bytes.borrow(),
            output.offset,
            output.length,
            &inc.bytes.borrow(),
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
            Rc::new(BytesRef::from_slice(
                output.bytes.clone(),
                output.offset + inc.length,
                output.length - inc.length,
            ))
        }
    }

    fn add(
        &self,
        prefix: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
        output: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
    ) -> Rc<BytesRef<Rc<RefCell<Vec<u8>>>>> {
        let no_output_clone = NO_OUTPUT.with(|rc| rc.clone());
        if Rc::ptr_eq(prefix, &no_output_clone) {
            return output.clone();
        }
        if Rc::ptr_eq(output, &no_output_clone) {
            return prefix.clone();
        }
        debug_assert!(prefix.length > 0);
        debug_assert!(output.length > 0);
        let mut buf = Vec::with_capacity(prefix.length + output.length);
        buf.copy_from(
            &prefix.bytes.borrow()[prefix.offset..(prefix.offset + prefix.length)],
            0,
        );
        buf.copy_from(
            &output.bytes.borrow()[output.offset..(output.offset + output.length)],
            prefix.length,
        );
        Rc::new(BytesRef::from_slice(
            Rc::new(RefCell::new(buf)),
            0,
            prefix.length + output.length,
        ))
    }

    fn write(
        &self,
        output: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>,
        out: &mut impl DataOutput,
    ) -> Result<()> {
        out.write_vint(output.length as i32)?;
        out.write_bytes_range(
            &output.bytes.borrow(),
            output.offset as i32,
            output.length as i32,
        )
    }

    fn read(&self, input: &mut impl DataInput) -> Result<Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>> {
        let len = input.read_vint()?;
        if len == 0 {
            Ok(NO_OUTPUT.with(|rc| rc.clone()))
        } else {
            let mut output = vec![0u8; len as usize];
            input.read_bytes(&mut output, 0, len)?;
            Ok(Rc::new(BytesRef::from_slice(
                Rc::new(RefCell::new(output)),
                0,
                len as usize,
            )))
        }
    }

    fn skip_output(&self, input: &mut impl DataInput) -> Result<()> {
        let len = input.read_vint()?;
        if len != 0 {
            input.skip_bytes(len as i64)?;
        }
        Ok(())
    }

    fn get_no_output(&self) -> Rc<BytesRef<Rc<RefCell<Vec<u8>>>>> {
        NO_OUTPUT.with(|rc| rc.clone())
    }

    fn output_to_string(&self, output: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>) -> String {
        output.to_string()
    }

    fn ram_bytes_used(&self, _output: &Rc<BytesRef<Rc<RefCell<Vec<u8>>>>>) -> i64 {
        // TODO
        0
    }
}
impl Display for ByteSequenceOutputs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ByteSequenceOutputs")
    }
}
