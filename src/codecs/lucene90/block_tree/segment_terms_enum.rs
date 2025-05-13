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
use crate::index::term_state::{TermState, TermStateEnum};
use crate::index::terms::Terms;
use crate::index::terms_enum::{SeekAction, SeekStatus, TermsEnum};
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::{ByteArrayDataInput, DataInput, IndexInput};
use crate::util::array_util::ArrayUtil;
use crate::util::attribute_source::AttributeSource;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::Arc;
use crate::util::fst_impl::reverse_random_access_reader::ReverseRandomAccessReader;
use crate::util::ToInt;

pub struct SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    frame: Frame<I, P>,
    segment_terms: Rc<RefCell<SegmentTerms<I, P>>>,
    arcs: Vec<Rc<RefCell<Arc<BytesRef<Rc<Vec<u8>>>>>>>,
    base: BaseTermsEnum,
    fst_reader: Option<ReverseRandomAccessReader<I::RandomAccessSlice>>,
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

        let v = Rc::new(RefCell::new(Arc::default()));
        let arcs = vec![v; 1];
        {
            let fr_borrow = fr.borrow();
            if fr_borrow.index.is_some() {
                fr_borrow
                    .index
                    .as_ref()
                    .unwrap()
                    .get_first_arc(&mut *arcs[0].borrow_mut());
                debug_assert!(arcs[0].borrow().is_final())
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
            arcs,
            fst_reader,
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
    pub(crate) fn get_arc(&mut self, ord: usize) -> Rc<RefCell<Arc<BytesRef<Rc<Vec<u8>>>>>> {
        if ord >= self.arcs.len() {
            let new_len = ArrayUtil::oversize(ord + 1, 0);
            for _ in self.arcs.len()..new_len {
                self.arcs.push(Rc::new(RefCell::new(Arc::default())))
            }
        }

        self.arcs[ord].clone()
    }
    pub(crate) fn push_frame_with_data(
        &mut self,
        arc: Option<Rc<RefCell<Arc<BytesRef<Rc<Vec<u8>>>>>>>,
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
        arc: Option<Rc<RefCell<Arc<BytesRef<Rc<Vec<u8>>>>>>>,
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
        arc: Option<Rc<RefCell<Arc<BytesRef<Rc<Vec<u8>>>>>>>,
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

    fn clear_eof(&mut self) -> bool {
        self.segment_terms.borrow_mut().eof = false;
        true
    }
    fn set_eof(&mut self) -> bool {
        self.segment_terms.borrow_mut().eof = true;
        true
    }
    pub fn prepare_seek_exact(
        &mut self,
        target: &BytesRef<Vec<u8>>,
        prefetch: bool,
    ) -> Result<Option<SeekAction<I, P>>> {
        {
            let segment_terms = self.segment_terms.borrow();
            let fr = segment_terms.fr.borrow();
            if fr.index.is_none() {
                return Err(LuceneError::illegal_state("terms index was not loaded"));
            }
            if fr.size()? > 0 {
                let mut iter = segment_terms.fr.borrow().iterator();
                let left = target
                    .cmp(
                        segment_terms
                            .fr
                            .borrow()
                            .get_min(&mut iter)?
                            .as_ref()
                            .unwrap(),
                    )
                    .to_int();
                let right = target
                    .cmp(
                        segment_terms
                            .fr
                            .borrow()
                            .get_max(&mut iter)?
                            .as_ref()
                            .unwrap(),
                    )
                    .to_int();
                if left < 0 || right > 0 {
                    return Ok(None);
                }
            }
        }
        self.segment_terms.borrow_mut().term.grow(1 + target.length);
        debug_assert!(self.clear_eof());
        let mut arc;
        let mut target_upto;
        {
            let mut segment_terms = self.segment_terms.borrow_mut();
            segment_terms.target_before_current_length =
                self.frame.current_frame.as_ref().unwrap().borrow().ord;
            segment_terms.output_accumulator.reset();
        }
        if !Rc::ptr_eq(
            self.frame.current_frame.as_ref().unwrap(),
            &self.frame.static_frame,
        ) {
            let mut segment_terms = self.segment_terms.borrow_mut();
            arc = self.arcs[0].clone();
            debug_assert!(arc.borrow().is_final());
            segment_terms.output_accumulator.push(arc.borrow().output());

            target_upto = 0;
            let mut last_frame = self.frame.stack[0].clone();
            debug_assert!(segment_terms.valid_index_prefix <= segment_terms.term.length() as i32);

            let target_limit =
                std::cmp::min(target.length, segment_terms.valid_index_prefix as usize);

            let mut cmp = 0;

            while target_upto < target_limit {
                let term_byte = segment_terms.term.byte_at(target_upto) as i32;
                let target_byte = target.bytes[target.offset + target_upto] as i32;
                cmp = term_byte - target_byte;
                if cmp != 0 {
                    break;
                }

                arc = self.arcs[1 + target_upto].clone();
                let arc_b = arc.borrow();
                debug_assert_eq!(
                    arc_b.label(),
                    target.bytes[target.offset + target_upto] as i32
                );
                segment_terms.output_accumulator.push(arc_b.output());

                if arc_b.is_final() {
                    let idx = 1 + last_frame.borrow().ord as usize;
                    last_frame = self.frame.stack[idx].clone();
                }

                target_upto += 1;
            }

            if cmp == 0 {
                let a =
                    &segment_terms.term.bytes_ref.bytes[target_upto..segment_terms.term.length()];
                let b = &target.bytes[target.offset + target_upto..target.offset + target.length];
                cmp = a.cmp(b).to_int();
            }

            if cmp < 0 {
                self.frame.current_frame = Some(last_frame);
            } else if cmp > 0 {
                segment_terms.target_before_current_length = last_frame.borrow().ord;
                last_frame.borrow_mut().rewind()?;
                self.frame.current_frame = Some(last_frame);
            } else {
                debug_assert_eq!(segment_terms.term.length(), target.length);
                if segment_terms.term_exists {
                    return Ok(Some(SeekAction::ReturnTrue));
                }
            }
        } else {
            arc = self.arcs[0].clone();
            let mut arc_b = arc.borrow_mut();
            {
                let mut segment_terms = self.segment_terms.borrow_mut();
                segment_terms.target_before_current_length = -1;

                self.segment_terms
                    .borrow()
                    .fr
                    .borrow()
                    .index
                    .as_ref()
                    .unwrap()
                    .get_first_arc(&mut *arc_b);
                debug_assert!(arc_b.is_final());

                segment_terms.output_accumulator.push(arc_b.output());

                self.frame.current_frame = Some(self.frame.static_frame.clone());

                target_upto = 0;
                segment_terms
                    .output_accumulator
                    .push(arc_b.next_final_output());
            }

            let new_frame = self.push_frame_with_length(Some(arc.clone()), 0)?;
            self.frame.current_frame = Some(new_frame);

            self.segment_terms
                .borrow_mut()
                .output_accumulator
                .pop(&arc_b.next_final_output());
        }
        while target_upto < target.length {
            let target_label = target.bytes[target.offset + target_upto] as i32;

            let next_arc = self.get_arc(1 + target_upto);
            let r = {
                let segment_terms = self.segment_terms.borrow_mut();
                let mut fr_guard = segment_terms.fr.borrow_mut();
                let fr_index = fr_guard.index.as_mut().unwrap();
                let reader = self.fst_reader.as_mut().unwrap();

                fr_index.find_target_arc(
                    target_label,
                    &mut *arc.borrow_mut(),
                    &mut *next_arc.borrow_mut(),
                    reader,
                )?
            };

            if r.is_none() {
                // index exhausted
                let mut current_frame = self.frame.current_frame.as_ref().unwrap().borrow_mut();

                self.segment_terms.borrow_mut().valid_index_prefix = current_frame.prefix_length;

                current_frame.scan_to_floor_frame(target)?;

                if !current_frame.has_terms {
                    let mut segment_terms = self.segment_terms.borrow_mut();
                    segment_terms.term_exists = false;
                    segment_terms
                        .term
                        .set_byte_at(target_upto, target_label as u8);
                    segment_terms.term.set_length(target_upto + 1);
                    return Ok(None);
                }

                if prefetch {
                    current_frame.prefetch_block()?;
                }

                return Ok(Some(SeekAction::Scan {
                    // TODO:could we avoid copy here?
                    target: target.clone(),
                    current_frame: self.frame.current_frame.as_ref().unwrap().clone(),
                }));
            } else {
                arc = next_arc;
                let arc_b = arc.borrow();
                {
                    let mut segment_terms = self.segment_terms.borrow_mut();
                    segment_terms
                        .term
                        .set_byte_at(target_upto, target_label as u8);
                    segment_terms.output_accumulator.push(arc_b.output());
                    target_upto += 1;
                }

                if arc_b.is_final() {
                    self.segment_terms
                        .borrow_mut()
                        .output_accumulator
                        .push(arc_b.next_final_output());

                    let new_frame =
                        self.push_frame_with_length(Some(arc.clone()), target_upto as i32)?;
                    self.frame.current_frame = Some(new_frame);

                    self.segment_terms
                        .borrow_mut()
                        .output_accumulator
                        .pop(&arc_b.next_final_output());
                }
            }
        }
        let mut segment_terms = self.segment_terms.borrow_mut();
        let mut current_frame = self.frame.current_frame.as_ref().unwrap().borrow_mut();

        segment_terms.valid_index_prefix = current_frame.prefix_length;

        current_frame.scan_to_floor_frame(target)?;

        if !current_frame.has_terms {
            segment_terms.term_exists = false;
            segment_terms.term.set_length(target_upto);
            return Ok(None);
        }

        if prefetch {
            current_frame.prefetch_block()?;
        }
        Ok(Some(SeekAction::Scan {
            target: target.clone(),
            current_frame: self.frame.current_frame.as_ref().unwrap().clone(),
        }))
    }
}

impl<I, P> BytesRefIterator for SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    type AV = Vec<u8>;

    fn next(&mut self) -> Result<Option<Cow<BytesRef<Self::AV>>>> {
        let input_none = {
            let segment_terms = self.segment_terms.borrow();
            segment_terms.input.is_none()
        };
        if input_none {
            let (arc, root_code) = {
                let segment_terms = self.segment_terms.borrow();
                let fr = segment_terms.fr.borrow();
                let arc = if let Some(index) = fr.index.as_ref() {
                    let mut arc = self.arcs[0].borrow_mut();
                    index.get_first_arc(&mut arc);
                    debug_assert!(arc.is_final());
                    Some(self.arcs[0].clone())
                } else {
                    None
                };
                (arc, fr.root_code.clone())
            };
            let new_frame = self.push_frame_with_data(arc, root_code, 0)?;
            self.frame.current_frame = Some(new_frame);
            self.frame
                .current_frame
                .as_ref()
                .unwrap()
                .borrow_mut()
                .load_block()?;
        }
        {
            let mut segment_terms = self.segment_terms.borrow_mut();
            let current_frame = self.frame.current_frame.as_ref().unwrap();
            segment_terms.target_before_current_length = current_frame.borrow().ord;
            debug_assert!(!segment_terms.eof);
        }

        {
            let is_static = Rc::ptr_eq(
                self.frame.current_frame.as_ref().unwrap(),
                &self.frame.static_frame,
            );
            if is_static {
                let target = {
                    let mut segment_terms = self.segment_terms.borrow_mut();
                    // TODO: avoid copy here?
                    segment_terms.term.get_bytes_ref().clone()
                };
                let found = self.seek_exact(&target)?;
                debug_assert!(found);
            }
        }
        {
            let mut segment_terms = self.segment_terms.borrow_mut();
            loop {
                let mut current_frame = self.frame.current_frame.as_ref().unwrap().borrow_mut();
                if current_frame.next_ent == current_frame.ent_count {
                    if !current_frame.is_last_in_floor {
                        current_frame.load_next_floor_block()?;
                        break;
                    } else {
                        if current_frame.ord == 0 {
                            segment_terms.eof = true;
                            segment_terms.term.clear();
                            segment_terms.valid_index_prefix = 0;
                            current_frame.rewind()?;
                            segment_terms.term_exists = false;
                            return Ok(None);
                        }

                        let last_fp = current_frame.fp_orig;
                        let parent_ord = current_frame.ord - 1;
                        drop(current_frame);
                        self.frame.current_frame =
                            Some(self.frame.stack[parent_ord as usize].clone());
                        let mut current_frame =
                            self.frame.current_frame.as_ref().unwrap().borrow_mut();

                        if current_frame.next_ent == -1 || current_frame.last_sub_fp != last_fp {
                            let target = segment_terms.term.get_bytes_ref();
                            current_frame.scan_to_floor_frame(target)?;
                            current_frame.load_block()?;
                            current_frame.scan_to_sub_block(last_fp)?;
                        }

                        let prefix = current_frame.prefix_length;
                        segment_terms.valid_index_prefix =
                            segment_terms.valid_index_prefix.min(prefix);
                    }
                } else {
                    break;
                }
            }
        }
        loop {
            let (has_next, last_sub_fp) = {
                let current_frame = self.frame.current_frame.as_ref().unwrap();
                let mut frame = current_frame.borrow_mut();
                (frame.next()?, frame.last_sub_fp)
            };
            if has_next {
                let length = self.segment_terms.borrow().term.length();
                let new_frame = self.push_frame(None, last_sub_fp, length as i32)?;
                self.frame.current_frame = Some(new_frame);
                self.frame
                    .current_frame
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .load_block()?;
                continue;
            } else {
                // could we avoid copy here?
                let term = self.segment_terms.borrow_mut().term.get_bytes_ref_copy();
                Some(term)
            };
        }
    }
}

impl<I, P> TermsEnum for SegmentTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        <BaseTermsEnum as TermsEnum>::attributes(&self.base)
    }

    fn seek_exact(&mut self, target: &BytesRef<Self::AV>) -> Result<bool> {
        let mut term_exists_supplier = self.prepare_seek_exact(target, false)?;
        Ok(term_exists_supplier.is_some() && term_exists_supplier.as_mut().unwrap().get()?)
    }

    fn seek_ceil(&mut self, target: &BytesRef<Self::AV>) -> Result<SeekStatus> {
        if self.segment_terms.borrow().fr.borrow().index.is_none() {
            return Err(LuceneError::illegal_state("terms index was not loaded"));
        }

        self.segment_terms.borrow_mut().term.grow(1 + target.length);
        debug_assert!(self.clear_eof());

        let mut target_upto;

        {
            let mut segment_terms = self.segment_terms.borrow_mut();
            segment_terms.target_before_current_length =
                self.frame.current_frame.as_ref().unwrap().borrow().ord;
            segment_terms.output_accumulator.reset();
        }
        let mut arc;

        if !Rc::ptr_eq(
            self.frame.current_frame.as_ref().unwrap(),
            &self.frame.static_frame,
        ) {
            let mut segment_terms = self.segment_terms.borrow_mut();
            arc = self.arcs[0].clone();
            debug_assert!(arc.borrow().is_final());
            let v = arc.borrow().output();
            segment_terms.output_accumulator.push(v);
            target_upto = 0;

            let mut last_frame = self.frame.stack[0].clone();
            debug_assert!(segment_terms.valid_index_prefix <= segment_terms.term.length() as i32);

            let target_limit =
                std::cmp::min(target.length, segment_terms.valid_index_prefix as usize);
            let mut cmp = 0;

            while target_upto < target_limit {
                let term_byte = segment_terms.term.byte_at(target_upto) as i32;
                let target_byte = target.bytes[target.offset + target_upto] as i32;
                cmp = term_byte - target_byte;
                if cmp != 0 {
                    break;
                }
                arc = self.arcs[1 + target_upto].clone();
                let arc_b = arc.borrow();
                debug_assert_eq!(
                    arc_b.label(),
                    target.bytes[target.offset + target_upto] as i32
                );
                segment_terms.output_accumulator.push(arc_b.output());

                if arc_b.is_final() {
                    let idx = 1 + last_frame.borrow().ord as usize;
                    last_frame = self.frame.stack[idx].clone();
                }

                target_upto += 1;
            }

            if cmp == 0 {
                cmp = segment_terms.term.bytes_ref.bytes[target_upto..segment_terms.term.length()]
                    .cmp(&target.bytes[target.offset + target_upto..target.offset + target.length])
                    .to_int();
            }

            if cmp < 0 {
                self.frame.current_frame = Some(last_frame);
            } else if cmp > 0 {
                self.segment_terms.borrow_mut().target_before_current_length = 0;
                last_frame.borrow_mut().rewind()?;
                self.frame.current_frame = Some(last_frame);
            } else {
                debug_assert_eq!(segment_terms.term.length(), target.length);
                if segment_terms.term_exists {
                    return Ok(SeekStatus::Found);
                }
            }
        } else {
            let v = {
                {
                    let mut segment_terms = self.segment_terms.borrow_mut();
                    segment_terms.target_before_current_length = -1;
                    arc = self.arcs[0].clone();
                    let mut arc_b = arc.borrow_mut();
                    segment_terms
                        .fr
                        .borrow()
                        .index
                        .as_ref()
                        .unwrap()
                        .get_first_arc(&mut *arc_b);

                    debug_assert!(arc_b.is_final());

                    segment_terms.output_accumulator.push(arc_b.output());

                    self.frame.current_frame = Some(self.frame.static_frame.clone());

                    target_upto = 0;
                    segment_terms
                        .output_accumulator
                        .push(arc_b.next_final_output());
                }
                self.push_frame_with_length(Some(self.arcs[0].clone()), 0)?
            };
            self.frame.current_frame = Some(v);
            self.segment_terms
                .borrow_mut()
                .output_accumulator
                .pop(&self.arcs[0].borrow().next_final_output());
        }
        while target_upto < target.length {
            let target_label = target.bytes[target.offset + target_upto] as i32;

            let next_arc = self.get_arc(1 + target_upto);
            let r = {
                let segment_terms = self.segment_terms.borrow_mut();
                let mut fr_guard = segment_terms.fr.borrow_mut();
                let fr_index = fr_guard.index.as_mut().unwrap();

                let reader = self.fst_reader.as_mut().unwrap();

                fr_index.find_target_arc(
                    target_label,
                    &mut *arc.borrow_mut(),
                    &mut *next_arc.borrow_mut(),
                    reader,
                )?
            };

            if r.is_none() {
                let result = {
                    let mut current_frame = self.frame.current_frame.as_ref().unwrap().borrow_mut();
                    self.segment_terms.borrow_mut().valid_index_prefix =
                        current_frame.prefix_length;

                    current_frame.scan_to_floor_frame(target)?;
                    current_frame.load_block()?;

                    current_frame.scan_to_term(target, false)?
                };
                if result == SeekStatus::End {
                    {
                        let mut segment_terms = self.segment_terms.borrow_mut();
                        segment_terms.term.copy_bytes_with_ref(target);
                        segment_terms.term_exists = false;
                    }

                    if self.next()?.is_some() {
                        return Ok(SeekStatus::NotFound);
                    } else {
                        return Ok(SeekStatus::End);
                    }
                } else {
                    return Ok(result);
                }
            } else {
                arc = next_arc;
                let arc_b = arc.borrow();
                {
                    let mut segment_terms = self.segment_terms.borrow_mut();
                    segment_terms
                        .term
                        .set_byte_at(target_upto, target_label as u8);
                    segment_terms.output_accumulator.push(arc_b.output());

                    target_upto += 1;
                }

                if arc_b.is_final() {
                    self.segment_terms
                        .borrow_mut()
                        .output_accumulator
                        .push(arc_b.next_final_output());

                    let new_frame =
                        self.push_frame_with_length(Some(arc.clone()), target_upto as i32)?;
                    self.frame.current_frame = Some(new_frame);

                    self.segment_terms
                        .borrow_mut()
                        .output_accumulator
                        .pop(&arc_b.next_final_output());
                }
            }
        }
        let result = {
            let current_frame_rc = self.frame.current_frame.as_ref().unwrap();
            let mut current_frame = current_frame_rc.borrow_mut();

            self.segment_terms.borrow_mut().valid_index_prefix = current_frame.prefix_length;
            current_frame.scan_to_floor_frame(target)?;
            current_frame.load_block()?;
            current_frame.scan_to_term(target, false)?
        };

        match result {
            SeekStatus::End => {
                {
                    let mut segment_terms = self.segment_terms.borrow_mut();
                    segment_terms.term.copy_bytes_with_ref(target);
                    segment_terms.term_exists = false;
                }
                if self.next()?.is_some() {
                    Ok(SeekStatus::NotFound)
                } else {
                    Ok(SeekStatus::End)
                }
            },
            _ => Ok(result),
        }
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn seek_exact_with_state(
        &mut self,
        target: &BytesRef<Self::AV>,
        other_state: &TermStateEnum,
    ) -> Result<()> {
        debug_assert!(self.clear_eof());
        let mut segment_terms = self.segment_terms.borrow_mut();
        if target.cmp(segment_terms.term.get_bytes_ref()).to_int() != 0
            || !segment_terms.term_exists
        {
            if let TermStateEnum::Block(block_state) = other_state {
                let mut current_frame = self.frame.static_frame.borrow_mut();
                current_frame.state.copy_from(other_state)?;
                self.frame.current_frame = Some(self.frame.static_frame.clone());
                segment_terms.term.copy_bytes_with_ref(target);
                current_frame.meta_data_upto = current_frame.get_term_block_ord();
                debug_assert!(current_frame.meta_data_upto > 0);
                segment_terms.valid_index_prefix = 0;
            }
        }
        Ok(())
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
