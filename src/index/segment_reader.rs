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
use crate::codecs::compound_directory::{CompoundDirectory, CompoundDirectoryBase};
use crate::codecs::doc_values_producer::{DocValuesProducer, Either2DocValuesProducer};
use crate::codecs::field_infos_format::FieldInfosFormat;
use crate::codecs::fields_producer::FieldsProducerEnum;
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::codecs::lucene90_compound_reader::Lucene90CompoundReader;
use crate::codecs::lucene90_doc_values_producer::Lucene90DocValuesProducer;
use crate::codecs::lucene90_live_docs_format::Lucene90LiveDocsFormat;
use crate::codecs::norms_producer::{NormsProducer, NormsProducerEnum};
use crate::codecs::points_reader::PointsReaderEnum;
use crate::codecs::stored_fields_reader::StoredFieldsReaderEnum;
use crate::codecs::term_vectors_reader::TermVectorsReaderEnum;
use crate::codecs::{Codec, get_default_code};
use crate::index::codec_reader::CodecReader;
use crate::index::field_infos::FieldInfos;
use crate::index::index_reader::IndexReader;
use crate::index::leaf_metadata::LeafMetaData;
use crate::index::leaf_reader::LeafReader;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_core_readers::{CfsOrBaseInput, SegmentCoreReaders};
use crate::index::segment_doc_values::SegmentDocValues;
use crate::index::segment_doc_values_producer::SegmentDocValuesProducer;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

pub struct SegmentReader<D>
where
    D: Directory,
{
    meta_data: LeafMetaData,
    live_docs: Option<Arc<<Lucene90LiveDocsFormat as LiveDocsFormat>::Bits>>,
    hard_live_docs: Option<Arc<<Lucene90LiveDocsFormat as LiveDocsFormat>::Bits>>,
    // Normally set to si.maxDoc - si.delDocCount, unless we
    // were created as an NRT reader from IW, in which case IW
    // tells us the number of live docs:
    num_docs: i32,
    core: SegmentCoreReaders<D>,
    seg_doc_values: SegmentDocValues<D>,
    /// True if we are holding RAM only liveDocs or DV updates,
    /// i.e. the SegmentCommitInfo delGen doesn't match our liveDocs.
    is_nrt: bool,
    doc_values_producer: Option<DocValuesProducers<D>>,
    field_infos: Rc<FieldInfos>,
}
impl<D> SegmentReader<D>
where
    D: Directory,
{
    fn assert_live_docs(
        is_nrt: bool,
        hard_live_docs: &Option<Arc<<Lucene90LiveDocsFormat as LiveDocsFormat>::Bits>>,
        live_docs: &Option<Arc<<Lucene90LiveDocsFormat as LiveDocsFormat>::Bits>>,
    ) -> bool {
        match is_nrt {
            true => debug_assert!(
                hard_live_docs.is_none() || live_docs.is_some(),
                "liveDocs must be non-null if hardLiveDocs are non-null"
            ),
            false => debug_assert!(
                Arc::ptr_eq(
                    hard_live_docs.as_ref().unwrap(),
                    live_docs.as_ref().unwrap()
                ),
                "non-nrt case must have identical liveDocs"
            ),
        }
        true
    }
    /// init most recent DocValues for the current commit
    fn init_doc_values_producer(
        si: &SegmentCommitInfo<D>,
        field_infos: Rc<FieldInfos>,
        seg_doc_values: &SegmentDocValues<D>,
        core_cfs_reader: &mut Option<CompoundDirectory<Lucene90CompoundReader<D>>>,
        core_field_infos: Rc<FieldInfos>,
    ) -> Result<Option<DocValuesProducers<D>>> {
        if !field_infos.has_doc_values() {
            return Ok(None);
        }
        let dir = core_cfs_reader;

        let producer = match si.has_field_updates() {
            true => Either2DocValuesProducer::A(SegmentDocValuesProducer::new(
                si,
                dir,
                Rc::clone(&core_field_infos),
                &field_infos,
                seg_doc_values,
            )?),
            // simple case, no DocValues updates
            false => Either2DocValuesProducer::B(seg_doc_values.get_doc_values_producer(
                -1,
                si,
                dir,
                field_infos,
            )?),
        };

        Ok(Some(producer))
    }
    /// init most recent FieldInfos for the current commit
    fn init_field_infos(
        si: &SegmentCommitInfo<D>,
        core_field_infos: Rc<FieldInfos>,
    ) -> Result<Rc<FieldInfos>> {
        if !si.has_field_updates() {
            return Ok(core_field_infos);
        }

        // updates always outside of CFS
        let fis_format = get_default_code().field_infos_format();
        let segment_suffix = num_bigint::BigInt::from(si.get_field_infos_gen()).to_str_radix(36);

        let infos = fis_format.read(
            &mut *si.info.dir.lock(),
            &si.info,
            &segment_suffix,
            &IOContext::read_once_io_context()?,
        )?;

        Ok(Rc::new(infos))
    }
}
pub type DocValuesProducers<D> = Either2DocValuesProducer<
    SegmentDocValuesProducer<D>,
    Rc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>,
