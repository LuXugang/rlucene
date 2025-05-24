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
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::rc::Rc;

use crate::codecs::lucene90::block_tree::field_reader::FieldReader;
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::codecs::CodecUtil;
use crate::index::field_infos::FieldInfos;
use crate::index::fields::Fields;
use crate::index::index_options::IndexOptions;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::IndexFileNames;
use crate::store::directory::Directory;
use crate::store::{DataInput, IndexInput, ReadAdvice};
use crate::util::error::lucene_error::{LuceneError, Result};
/// A block-based terms index and dictionary that assigns terms to variable
/// length blocks according to how they share prefixes. The terms index is a
/// prefix trie whose leaves are term blocks. The advantage of this approach is
/// that `seek_exact` is often able to determine a term cannot exist without
/// doing any IO, and intersection with Automata is very fast. Note that this
/// terms dictionary has its own fixed terms index (i.e., it does not support a
/// pluggable terms index implementation).
///
/// **NOTE**: this terms dictionary supports `min/max_items_per_block` during
/// indexing to control how much memory the terms index uses.
///
/// The data structure used by this implementation is very similar to a [burst
/// trie] (http://citeseer.ist.psu.edu/viewdoc/summary?doi=10.1.1.18.3499), but with added logic to break
/// up too-large blocks of all terms sharing a given prefix into smaller ones.
///
/// Use `CheckIndex` with the `-verbose` option to see summary statistics on the
/// blocks in the dictionary.
///
/// See [`Lucene90BlockTreeTermsWriter`](crate::codecs::lucene90::block_tree::lucene90_block_tree_terms_writer::Lucene90BlockTreeTermsWriter).
///
/// [`Lucene90BlockTreeTermsWriter`]: crate::codecs::lucene90::writer::Lucene90BlockTreeTermsWriter
pub struct Lucene90BlockTreeTermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    terms_reader: Rc<RefCell<TermsReader<I, P>>>,
    // Open input to the terms index file (_X.tip)
    index_in: Rc<RefCell<I>>,
    field_map: HashMap<i32, FieldReader<I, P>>,
    field_list: Vec<String>,
    field_infos: Rc<FieldInfos>,
}
pub struct TermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    // Open input to the main terms dict file (_X.tib)
    pub(crate) terms_in: I,
    pub(crate) postings_reader: P,
    pub(crate) segment: String,
    pub(crate) version: i32,
}

