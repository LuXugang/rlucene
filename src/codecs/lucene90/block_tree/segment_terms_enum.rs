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
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::lucene90::block_tree::field_reader::FieldReader;
use crate::codecs::lucene90::block_tree::lucene90_block_tree_terms_reader::lucene90_bttr_util;
use crate::codecs::lucene90::block_tree::segment_terms_enum_frame::SegmentTermsEnumFrame;
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::index::base_terms_enum::BaseTermsEnum;
use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::{ByteArrayDataInput, DataInput, IndexInput};
use crate::util::array_util::ArrayUtil;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::Arc;
use crate::util::fst_impl::reverse_random_access_reader::ReverseRandomAccessReader;

pub struct SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    frame: Frame<I, P>,
    segment_terms: Rc<RefCell<SegmentTerms<I, P>>>,
    base: BaseTermsEnum,
}
pub struct SegmentTerms<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    // Lazy init: input stream
    pub(crate) input: Option<I>,

    pub(crate) term_exists: bool,
    pub(crate) fr: Rc<RefCell<FieldReader<I, P>>>,
    target_before_current_length: i32,
    output_accumulator: OutputAccumulator,
    valid_index_prefix: i32,
    eof: bool,
    pub(crate) term: BytesRefBuilder<Vec<u8>>,
    fst_reader: Option<ReverseRandomAccessReader<I::RandomAccessSlice>>,
    arcs: Vec<Arc<BytesRef<Rc<Vec<u8>>>>>,
}
pub struct Frame<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    stack: Vec<Rc<RefCell<SegmentTermsEnumFrame<I, P>>>>,
    static_frame: Rc<RefCell<SegmentTermsEnumFrame<I, P>>>,
    pub(crate) current_frame: Option<Rc<RefCell<SegmentTermsEnumFrame<I, P>>>>,
}
impl<I, P> SegmentTerms<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    pub(crate) fn init_index_input(&mut self) -> Result<()> {
        if self.input.is_none() {
            self.input = Some(self.fr.borrow().parent.borrow_mut().terms_in.try_clone()?);
        }
        Ok(())
    }
}
impl<I, P> SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    pub fn new(fr: Rc<RefCell<FieldReader<I, P>>>) -> Result<Self> {
        // Construct SegmentTerms first
        let fst_reader = match fr.borrow_mut().index.as_mut() {
            Some(fst) => Some(fst.get_bytes_reader()?),
            None => None,
        };

        let mut arcs = vec![Arc::default(); 1];
        {
            let fr_borrow = fr.borrow();
            if fr_borrow.index.is_some() {
                fr_borrow
                    .index
                    .as_ref()
                    .unwrap()
                    .get_first_arc(&mut arcs[0]);
                debug_assert!(arcs[0].is_final())
            }
        }

        let segment_terms = Rc::new(RefCell::new(SegmentTerms {
            input: None,
            term_exists: false,
            fr,
            target_before_current_length: 0,
            output_accumulator: OutputAccumulator::new(),
            valid_index_prefix: 0,
            eof: false,
            term: BytesRefBuilder::new(),
            fst_reader,
            arcs,
        }));

        // Create static_frame
        let static_frame = Rc::new(RefCell::new(SegmentTermsEnumFrame::new(
            segment_terms.clone(),
            -1,
        )?));

        // Build Frame
        let frame = Frame {
            stack: Vec::new(),
            static_frame: static_frame.clone(),
            current_frame: Some(static_frame),
        };