>;

impl<D> IndexReader for SegmentReader<D>
where
    D: Directory,
{
    fn max_doc(&self) -> Result<i32> {
        todo!()
    }

    fn num_docs(&self) -> Result<i32> {
        Ok(self.num_docs)
    }
}
impl<D> LeafReader for SegmentReader<D>
where
    D: Directory,
{
    type NumericDocValues = <DocValuesProducers<D> as DocValuesProducer>::NumericDocValues;

    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
        CodecReader::get_numeric_doc_values(self, field)
    }

    type BinaryDocValues = <DocValuesProducers<D> as DocValuesProducer>::BinaryDocValues;

    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
        CodecReader::get_binary_doc_values(self, field)
    }

    type SortedDocValues = <DocValuesProducers<D> as DocValuesProducer>::SortedDocValues;

    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
        CodecReader::get_sorted_doc_values(self, field)
    }

    type SortedNumericDocValues =
        <DocValuesProducers<D> as DocValuesProducer>::SortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        CodecReader::get_sorted_numeric_doc_values(self, field)
    }

    type SortedSetDocValues = <DocValuesProducers<D> as DocValuesProducer>::SortedSetDocValues;

    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
        CodecReader::get_sorted_set_doc_values(self, field)
    }

    type NormNumericDocValues =
        <NormsProducerEnum<CfsOrBaseInput<D>> as NormsProducer>::NumericDocValues;

    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        CodecReader::get_norm_values(self, field)
    }

    type DocValuesSkipper = <DocValuesProducers<D> as DocValuesProducer>::DocValuesSkipper;

    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        CodecReader::get_doc_values_skipper(self, field)
    }

    fn get_field_infos(&self) -> Result<&Rc<FieldInfos>> {
        Ok(&self.field_infos)
    }

    type Bits = <Lucene90LiveDocsFormat as LiveDocsFormat>::Bits;

    fn get_live_docs(&self) -> Result<Option<Arc<Self::Bits>>> {
        Ok(self.live_docs.clone())
    }
}
impl<D> CodecReader for SegmentReader<D>
where
    D: Directory,
{
    type StoredFieldsReader = StoredFieldsReaderEnum<CfsOrBaseInput<D>>;
    type TermVectorsReader = TermVectorsReaderEnum<CfsOrBaseInput<D>>;
    type NormsProducer = NormsProducerEnum<CfsOrBaseInput<D>>;
    type DocValuesProducer = DocValuesProducers<D>;
    type FieldsProducer = FieldsProducerEnum<CfsOrBaseInput<D>>;
    type PointsReader = PointsReaderEnum<CfsOrBaseInput<D>>;

    fn get_fields_reader(&self) -> Result<Cow<'_, Self::StoredFieldsReader>> {
        self.ensure_open()?;
        Ok(Cow::Owned(self.core.fields_reader_orig.clone()))
    }

    fn get_term_vectors_reader(&self) -> Result<Option<Cow<'_, Self::TermVectorsReader>>> {
        self.ensure_open()?;
        Ok(self
            .core
            .term_vectors_reader_orig
            .as_ref()
            .map(|tv| Cow::Owned(tv.clone())))
    }

    fn get_norms_reader(&self) -> Result<Option<Cow<'_, Self::NormsProducer>>> {
        self.ensure_open()?;
        Ok(self.core.norms_producer.as_ref().map(Cow::Borrowed))
    }

    fn get_doc_values_reader(&self) -> Result<Option<Cow<'_, Self::DocValuesProducer>>> {
        self.ensure_open()?;
        Ok(self.doc_values_producer.as_ref().map(Cow::Borrowed))
    }

    fn get_postings_reader(&self) -> Result<Option<Cow<'_, Self::FieldsProducer>>> {
        self.ensure_open()?;
        Ok(self.core.fields.as_ref().map(Cow::Borrowed))
    }

    fn get_points_reader(&self) -> Result<Option<Cow<'_, Self::PointsReader>>> {
        self.ensure_open()?;
        Ok(self.core.points_reader.as_ref().map(Cow::Borrowed))
    }

    fn check_integrity(&self) -> Result<()> {
        CodecReader::default_check_integrity(self)?;
        if let Some(dv) = &self.core.cfs_reader {
            dv.sub_compound_dir.check_integrity()?;
        }
        Ok(())
    }
}
