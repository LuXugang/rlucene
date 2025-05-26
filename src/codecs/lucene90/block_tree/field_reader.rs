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
use std::fmt;
use std::rc::Rc;

use crate::codecs::lucene90::block_tree::lucene90_block_tree_terms_reader::{
    lucene90_bttr_util, TermsReader,
};
use crate::codecs::lucene90::block_tree::segment_terms_enum::SegmentTermsEnum;
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::index::field_info::FieldInfo;
use crate::index::filtered_terms_enum::{FilteredTermsEnum, FilteredTermsEnumBase};
use crate::index::index_options::IndexOptions;
use crate::index::terms::Terms;
use crate::index::terms_enum::TermsEnum;
use crate::index::BytesRef;
use crate::store::{ByteArrayDataInput, DataInput, IndexInput};
use crate::util::automation::compiled_automaton::CompiledAutomaton;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::Result;
use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
use crate::util::fst_impl::fst::{fst_util, FST};
use crate::util::fst_impl::off_heap_fst_store::OffHeapFSTStore;
use crate::util::ToInt;

/// BlockTree's implementation of [`Terms`].
#[allow(clippy::type_complexity)]
pub struct FieldReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    pub(crate) num_terms: i64,
    pub(crate) field_info: Rc<FieldInfo>,
    pub(crate) sum_total_term_freq: i64,
    pub(crate) sum_doc_freq: i64,
    pub(crate) doc_count: i32,
    pub(crate) root_block_fp: i64,
    pub(crate) root_code: BytesRef<Rc<Vec<u8>>>,
    pub(crate) min_term: BytesRef<Vec<u8>>,
    pub(crate) max_term: BytesRef<Vec<u8>>,
    pub(crate) parent: Rc<RefCell<TermsReader<I, P>>>,
    // FieldReader needs to be held as an immutable reference in SegmentTermsEnum, but
    // FST#get_bytes_reader requires a mutable borrow. Therefore, we define `index` with interior
    // mutability by `RefCell`.
    pub(crate) index:
        Option<RefCell<FST<BytesRef<Rc<Vec<u8>>>, ByteSequenceOutputs, OffHeapFSTStore<I>>>>,
}
impl<I, P> FieldReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new<I1: IndexInput>(
        parent: Rc<RefCell<TermsReader<I, P>>>,
        field_info: Rc<FieldInfo>,
        num_terms: i64,
        root_code: BytesRef<Vec<u8>>,
        sum_total_term_freq: i64,
        sum_doc_freq: i64,
        doc_count: i32,
        index_start_fp: i64,
        meta_in: &mut I1,
        index_in: Rc<RefCell<I>>,
        min_term: BytesRef<Vec<u8>>,
        max_term: BytesRef<Vec<u8>>,
    ) -> Result<Self> {
        assert!(num_terms > 0);
        // Read FST metadata and build the index
        let metadata = fst_util::read_metadata(meta_in, ByteSequenceOutputs)?;
        let store = OffHeapFSTStore::new(index_in, index_start_fp, metadata.num_bytes);
        let index = FST::from_fst_reader(Some(metadata), Some(store))
            .expect("metadata and store are some, should not return None");
        let empty_output = index.metadata().empty_output().cloned();

        let mut v = Self {
            parent,
            field_info,
            num_terms,
            sum_total_term_freq,
            sum_doc_freq,
            doc_count,
            // init with padding value
            root_block_fp: 0,
            // init with padding value
            root_code: BytesRef::new(),
            min_term,
            max_term,
            index: Some(RefCell::new(index)),
        };
        // ownership to ByteArrayDataInput
        let mut input =
            ByteArrayDataInput::with_range(root_code.bytes, root_code.offset, root_code.length);
        v.root_block_fp = v.read_vlong_output(&mut input)?;
        // ownership from ByteArrayDataInput
        let root_code = BytesRef {
            bytes: Rc::new(input.bytes),
            offset: root_code.offset,
            length: root_code.length,
        };
        // Get empty output and adjust rootCode
        let root_code_final = match empty_output {
            Some(empty_output) => {
                if root_code.bytes_equals(&empty_output) {
                    empty_output
                } else {
                    root_code
                }
            },
            None => root_code,
        };
        v.root_code = root_code_final;
        Ok(v)
    }
    pub(crate) fn read_vlong_output(&self, input: &mut impl DataInput) -> Result<i64> {
        let version = self.parent.borrow().version;
        if version >= lucene90_bttr_util::VERSION_MSB_VLONG_OUTPUT {
            field_reader_util::read_msb_vlong(input)
        } else {
            input.read_vlong()
        }
    }
}
impl<I, P> Terms for FieldReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    type AV = Vec<u8>;
    type TermsEnum<'a>
        = SegmentTermsEnum<'a, I, P>
    where
        Self: 'a;

    fn iterator(&self) -> Result<Self::TermsEnum<'_>> {
        SegmentTermsEnum::new(self)
    }

    type IntersectIter<'a>
        = FilteredTermsEnum<Self::TermsEnum<'a>, Self::AV, AutomatonTermsEnum>
    where
        Self::TermsEnum<'a>: BytesRefIterator<AV = Self::AV>,
        AutomatonTermsEnum: FilteredTermsEnumBase<AV = Self::AV>,
        I: 'a,
        P: 'a;

    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter<'_>> {
        self.default_intersect(compiled, start_term)
    }

    fn size(&self) -> Result<i64> {
        Ok(self.num_terms)
    }

    fn get_sum_total_term_freq(&self) -> Result<i64> {
        Ok(self.sum_total_term_freq)
    }

    fn get_sum_doc_freq(&self) -> Result<i64> {
        Ok(self.sum_doc_freq)
    }

    fn get_doc_count(&self) -> Result<i32> {
        Ok(self.doc_count)
    }

    fn has_freqs(&self) -> bool {
        self.field_info
            .get_index_options()
            .cmp(&IndexOptions::DocsAndFreqs)
            .to_int()
            >= 0
    }

    fn has_offsets(&self) -> bool {
        self.field_info
            .get_index_options()
            .cmp(&IndexOptions::DocsAndFreqsAndPositionsAndOffsets)
            .to_int()
            >= 0
    }

    fn has_positions(&self) -> bool {
        self.field_info
            .get_index_options()
            .cmp(&IndexOptions::DocsAndFreqsAndPositions)
            .to_int()
            >= 0
    }

    fn has_payloads(&self) -> bool {
        self.field_info.has_payloads()
    }

    fn get_min<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Self::AV>>>>
    where
        T: TermsEnum<AV = Self::AV>,
    {
        Ok(Option::from(Cow::Borrowed(&self.min_term)))
    }

    fn get_max<T>(&self, _iterator: &mut T) -> Result<Option<Cow<BytesRef<Self::AV>>>> {
        Ok(Option::from(Cow::Borrowed(&self.max_term)))
    }

    fn get_stats(&self) -> Result<String> {
        todo!()
    }
}
impl<I, P> fmt::Display for FieldReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockTreeTerms(seg={} terms={} postings={} positions={} docs={})",
            self.parent.borrow().segment,
            self.num_terms,
            self.sum_doc_freq,
            self.sum_total_term_freq,
            self.doc_count
        )
    }
}

pub(crate) mod field_reader_util {
    use crate::store::DataInput;

    /// Decodes a variable-length `byte[]` in MSB order back to a `long`,
    /// as written by
    /// [`Lucene90BlockTreeTermsWriter::write_msb_vlong`](crate::codecs::lucene90::lucene90_block_trree_terms_writer::Lucene90BlockTreeTermsWriter::write_msb_vlong).
    ///
    ///
    /// Package-private for testing.
    pub(crate) fn read_msb_vlong(
        input: &mut impl DataInput,
    ) -> crate::util::error::lucene_error::Result<i64> {
        let mut l: i64 = 0;
        loop {
            let b = input.read_byte()?;
            l = (l << 7) | (b & 0x7F) as i64;
            if (b & 0x80) == 0 {
                break;
            }
        }
        Ok(l)
    }
}