        Ok(Self {
            frame,
            segment_terms,
            base: BaseTermsEnum::default(),
        })
    }
    fn get_frame(&mut self, ord: usize) -> Result<Rc<RefCell<SegmentTermsEnumFrame<I, P>>>> {
        if ord >= self.frame.stack.len() {
            let new_len = ArrayUtil::oversize(ord + 1, 0);
            let mut next = Vec::with_capacity(new_len);
            next.extend_from_slice(&self.frame.stack);

            for i in self.frame.stack.len()..new_len {
                let frame = Rc::new(RefCell::new(SegmentTermsEnumFrame::new(
                    self.segment_terms.clone(),
                    i as i32,
                )?));
                next.push(frame);
            }

            self.frame.stack = next;
        }

        debug_assert_eq!(
            self.frame.stack[ord].borrow().ord,
            ord as i32,
            "Frame ord mismatch"
        );

        Ok(self.frame.stack[ord].clone())
    }
    fn check_arc_capacity(&mut self, ord: usize) {
        let mut segment_terms = self.segment_terms.borrow_mut();
        let arcs = &mut segment_terms.arcs;

        if ord >= arcs.len() {
            let new_len = ArrayUtil::oversize(ord + 1, 0);
            arcs.resize_with(new_len, Arc::default);
        }
    }
    pub(crate) fn push_frame_with_data(
        &mut self,
        arc: Option<Arc<BytesRef<Rc<Vec<u8>>>>>,
        frame_data: BytesRef<Rc<Vec<u8>>>,
        length: i32,
    ) -> Result<Rc<RefCell<SegmentTermsEnumFrame<I, P>>>> {
        {
            let mut segment_terms = self.segment_terms.borrow_mut();

            segment_terms.output_accumulator.reset();
            segment_terms.output_accumulator.push(frame_data);
        }

        self.push_frame_with_length(arc, length)
    }
    pub(crate) fn push_frame_with_length(
        &mut self,
        arc: Option<Arc<BytesRef<Rc<Vec<u8>>>>>,
        length: i32,
    ) -> Result<Rc<RefCell<SegmentTermsEnumFrame<I, P>>>> {
        self.segment_terms
            .borrow_mut()
            .output_accumulator
            .prepare_read();

        let code = self
            .segment_terms
            .borrow()
            .fr
            .borrow()
            .read_vlong_output(&mut self.segment_terms.borrow_mut().output_accumulator)?;

        let fp_seek = ((code as u64) >> lucene90_bttr_util::OUTPUT_FLAGS_NUM_BITS) as i64;

        let current_ord = self.frame.current_frame.as_ref().unwrap().borrow().ord;
        let f_rc = self.get_frame((current_ord + 1) as usize)?;

        {
            let mut f = f_rc.borrow_mut();
            f.has_terms = (code & lucene90_bttr_util::OUTPUT_FLAG_HAS_TERMS as i64) != 0;
            f.has_terms_orig = f.has_terms;
            f.is_floor = (code & lucene90_bttr_util::OUTPUT_FLAG_IS_FLOOR as i64) != 0;

            if f.is_floor {
                f.set_floor_data(&self.segment_terms.borrow_mut().output_accumulator)?;
            }
        }

        self.push_frame(arc, fp_seek, length)?;

        Ok(f_rc)
    }
    pub(crate) fn push_frame(
        &mut self,
        arc: Option<Arc<BytesRef<Rc<Vec<u8>>>>>,
        fp: i64,
        length: i32,
    ) -> Result<Rc<RefCell<SegmentTermsEnumFrame<I, P>>>> {
        let current_ord = self.frame.current_frame.as_ref().unwrap().borrow().ord;

        let f_rc = self.get_frame((current_ord + 1) as usize)?;

        {
            let mut f = f_rc.borrow_mut();
            f.arc = arc;

            if f.fp_orig == fp && f.next_ent != -1 {
                if f.ord > self.segment_terms.borrow().target_before_current_length {
                    f.rewind()?;
                }
                debug_assert_eq!(length, f.prefix_length);
            } else {
                f.next_ent = -1;
                f.prefix_length = length;
                f.state.get_block_term_state().term_block_ord = 0;
                f.fp_orig = fp;
                f.fp = fp;
                f.last_sub_fp = -1;
            }
        }

        self.frame.current_frame = Some(f_rc.clone());
        Ok(f_rc)
    }
    fn set_eof(&mut self) -> bool {
        self.segment_terms.borrow_mut().eof = true;
        true
    }

    fn clear_eof(&mut self) -> bool {
        self.segment_terms.borrow_mut().eof = false;
        true
    }
}

impl<I, P> BytesRefIterator for SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    type AV = Vec<u8>;
}

