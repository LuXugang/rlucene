/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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
use crate::index::base_terms_enum::BaseTermsEnum;
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
pub struct FieldReader<I, PR>
where
    I: IndexInput,
    PR: PostingsReaderBase,
{
    pub(crate) num_terms: i64,
    pub(crate) field_info: Rc<FieldInfo>,
    pub(crate) sum_total_term_freq: i64,
    pub(crate) sum_doc_freq: i64,
    pub(crate) doc_count: i32,
    pub(crate) root_block_fp: i64,
    pub(crate) root_code: BytesRef<Rc<Vec<u8>>>,
    pub(crate) min_term: Rc<BytesRef<Vec<u8>>>,
    pub(crate) max_term: Rc<BytesRef<Vec<u8>>>,
    pub(crate) parent: Rc<TermsReader<I, PR>>,
    pub(crate) index: Option<Rc<FST<ByteSequenceOutputs, OffHeapFSTStore<I>>>>,
}
impl<I, PR> FieldReader<I, PR>
where
    I: IndexInput,
    PR: PostingsReaderBase,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new<I1: IndexInput>(
        parent: Rc<TermsReader<I, PR>>,
        field_info: Rc<FieldInfo>,
        num_terms: i64,
        root_code: BytesRef<Vec<u8>>,
        sum_total_term_freq: i64,
        sum_doc_freq: i64,
        doc_count: i32,
        index_start_fp: i64,
        meta_in: &mut I1,
        index_in: Rc<RefCell<I>>,
        min_term: Rc<BytesRef<Vec<u8>>>,
        max_term: Rc<BytesRef<Vec<u8>>>,
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
            index: Some(Rc::new(index)),
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
        let version = self.parent.version;
        if version >= lucene90_bttr_util::VERSION_MSB_VLONG_OUTPUT {
            field_reader_util::read_msb_vlong(input)
        } else {
            input.read_vlong()
        }
    }
}
impl<I, PR> Terms for FieldReader<I, PR>
where
    I: IndexInput,
    PR: PostingsReaderBase,
{
    type TermsEnum = BaseTermsEnum<SegmentTermsEnum<I, PR>>;

    fn iterator(&self) -> Result<Self::TermsEnum> {
        SegmentTermsEnum::new(self.clone())
    }

    type IntersectIter
        = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>
    where
        Self::TermsEnum: BytesRefIterator,
        AutomatonTermsEnum: FilteredTermsEnumBase;
    fn intersect(
        &self,
        compiled: &mut CompiledAutomaton,
        start_term: Option<BytesRef<Vec<u8>>>,
    ) -> Result<Self::IntersectIter> {
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

    fn get_min<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        Ok(Option::from(Cow::Borrowed(&*self.min_term)))
    }

    fn get_max<'a, T>(&'a self, _iterator: &'a mut T) -> Result<Option<Cow<'a, BytesRef<Vec<u8>>>>>
    where
        T: TermsEnum,
    {
        Ok(Option::from(Cow::Borrowed(&*self.max_term)))
    }

    fn get_stats(&self) -> Result<String> {
        todo!()
    }
}
impl<I, PR> fmt::Display for FieldReader<I, PR>
where
    I: IndexInput,
    PR: PostingsReaderBase,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlockTreeTerms(seg={} terms={} postings={} positions={} docs={})",
            self.parent.segment,
            self.num_terms,
            self.sum_doc_freq,
            self.sum_total_term_freq,
            self.doc_count
        )
    }
}
impl<I, PR> Clone for FieldReader<I, PR>
where
    I: IndexInput,
    PR: PostingsReaderBase,
{
    // used to init SegmentTermsEnum
    fn clone(&self) -> Self {
        Self {
            num_terms: self.num_terms,
            field_info: Rc::clone(&self.field_info),
            sum_total_term_freq: self.sum_total_term_freq,
            sum_doc_freq: self.sum_doc_freq,
            doc_count: self.doc_count,
            root_block_fp: self.root_block_fp,
            root_code: self.root_code.clone(),
            min_term: self.min_term.clone(),
            max_term: self.max_term.clone(),
            parent: Rc::clone(&self.parent),
            index: Some(Rc::clone(self.index.as_ref().unwrap())),
        }
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