impl<I, P> Lucene90BlockTreeTermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    pub fn new<D>(postings_reader: P, state: &SegmentReadState<D>) -> Result<Self>
    where
        D: Directory<IndexInputType = I>,
    {
        let segment = state.segment_info.name.clone();

        let terms_name = IndexFileNames::segment_file_name(
            &segment,
            &state.segment_suffix,
            lucene90_bttr_util::TERMS_EXTENSION,
        );

        let mut terms_in = state
            .directory
            .lock()
            .open_input(&terms_name, &state.context)?;

        let version = CodecUtil::check_index_header(
            &mut terms_in,
            lucene90_bttr_util::TERMS_CODEC_NAME,
            lucene90_bttr_util::VERSION_START,
            lucene90_bttr_util::VERSION_CURRENT,
            state.segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let index_name = IndexFileNames::segment_file_name(
            &segment,
            &state.segment_suffix,
            lucene90_bttr_util::TERMS_INDEX_EXTENSION,
        );

        let mut index_in = state.directory.lock().open_input(
            &index_name,
            &state.context.with_read_advice(ReadAdvice::RandomPreload)?,
        )?;

        CodecUtil::check_index_header(
            &mut index_in,
            lucene90_bttr_util::TERMS_INDEX_CODEC_NAME,
            version,
            version,
            state.segment_info.get_id(),
            &state.segment_suffix,
        )?;

        let meta_name = IndexFileNames::segment_file_name(
            &segment,
            &state.segment_suffix,
            lucene90_bttr_util::TERMS_META_EXTENSION,
        );

        let mut field_map = HashMap::new();
        let mut index_length = -1i64;
        let mut terms_length = -1i64;

        let mut prior_error = None;
        let mut meta_in = state.directory.lock().open_checksum_input(&meta_name)?;
        let index_in = Rc::new(RefCell::new(index_in));
        let terms_reader = Rc::new(RefCell::new(TermsReader {
            terms_in,
            postings_reader,
            segment,
            version,
        }));
        let result: Result<()> = (|| {
            CodecUtil::check_index_header(
                &mut meta_in,
                lucene90_bttr_util::TERMS_META_CODEC_NAME,
                version,
                version,
                state.segment_info.get_id(),
                &state.segment_suffix,
            )?;
            terms_reader
                .borrow_mut()
                .postings_reader
                .init(&mut meta_in, state)?;

            let num_fields = meta_in.read_vint()?;
            if num_fields < 0 {
                return Err(LuceneError::corrupt_index(format!(
                    "invalid numFields: {}",
                    num_fields
                )));
            }

            for _ in 0..num_fields {
                let field = meta_in.read_vint()?;
                let num_terms = meta_in.read_vlong()?;
                if num_terms <= 0 {
                    return Err(LuceneError::corrupt_index(format!(
                        "Illegal numTerms for field number: {}",
                        field
                    )));
                }

                let root_code = lucene90_bttr_util::read_bytes_ref(&mut meta_in)?;
                let field_info =
                    state
                        .field_infos
                        .field_info_by_number(field)?
                        .ok_or_else(|| {
                            LuceneError::corrupt_index(format!("invalid field number: {}", field))
                        })?;

                let sum_total_term_freq = meta_in.read_vlong()?;
                // when frequencies are omitted, sumDocFreq=sumTotalTermFreq and only one value
                // is written.
                let sum_doc_freq = if *field_info.get_index_options() == IndexOptions::DOCS {
                    sum_total_term_freq
                } else {
                    meta_in.read_vlong()?
                };

                let doc_count = meta_in.read_vint()?;
                let min_term = lucene90_bttr_util::read_bytes_ref(&mut meta_in)?;
                let mut max_term = lucene90_bttr_util::read_bytes_ref(&mut meta_in)?;

                if num_terms == 1 {
                    assert_eq!(max_term, min_term);
                    // save heap for edge case of a single term only so min == max
                    max_term = min_term.clone();
                }

                let max_doc = state.segment_info.max_doc()?;
                if doc_count < 0 || doc_count > max_doc {
                    return Err(LuceneError::corrupt_index(format!(
                        "invalid docCount: {} maxDoc: {}",
                        doc_count, max_doc
                    )));
                }

                if sum_doc_freq < doc_count as i64 {
                    return Err(LuceneError::corrupt_index(format!(
                        "invalid sumDocFreq: {} docCount: {}",
                        sum_doc_freq, doc_count
                    )));
                }

                if sum_total_term_freq < sum_doc_freq {
                    return Err(LuceneError::corrupt_index(format!(
                        "invalid sumTotalTermFreq: {} sumDocFreq: {}",
                        sum_total_term_freq, sum_doc_freq
                    )));
                }

                let index_start_fp = meta_in.read_vlong()?;

                let reader = FieldReader::new(
                    terms_reader.clone(),
                    field_info.clone(),
                    num_terms,
                    root_code,
                    sum_total_term_freq,
                    sum_doc_freq,
                    doc_count,
                    index_start_fp,
                    &mut meta_in,
                    index_in.clone(),
                    min_term,
                    max_term,
                )?;

                if field_map
                    .insert(field_info.get_field_number(), reader)
                    .is_some()
                {
                    return Err(LuceneError::corrupt_index(format!(
                        "duplicate field: {}",
                        field_info.name
                    )));
                }
            }

            index_length = meta_in.read_long()?;
            terms_length = meta_in.read_long()?;
            Ok(())
        })();

        match result {
            Ok(_) => {},
            Err(e) => {
                prior_error = Some(e);
            },
        }

        if let Some(mut e) = prior_error {
            return Err(CodecUtil::check_footer_with_error(&mut meta_in, &mut e));
        } else {
            CodecUtil::check_footer(&mut meta_in)?;
        }
        // At this point the checksum of the meta file has been verified so the lengths
        // are likely correct
        CodecUtil::retrieve_checksum_with_expected(&mut *index_in.borrow_mut(), index_length)?;
        CodecUtil::retrieve_checksum_with_expected(
            &mut terms_reader.borrow_mut().terms_in,
            terms_length,
        )?;

        let field_list = lucene90_bttr_util::sort_field_names(&field_map, &state.field_infos)?;
        Ok(Lucene90BlockTreeTermsReader {
            terms_reader,
            index_in,
            field_map,
            field_list,
            field_infos: Rc::clone(&state.field_infos),
        })
    }
}
impl<I, P> Fields for Lucene90BlockTreeTermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    fn iterator(&self) -> &[String] {
        &self.field_list
    }

    type Terms = FieldReader<I, P>;

    fn terms(&mut self, field: &str) -> Result<Option<&mut Self::Terms>> {
        let field_info = self.field_infos.field_info_by_name(field);
        match field_info {
            Some(f) => Ok(self.field_map.get_mut(&f.number)),
            None => Ok(None),
        }
    }

    fn size(&self) -> i32 {
        debug_assert!(self.field_map.len() <= i32::MAX as usize);
        self.field_map.len() as i32
    }
}
impl<I, P> Display for Lucene90BlockTreeTermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lucene90BlockTreeTermsReader(fields={}, delegate={})",
            self.field_map.len(),
            self.terms_reader.borrow().postings_reader
        )
    }
}