impl<I, P> TermsEnum for SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        <BaseTermsEnum as TermsEnum>::attributes(&self.base)
    }

    fn seek_ceil(&mut self, target: &BytesRef<Self::AV>) -> Result<SeekStatus> {
        todo!()
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn seek_exact_with_state(
        &mut self,
        term: &BytesRef<Self::AV>,
        state: &TermStateEnum,
    ) -> Result<()> {
        todo!()
    }

    fn term(&self) -> Result<Cow<BytesRef<Self::AV>>> {
        debug_assert!(!self.segment_terms.borrow().eof);
        // TODO: could we avoid copy here
        let v = self.segment_terms.borrow_mut().term.bytes_ref.clone();
        Ok(Cow::Owned(v))
    }

    fn ord(&self) -> Result<i64> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn doc_freq(&self) -> Result<i32> {
        debug_assert!(!self.segment_terms.borrow().eof);

        let current_frame_rc = self.frame.current_frame.as_ref().unwrap();
        let mut current_frame = current_frame_rc.borrow_mut();
        current_frame.decode_meta_data()?;

        Ok(current_frame.state.get_block_term_state().doc_freq)
    }

    fn total_term_freq(&self) -> Result<i64> {
        debug_assert!(!self.segment_terms.borrow().eof);

        let current_frame_rc = self.frame.current_frame.as_ref().unwrap();
        let mut current_frame = current_frame_rc.borrow_mut();
        current_frame.decode_meta_data()?;

        Ok(current_frame.state.get_block_term_state().total_term_freq)
    }

    type PostingsEnum = P::PostingsEnum;

    fn postings_with_flags(
        &mut self,
        reuse: Option<Self::PostingsEnum>,
        flags: i32,
    ) -> Result<Self::PostingsEnum> {
        debug_assert!(!self.segment_terms.borrow().eof);

        let current_frame = self.frame.current_frame.as_ref().unwrap();
        let mut frame = current_frame.borrow_mut();
        frame.decode_meta_data()?; // 解码 term metadata

        let segment_terms = self.segment_terms.borrow();
        let fr_borrow = segment_terms.fr.borrow();
        let field_info = &fr_borrow.field_info;
        let postings_reader = &mut fr_borrow.parent.borrow_mut().postings_reader;

        let v = postings_reader
            .postings(field_info, &frame.state, reuse, flags)?
            .unwrap();
        Ok(v)
    }

    type ImpactsEnum = P::ImpactsEnum;

    fn impacts(&mut self, flags: i32) -> Result<Self::ImpactsEnum> {
        debug_assert!(!self.segment_terms.borrow().eof);
        let current_frame = self.frame.current_frame.as_ref().unwrap();
        let mut frame = current_frame.borrow_mut();
        frame.decode_meta_data()?;

        let segment_terms = self.segment_terms.borrow();
        let fr_borrow = segment_terms.fr.borrow();
        let field_info = &fr_borrow.field_info;
        let postings_reader = &mut fr_borrow.parent.borrow_mut().postings_reader;

        let result = postings_reader.impacts(field_info, &frame.state, flags)?;
        Ok(result)
    }

    type TermState = BlockTermStateEnum;

    fn term_state(&self) -> Result<Self::TermState> {
        debug_assert!(!self.segment_terms.borrow().eof);

        let current_frame = self.frame.current_frame.as_ref().unwrap();
        let mut frame = current_frame.borrow_mut();
        frame.decode_meta_data()?;

        let cloned_state = frame.state.clone();
        Ok(cloned_state)
    }
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

impl Display for OutputAccumulator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "OutputAccumulator")
    }
}

impl DataInput for OutputAccumulator {
    fn read_byte(&mut self) -> Result<u8> {
        if self.index >= self.current.length {
            self.output_index += 1;
            self.current = self.outputs[self.output_index].clone();
            self.index = 0;
        }
        let byte = self.current.bytes[self.current.offset + self.index];
        self.index += 1;
        Ok(byte)
    }

    fn read_bytes(&mut self, _b: &mut [u8], _offset: i32, _len: i32) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn skip_bytes(&mut self, _num_bytes: i64) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }
}
