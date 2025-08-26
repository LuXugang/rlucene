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
use crate::codecs::Codec;
use crate::codecs::compressing::lucene90_compressing_term_vectors_format::Lucene90CompressingTermVectorsFormat;
use crate::codecs::compression::compression_mode::CompressionModeEnum;
use crate::codecs::term_vectors_format::TermVectorsFormat;
use crate::codecs::term_vectors_reader::TermVectorsReader;
use crate::codecs::term_vectors_writer::{TermVectorsWriter, TermVectorsWriterEnum};
use crate::index::field_infos::FieldInfos;
use crate::index::fields::Fields;
use crate::index::postings_enum::{PostingsEnum, postings_enum_util};
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::sorting_stored_fields_consumer::NoCompression;
use crate::index::term_vectors::TermVectors;
use crate::index::term_vectors_consumer::TermVectorsConsumerBase;
use crate::index::terms::Terms;
use crate::index::terms_enum::TermsEnum;
use crate::index::tracking_tmp_output_directory_wrapper::TrackingTmpOutputDirectoryWrapper;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::store::flush_info::FlushInfo;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{IOUtils, ToInt};
use parking_lot::Mutex;
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct SortingTermVectorsConsumer<D>
where
    D: Directory,
{
    pub(crate) writer: Option<TermVectorsWriterEnum<TrackingTmpOutputDirectoryWrapper<D>>>,
    tmp_directory: Arc<Mutex<TrackingTmpOutputDirectoryWrapper<D>>>,
    stored_fields_format: Option<Lucene90CompressingTermVectorsFormat>,
}
impl<D> SortingTermVectorsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(directory: Arc<Mutex<D>>) -> Self {
        let tmp_directory = Arc::new(Mutex::new(TrackingTmpOutputDirectoryWrapper::new(
            directory,
        )));
        Self {
            writer: None,
            tmp_directory,
            stored_fields_format: None,
        }
    }

    fn write_term_vectors<TVW, F>(
        writer: &mut TVW,
        vectors: &Option<F>,
        field_infos: &Rc<FieldInfos>,
    ) -> Result<()>
    where
        TVW: TermVectorsWriter,
        F: Fields,
    {
        if vectors.is_none() {
            writer.start_document(0)?;
            writer.finish_document()?;
            return Ok(());
        }
        let vectors = vectors.as_ref().unwrap();

        let mut num_fields = vectors.size()?;
        if num_fields == -1 {
            // count manually! TODO: Maybe enforce that Fields.size() returns something valid?
            for _ in vectors.iterator() {
                num_fields += 1;
            }
        }
        writer.start_document(num_fields)?;
        let mut last_field_name: Option<String> = None;
        let mut docs_and_positions = None;
        let mut field_count = 0;
        let mut terms_enum;
        for field_name in vectors.iterator() {
            field_count += 1;
            let field_info = match field_infos.field_info_by_name(field_name) {
                Some(fi) => fi,
                None => {
                    return Err(LuceneError::illegal_state(format!(
                        "Field '{field_name}' not found in FieldInfos"
                    )));
                },
            };

            debug_assert!({
                let v = last_field_name.is_none()
                    || field_name.cmp(last_field_name.as_ref().unwrap()).to_int() > 0;
                last_field_name = Some(field_name.clone());
                v
            });

            let terms = match vectors.terms(field_name)? {
                Some(t) => t,
                None => continue,
            };

            let has_positions = terms.has_positions();
            let has_offsets = terms.has_offsets();
            let has_payloads = terms.has_payloads();
            debug_assert!(!has_payloads || has_positions);

            let mut num_terms = terms.size()?;
            if num_terms == -1 {
                // count manually. It is stupid, but needed, as Terms.size() is not a mandatory statistics
                // function
                num_terms = 0;
                terms_enum = terms.iterator()?;
                while terms_enum.next()?.is_some() {
                    num_terms += 1;
                }
            }
            writer.start_field(
                &field_info,
                num_terms as usize,
                has_positions,
                has_offsets,
                has_payloads,
            )?;
            terms_enum = terms.iterator()?;
            let mut term_count = 0;
            while terms_enum.next()?.is_some() {
                term_count += 1;

                let freq = terms_enum.total_term_freq()? as i32;
                writer.start_term(&*terms_enum.term()?, freq)?;

                if has_positions || has_offsets {
                    docs_and_positions = Some(terms_enum.postings_with_flags(
                        docs_and_positions,
                        (postings_enum_util::OFFSETS | postings_enum_util::PAYLOADS) as i32,
                    )?);
                    match docs_and_positions {
                        Some(ref mut dap) => {
                            let doc_id = dap.next_doc()?;
                            debug_assert!(doc_id != NO_MORE_DOCS);
                            debug_assert!(dap.freq()? == freq);

                            for _ in 0..freq {
                                let pos = dap.next_position()?;
                                let start_offset = dap.start_offset()?;
                                let end_offset = dap.end_offset()?;
                                let payload = dap.get_payload()?;
                                debug_assert!(!has_positions || pos >= 0);
                                writer.add_position(
                                    pos,
                                    start_offset,
                                    end_offset,
                                    payload.as_ref().map(Cow::as_ref),
                                )?;
                            }
                        },
                        None => {
                            debug_assert!(false, "docs_and_positions is None");
                        },
                    }
                }
                writer.finish_term()?;
            }
            debug_assert!(term_count == num_terms);
            writer.finish_field()?;
        }
        debug_assert!(field_count == num_fields);
        writer.finish_document()?;
        Ok(())
    }
}

