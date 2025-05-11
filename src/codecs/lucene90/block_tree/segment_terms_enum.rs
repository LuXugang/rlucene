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
use std::cell::RefCell;
use std::rc::Rc;

use crate::codecs::lucene90::block_tree::field_reader::FieldReader;
use crate::codecs::lucene90::block_tree::lucene90_block_tree_terms_reader::lucene90_bttr_util;
use crate::codecs::lucene90::block_tree::segment_terms_enum_frame::SegmentTermsEnumFrame;
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::{ByteArrayDataInput, IndexInput};
use crate::util::fst_impl::fst::Arc;
use crate::util::fst_impl::fst_reader::FstReader;

pub struct SegmentTermsEnum<I, P, R, F>
where
    I: IndexInput,
    P: PostingsReaderBase<I>,
    F: FstReader,
{
    // Lazy init: input stream
    pub(crate) input: Option<Rc<RefCell<I>>>,
    pub(crate) stack: Vec<SegmentTermsEnumFrame<I, P, R, F>>,
    pub(crate) static_frame: SegmentTermsEnumFrame<I, P, R, F>,
    pub(crate) current_frame: Option<SegmentTermsEnumFrame<I, P, R, F>>,
    pub(crate) term_exists: bool,
    pub(crate) fr: Rc<FieldReader<I, P>>,
    pub(crate) target_before_current_length: i32,
    pub(crate) output_accumulator: OutputAccumulator,
    pub(crate) valid_index_prefix: i32,
    pub(crate) eof: bool,
    pub(crate) term: BytesRefBuilder<Vec<u8>>,
    pub(crate) fst_reader: Option<F::FstBytesReader>,
    pub(crate) arcs: Vec<Arc<BytesRef<Rc<Vec<u8>>>>>,
}

pub struct OutputAccumulator {
    pub(crate) outputs: Vec<BytesRef<Rc<Vec<u8>>>>,
    pub(crate) current: BytesRef<Rc<Vec<u8>>>,
    pub(crate) num: usize,
    pub(crate) output_index: usize,
    pub(crate) index: usize,
}
impl OutputAccumulator {
    pub(crate) fn new() -> Self {
        Self {
            outputs: Vec::with_capacity(16),
            current: BytesRef::new(),
            num: 0,
            output_index: 0,
            index: 0,
        }
    }
    pub(crate) fn push(&mut self, output: BytesRef<Rc<Vec<u8>>>) {
        if !lucene90_bttr_util::NO_OUTPUT.with(|rc| BytesRef::equals(&output, rc)) {
            debug_assert!(output.length > 0);
            if self.outputs.len() == self.num {
                self.outputs.resize(self.num + 1, BytesRef::new());
            }
            self.outputs[self.num] = output;
            self.num += 1;
        }
    }

    pub(crate) fn pop(&mut self, output: &BytesRef<Rc<Vec<u8>>>) {
        if !lucene90_bttr_util::NO_OUTPUT.with(|rc| BytesRef::equals(output, rc)) {
            debug_assert!(self.num > 0);
            debug_assert!(&self.outputs[self.num - 1] == output);
            self.num -= 1;
        }
    }
    pub(crate) fn pop_n(&mut self, cnt: usize) {
        debug_assert!(self.num >= cnt);
        self.num -= cnt;
    }

    pub(crate) fn output_count(&self) -> usize {
        self.num
    }

    pub(crate) fn reset(&mut self) {
        self.num = 0;
    }

    pub(crate) fn prepare_read(&mut self) {
        self.index = 0;
        self.output_index = 0;
        self.current = self.outputs[0].clone();
    }
    /// Set the last arc as the source of the floorData.  
    /// This won't change the reading position of this [`OutputAccumulator`].
    pub(crate) fn set_floor_data(&self, floor_data: &mut ByteArrayDataInput<Rc<Vec<u8>>>) {
        debug_assert!(
            self.output_index == self.num - 1,
            "floor data should be stored in last arc, got output_index={}, num={}",
            self.output_index,
            self.num
        );

        let output = self.outputs[self.output_index].clone();
        let start = output.offset + self.index;
        let length = output.length - self.index;

        floor_data.reset_with_range(output.bytes, start, length);
    }
}
