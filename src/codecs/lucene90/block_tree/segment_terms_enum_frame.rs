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

use crate::codecs::block_term_state::BlockTermStateEnum;
use crate::codecs::lucene90::block_tree::segment_terms_enum::SegmentTermsEnum;
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::index::BytesRef;
use crate::store::{ByteArrayDataInput, IndexInput};
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::fst::Arc;
use crate::util::fst_impl::fst_reader::FstReader;

pub struct SegmentTermsEnumFrame<I, P, R, F>
where
    I: IndexInput,
    P: PostingsReaderBase<I>,
    F: FstReader,
{
    /// Our index in stack[]
    pub(crate) ord: i32,

    pub(crate) has_terms: bool,
    pub(crate) has_terms_orig: bool,
    pub(crate) is_floor: bool,

    pub(crate) arc: Option<Arc<BytesRef<Rc<Vec<u8>>>>>,

    /// File pointer where this block was loaded from
    pub(crate) fp: i64,
    pub(crate) fp_orig: i64,
    pub(crate) fp_end: i64,
    pub(crate) total_suffix_bytes: i64, // for stats

    pub(crate) suffix_bytes: Vec<u8>,
    pub(crate) suffixes_reader: ByteArrayDataInput<Vec<u8>>,

    pub(crate) suffix_length_bytes: Vec<u8>,
    pub(crate) suffix_lengths_reader: ByteArrayDataInput<Vec<u8>>,

    pub(crate) stat_bytes: Vec<u8>,
    pub(crate) stats_singleton_run_length: i32,
    pub(crate) stats_reader: ByteArrayDataInput<Vec<u8>>,

    pub(crate) rewind_pos: i32,
    pub(crate) floor_data_reader: ByteArrayDataInput<Vec<u8>>,

    // Length of prefix shared by all terms in this block
    pub(crate) prefix_length: i32,

    // Number of entries (term or sub-block) in this block
    pub(crate) ent_count: i32,

    // Which term we will next read, or -1 if the block isn't loaded yet
    pub(crate) next_ent: i32,

    // True if this block is either not a floor block, or it's the last sub-block of a floor block
    pub(crate) is_last_in_floor: bool,

    // True if all entries are terms
    pub(crate) is_leaf_block: bool,

    // True if all entries have the same length.
    pub(crate) all_equal: bool,

    pub(crate) last_sub_fp: i64,

    pub(crate) next_floor_label: i32,
    pub(crate) num_follow_floor_blocks: i32,

    // Next term to decode metaData; we decode metaData
    // lazily so that scanning to find the matching term is
    // fast and only if you find a match and app wants the
    // stats or docs/positions enums, will we decode the
    // metaData
    pub(crate) meta_data_upto: i32,

    pub(crate) state: BlockTermStateEnum,

    // metadata buffer
    pub(crate) bytes: Vec<u8>,
    pub(crate) bytes_reader: ByteArrayDataInput<Vec<u8>>,

    /// parent SegmentTermsEnum
    ste: Rc<RefCell<SegmentTermsEnum<I, P, R, F>>>,
}
impl<I, P, R, F> SegmentTermsEnumFrame<I, P, R, F>
where
    I: IndexInput,
    P: PostingsReaderBase<I>,
    F: FstReader,
{
    pub fn new(ste: Rc<RefCell<SegmentTermsEnum<I, P, R, F>>>, ord: i32) -> Result<Self> {
        let mut state = ste
            .borrow()
            .fr
            .parent
            .borrow_mut()
            .postings_reader
            .new_term_state()?;

        state.get_block_term_state().total_term_freq = -1;

        Ok(Self {
            ord,
            ste,
            state,

            arc: None,
            has_terms: false,
            has_terms_orig: false,
            is_floor: false,

            fp: 0,
            fp_orig: 0,
            fp_end: 0,
            total_suffix_bytes: 0,

            suffix_bytes: vec![0u8; 128],
            suffixes_reader: ByteArrayDataInput::new(),

            suffix_length_bytes: vec![0u8; 32],
            suffix_lengths_reader: ByteArrayDataInput::new(),

            stat_bytes: vec![0u8; 64],
            stats_singleton_run_length: 0,
            stats_reader: ByteArrayDataInput::new(),

            rewind_pos: 0,
            floor_data_reader: ByteArrayDataInput::new(),

            prefix_length: 0,
            ent_count: 0,
            next_ent: 0,

            is_last_in_floor: false,
            is_leaf_block: false,
            all_equal: false,

            last_sub_fp: 0,
            next_floor_label: 0,
            num_follow_floor_blocks: 0,

            meta_data_upto: 0,

            bytes: vec![0u8; 32],
            bytes_reader: ByteArrayDataInput::new(),
        })
    }
}