impl<D> TermVectorsConsumerBase for SortingTermVectorsConsumer<D>
where
    D: Directory,
{
    type Directory = D;

    fn flush<DM, D1>(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        sort_map: &Option<Rc<DM>>,
        codec: &impl Codec,
        segment_info: &SegmentInfo<D1>,
    ) -> Result<()>
    where
        DM: DocMap,
        D1: Directory,
    {
        {
            let mut tmp_dir = self.tmp_directory.lock();
            let mut reader = self.stored_fields_format.as_mut().unwrap().vectors_reader(
                &mut *tmp_dir,
                segment_info,
                state.field_infos.clone(),
                &IOContext::default_io_context()?,
            )?;
            // Don't pull a merge instance, since merge instances optimize for
            // sequential access while term vectors will likely be accessed in random
            // order here.
            let mut writer = codec.term_vectors_format().vectors_writer(
                state.directory,
                segment_info,
                &state.context.clone(),
            )?;

            reader.check_integrity()?;
            let max_doc = segment_info.max_doc()?;
            for doc_id in 0..max_doc {
                let read_id = match sort_map {
                    Some(sm) => sm.new_to_old(doc_id),
                    None => doc_id,
                };
                let vectors = reader.get(read_id)?;
                Self::write_term_vectors(&mut writer, &vectors, &state.field_infos)?;
            }
            writer.finish(max_doc, state.directory)?;
            let name_map: Vec<String> = tmp_dir.get_temporary_files().into_values().collect();
            let names: Vec<&str> = name_map.iter().map(String::as_str).collect();
            IOUtils::delete_files(&mut *tmp_dir, names.as_slice())?;
        }
        Ok(())
    }

    fn init_term_vectors_writer<D1>(
        &mut self,
        last_doc_id: i32,
        info: &SegmentInfo<D1>,
        bytes_used: i64,
    ) -> Result<()>
    where
        D1: Directory,
    {
        let context = IOContext::with_flush(FlushInfo::new(last_doc_id, bytes_used))?;
        let term_vectors_format = Lucene90CompressingTermVectorsFormat::new(
            "TempTermVectors",
            "",
            CompressionModeEnum::Impl(NoCompression),
            8 * 1024,
            128,
            10,
        )?;
        self.writer = Option::from(term_vectors_format.vectors_writer(
            &mut *self.tmp_directory.lock(),
            info,
            &context,
        )?);
        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        let mut tmp_dir = self.tmp_directory.lock();
        let name_map: Vec<String> = tmp_dir.get_temporary_files().into_values().collect();
        let names: Vec<&str> = name_map.iter().map(String::as_str).collect();
        IOUtils::delete_files(&mut *tmp_dir, names.as_slice())?;
        Ok(())
    }
}