pub mod lucene90_bttr_util {
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::codecs::block_tree::field_reader::FieldReader;
    use crate::codecs::postings_reader_base::PostingsReaderBase;
    use crate::index::field_infos::FieldInfos;
    use crate::index::BytesRef;
    use crate::store::IndexInput;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
    use crate::util::fst_impl::outputs::Outputs;

    pub(crate) const OUTPUT_FLAGS_NUM_BITS: i32 = 2;
    pub(crate) const OUTPUT_FLAGS_MASK: i32 = 0x3;
    pub(crate) const OUTPUT_FLAG_IS_FLOOR: i32 = 0x1;
    pub(crate) const OUTPUT_FLAG_HAS_TERMS: i32 = 0x2;

    /// Extension of terms file
    pub(crate) const TERMS_EXTENSION: &str = "tim";
    pub(crate) const TERMS_CODEC_NAME: &str = "BlockTreeTermsDict";
    /// Initial terms format
    pub const VERSION_START: i32 = 0;
    /// Version that encodes output as MSB VLong for better FST sharing
    /// (GITHUB#12620)
    pub const VERSION_MSB_VLONG_OUTPUT: i32 = 1;
    /// Version that specializes arc store for continuous label in FST
    pub const VERSION_FST_CONTINUOUS_ARCS: i32 = 2;
    /// Current terms format version
    pub const VERSION_CURRENT: i32 = VERSION_FST_CONTINUOUS_ARCS;
    /// Extension of terms index file
    pub(crate) const TERMS_INDEX_EXTENSION: &str = "tip";
    pub(crate) const TERMS_INDEX_CODEC_NAME: &str = "BlockTreeTermsIndex";
    /// Extension of terms meta file
    pub(crate) const TERMS_META_EXTENSION: &str = "tmd";
    pub(crate) const TERMS_META_CODEC_NAME: &str = "BlockTreeTermsMeta";
    thread_local! {
        pub(crate) static NO_OUTPUT:BytesRef<Rc<Vec<u8>>> ={let v = ByteSequenceOutputs::get_singleton(); v.get_no_output()};
    }
    pub(super) fn read_bytes_ref<I: IndexInput>(input: &mut I) -> Result<BytesRef<Vec<u8>>> {
        let num_bytes = input.read_vint()?;
        if num_bytes < 0 {
            return Err(LuceneError::corrupt_index(format!(
                "invalid bytes length: {} (resource={})",
                num_bytes, input
            )));
        }
        let mut buffer = vec![0u8; num_bytes as usize];
        input.read_bytes(&mut buffer, 0, num_bytes)?;
        Ok(BytesRef::from_slice(buffer, 0, num_bytes as usize))
    }
    pub(super) fn sort_field_names<I, P>(
        field_map: &HashMap<i32, FieldReader<I, P>>,
        field_infos: &FieldInfos,
    ) -> Result<Vec<String>>
    where
        I: IndexInput,
        P: PostingsReaderBase,
    {
        let mut field_names = Vec::with_capacity(field_map.len());

        for field_number in field_map.keys() {
            let field_info = field_infos
                .field_info_by_number(*field_number)?
                .ok_or_else(|| {
                    LuceneError::illegal_state(format!(
                        "Missing field info for field number {}",
                        field_number
                    ))
                })?;
            field_names.push(field_info.name.clone());
        }

        field_names.sort();
        Ok(field_names)
    }
}
