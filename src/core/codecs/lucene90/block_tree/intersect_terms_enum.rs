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
use crate::core::codecs::block_term_state::BlockTermStateEnum;
use crate::core::codecs::block_tree::field_reader::FieldReader;
use crate::core::codecs::block_tree::intersect_terms_enum_frame::IntersectTermsEnumFrame;
use crate::core::codecs::block_tree::segment_terms_enum::OutputAccumulator;
use crate::core::codecs::postings_reader_base::PostingsReaderBase;
use crate::core::index::BytesRef;
use crate::core::store::IndexInput;
use crate::core::util::automation::byte_runnable::ByteRunnableEnum;
use crate::core::util::automation::transition_accessor::TransitionAccessorEnum;
use crate::core::util::fst_impl::fst::Arc;
use crate::core::util::fst_impl::reverse_random_access_reader::ReverseRandomAccessReader;

pub struct IntersectTermsEnum<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase<TermState = BlockTermStateEnum>,
{
    pub(crate) input: Option<I>,
    stack: Vec<IntersectTermsEnumFrame>,
    arcs: Vec<Arc<BytesRef<std::sync::Arc<Vec<u8>>>>>,
    pub(crate) run_automation: ByteRunnableEnum,
    pub(crate) automaton: TransitionAccessorEnum,
    common_suffix: BytesRef<Vec<u8>>,
    current_frame: usize,
    current_transition: usize,
    term: BytesRef<Vec<u8>>,
    fst_reader: Option<ReverseRandomAccessReader<I::RandomAccessSlice>>,
    pub(crate) fr: FieldReader<I, P>,
    saved_start_term: BytesRef<Vec<u8>>,
    output_accumulator: OutputAccumulator,
}
