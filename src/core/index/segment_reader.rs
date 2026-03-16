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
use crate::core::codecs::compound_directory::CompoundDirectoryBase;
use crate::core::codecs::doc_values_producer::{
    DefaultDocValuesProducer, DocValuesProducer, DocValuesProducerEnum2,
};
use crate::core::codecs::field_infos_format::FieldInfosFormat;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::codecs::norms_producer::{DefaultNormProducer, NormsProducer};
use crate::core::codecs::points_reader::{DefaultPointsReader, PointsReader};

use crate::core::codecs::fields_producer::DefaultFieldsProducer;
use crate::core::codecs::stored_fields_reader::DefaultStoredFieldsReader;
use crate::core::codecs::term_vectors_reader::DefaultTermVectorsReader;
use crate::core::codecs::{Codec, get_default_code};
use crate::core::index::codec_reader::{CodecReader, StoredFieldsType, TermVectorsType};
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::fields::Fields;
use crate::core::index::index_reader::{CacheHelper, CacheKey, IndexReader, IndexReaderBase};
use crate::core::index::leaf_metadata::LeafMetaData;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::pending_deletes::DocBits;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_core_readers::{
    SegmentCoreReaders, SegmentCoreReadersCacheHelperImpl,
};
use crate::core::index::segment_doc_values::SegmentDocValues;
use crate::core::index::segment_doc_values_producer::SegmentDocValuesProducer;
use crate::core::index::term::Term;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::bits::{Bits, BitsEnum2};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// IndexReader implementation over a single segment.
/// Instances pointing to the same segment
/// (but with different deletes, etc.) may share the same core data
pub struct SegmentReader<D>
where
    D: Directory,
{
    pub(crate) si: SegmentCommitInfo<D>,
    pub(crate) original_si_id: String,
    pub(crate) original_si_dir: Arc<D>,
    meta_data: LeafMetaData,
    live_docs: Option<DocBits>,
    hard_live_docs: Option<DocBits>,
    // Normally set to si.maxDoc - si.delDocCount, unless we
    // were created as an NRT reader from IW, in which case IW
    // tells us the number of live docs:
    num_docs: i32,
    core: Arc<SegmentCoreReaders<D>>,
    seg_doc_values: Arc<SegmentDocValues<D>>,
    /// True if we are holding RAM only liveDocs or DV updates,
    /// i.e. the SegmentCommitInfo delGen doesn't match our liveDocs.
    is_nrt: bool,
    doc_values_producer: Option<Arc<DocValuesProducers<D>>>,
    field_infos: Arc<FieldInfos>,
    index_base: IndexReaderBase,
    reader_cache_helper: CacheHelperImpl,
}
impl<D> SegmentReader<D>
where
    D: Directory,
{
    pub(crate) fn new(
        si: &SegmentCommitInfo<D>,
        created_version_major: i32,
        context: &IOContext,
    ) -> Result<Self> {
        let si = si.clone();
        let meta_data = LeafMetaData::new(
            created_version_major,
            si.info.get_min_version(),
            si.info.get_index_sort().clone(),
            si.info.get_has_blocks(),
        )?;

        let is_nrt = false;
        let core = Arc::new(SegmentCoreReaders::new(si.info.dir.as_ref(), &si, context)?);
        let seg_doc_values = Arc::new(SegmentDocValues::new());
        let num_docs = si.info.max_doc()? - si.get_del_count();
        let info_id = si.info.get_id_str();
        let dir = si.info.dir.clone();
        let mut segment_reader = Self {
            si,
            original_si_id: info_id,
            original_si_dir: dir,
            meta_data,
            is_nrt,
            core,
            seg_doc_values,
            hard_live_docs: None,
            live_docs: None,
            num_docs,
            field_infos: Arc::new(FieldInfos::default()),
            doc_values_producer: None,
            index_base: IndexReaderBase::new(),
            reader_cache_helper: CacheHelperImpl::new(),
        };
        let result = (|| {
            let si = &segment_reader.si;
            let (hard_live_docs, live_docs) = if si.has_deletions() {
                // NOTE: the bitvector is stored using the regular directory, not cfs
                let ld = Arc::new(get_default_code().live_docs_format().read_live_docs(
                    si.info.dir.as_ref(),
                    si,
                    &IOContext::read_once_io_context()?,
                )?);
                (Some(DocBits::A(ld.clone())), Some(DocBits::A(ld)))
            } else {
                debug_assert_eq!(si.get_del_count(), 0);
                (None, None)
            };

            let field_infos =
                Self::init_field_infos(si, segment_reader.core.core_field_infos.clone())?;
            let doc_values_producer = Self::init_doc_values_producer(
                si,
                field_infos.clone(),
                &segment_reader.seg_doc_values,
                &segment_reader.core,
            )?;

            debug_assert!(Self::assert_live_docs(
                is_nrt,
                hard_live_docs.as_ref(),
                live_docs.as_ref()
            )?);

            Ok((hard_live_docs, live_docs, field_infos, doc_values_producer))
        })();
        match result {
            Ok(r) => {
                segment_reader.hard_live_docs = r.0;
                segment_reader.live_docs = r.1;
                segment_reader.field_infos = r.2;
                segment_reader.doc_values_producer = r.3;
                Ok(segment_reader)
            },
            Err(e) => {
                segment_reader.do_close()?;
                Err(e)
            },
        }
    }
    /// Create new SegmentReader sharing core from a previous SegmentReader and using the provided liveDocs,
    /// and recording whether those liveDocs were carried in ram (isNRT=true).
    pub(crate) fn new_from_reader(
        si: &SegmentCommitInfo<D>,
        sr: &SegmentReader<D>,
        live_docs: Option<DocBits>,
        hard_live_docs: Option<DocBits>,
        num_docs: i32,
        is_nrt: bool,
    ) -> Result<Self> {
        let si = si.clone();
        let max_doc = si.info.max_doc()?;
        if num_docs > max_doc {
            return Err(LuceneError::illegal_argument(format!(
                "numDocs={} but maxDoc={}",
                num_docs, max_doc
            )));
        }
        if let Some(ld) = &live_docs {
            let len = ld.length();
            if len != max_doc as usize {
                return Err(LuceneError::illegal_argument(format!(
                    "maxDoc={} but liveDocs.size()={}",
                    max_doc, len
                )));
            }
        }

        let meta_data = sr.meta_data.clone();
        let core = sr.core.clone();
        let seg_doc_values = sr.seg_doc_values.clone();
        core.inc_ref()?;
        debug_assert!(Self::assert_live_docs(
            is_nrt,
            hard_live_docs.as_ref(),
            live_docs.as_ref()
        )?);
        let info_id = si.info.get_id_str();
        let dir = si.info.dir.clone();
        let mut segment_reader = Self {
            si,
            original_si_id: info_id,
            original_si_dir: dir,
            meta_data,
            is_nrt,
            core: core.clone(),
            seg_doc_values: seg_doc_values.clone(),
            hard_live_docs,
            live_docs,
            num_docs,
            field_infos: Arc::new(FieldInfos::default()),
            doc_values_producer: None,
            index_base: IndexReaderBase::new(),
            reader_cache_helper: CacheHelperImpl::new(),
        };
        let result = (|| {
            let si = &segment_reader.si;
            let field_infos = Self::init_field_infos(si, core.core_field_infos.clone())?;
            let doc_values_producer =
                Self::init_doc_values_producer(si, field_infos.clone(), &seg_doc_values, &core)?;
            Ok((field_infos, doc_values_producer))
        })();
        match result {
            Ok(r) => {
                segment_reader.field_infos = r.0;
                segment_reader.doc_values_producer = r.1;
                Ok(segment_reader)
            },
            Err(e) => {
                segment_reader.do_close()?;
                Err(e)
            },
        }
    }
    fn assert_live_docs(
        is_nrt: bool,
        hard_live_docs: Option<&DocBits>,
        live_docs: Option<&DocBits>,
    ) -> Result<bool> {
        match is_nrt {
            true => debug_assert!(
                hard_live_docs.is_none() || live_docs.is_some(),
                "liveDocs must be non-null if hardLiveDocs are non-null"
            ),
            false => debug_assert!(
                match (hard_live_docs, live_docs) {
                    (None, None) => true,
                    (Some(reader_bits), Some(current_bits)) => match (reader_bits, current_bits) {
                        (BitsEnum2::A(r_bits), BitsEnum2::A(c_bits)) => Arc::ptr_eq(r_bits, c_bits),
                        (BitsEnum2::B(r_bits), BitsEnum2::B(c_bits)) => match (r_bits, c_bits) {
                            (BitsEnum2::A(r_fixed), BitsEnum2::A(c_fixed)) => {
                                Arc::ptr_eq(r_fixed, c_fixed)
                            },
                            (BitsEnum2::B(_), BitsEnum2::B(_)) => {
                                return Err(LuceneError::illegal_state(
                                    "live docs should be FixedBitSet",
                                ));
                            },
                            _ => false,
                        },
                        _ => false,
                    },
                    _ => false,
                },
                "non-nrt case must have identical liveDocs"
            ),
        }
        Ok(true)
    }
    /// init most recent DocValues for the current commit
    fn init_doc_values_producer(
        si: &SegmentCommitInfo<D>,
        field_infos: Arc<FieldInfos>,
        seg_doc_values: &SegmentDocValues<D>,
        core: &SegmentCoreReaders<D>,
    ) -> Result<Option<Arc<DocValuesProducers<D>>>> {
        if !field_infos.has_doc_values() {
            return Ok(None);
        }
        let dir = &core.cfs_reader;

        let producer = match si.has_field_updates() {
            true => DocValuesProducerEnum2::A(SegmentDocValuesProducer::new(
                si,
                dir.as_ref(),
                Arc::clone(&core.core_field_infos),
                &field_infos,
                seg_doc_values,
            )?),
            // simple case, no DocValues updates
            false => DocValuesProducerEnum2::B(seg_doc_values.get_doc_values_producer(
                -1,
                si,
                dir.as_ref(),
                field_infos,
            )?),
        };

        Ok(Some(Arc::new(producer)))
    }
    /// init most recent FieldInfos for the current commit
    fn init_field_infos(
        si: &SegmentCommitInfo<D>,
        core_field_infos: Arc<FieldInfos>,
    ) -> Result<Arc<FieldInfos>> {
        if !si.has_field_updates() {
            return Ok(core_field_infos);
        }

        // updates always outside of CFS
        let fis_format = get_default_code().field_infos_format();
        let segment_suffix = num_bigint::BigInt::from(si.get_field_infos_gen()).to_str_radix(36);

        let infos = fis_format.read(
            si.info.dir.as_ref(),
            &si.info,
            &segment_suffix,
            &IOContext::read_once_io_context()?,
        )?;

        Ok(Arc::new(infos))
    }
    /// Return the name of the segment this reader is reading.
    pub fn get_segment_name(&self) -> &str {
        &self.si.info.name
    }
    /// Return the SegmentInfoPerCommit of the segment this reader is reading.
    pub fn get_segment_info(&self) -> &SegmentCommitInfo<D> {
        &self.si
    }
    /// Returns the directory this index resides in.
    pub fn directory(&self) -> &D {
        self.si.info.dir.as_ref()
    }
    pub fn get_original_segment_info_id(&self) -> &str {
        &self.original_si_id
    }
    pub fn get_original_dir(&self) -> Arc<D> {
        self.original_si_dir.clone()
    }
}
pub type DocValuesProducers<D> = DocValuesProducerEnum2<
    SegmentDocValuesProducer<D>,
    Arc<DefaultDocValuesProducer<<D as Directory>::IndexInput>>,
>;

impl<D> Display for SegmentReader<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let v = self.si.info.max_doc().expect("max_doc should be set")
            - self.num_docs
            - self.si.get_del_count();
        write!(f, "{}", self.si.to_string_with_pending_del_count(v))
    }
}

impl<D> IndexReader for SegmentReader<D>
where
    D: Directory,
{
    type TermVectors = TermVectorsType<<Self as CodecReader>::TermVectorsReader>;

    fn term_vectors(&self) -> Result<Self::TermVectors> {
        CodecReader::term_vectors(self)
    }

    fn max_doc(&self) -> Result<i32> {
        self.si.info.max_doc()
    }

    fn num_docs(&self) -> Result<i32> {
        Ok(self.num_docs)
    }

    type StoredFields = StoredFieldsType<<Self as CodecReader>::StoredFieldsReader>;

    fn stored_fields(&self) -> Result<Self::StoredFields> {
        CodecReader::stored_fields(self)
    }

    fn do_close(&self) -> Result<()> {
        if self.core.dec_ref().is_err()
            && let Some(dv) = &self.doc_values_producer
        {
            match dv.as_ref() {
                DocValuesProducerEnum2::A(a) => self.seg_doc_values.dec_ref(&a.dv_gens)?,
                DocValuesProducerEnum2::B(_) => {
                    let gens = vec![-1_i64, 1];
                    self.seg_doc_values.dec_ref(&gens)?
                },
            }
        }
        Ok(())
    }

    type ReaderCacheHelper = CacheHelperImpl;

    fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
        Ok(Some(self.reader_cache_helper.clone()))
    }

    fn doc_freq(&self, term: &Term) -> Result<i32> {
        LeafReader::doc_freq(self, term)
    }

    fn total_term_freq(&self, term: &Term) -> Result<i64> {
        LeafReader::get_total_term_freq(self, term)
    }

    fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
        LeafReader::get_sum_doc_freq(self, field)
    }

    fn get_doc_count(&self, field: &str) -> Result<i32> {
        LeafReader::get_doc_count(self, field)
    }

    fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
        LeafReader::get_sum_total_term_freq(self, field)
    }

    fn index_base(&self) -> &IndexReaderBase {
        &self.index_base
    }
}
impl<D> LeafReader for SegmentReader<D>
where
    D: Directory,
{
    type CacheHelper = SegmentCoreReadersCacheHelperImpl;

    fn get_core_cache_helper_ref(&self) -> Result<Option<&Self::CacheHelper>> {
        Ok(Option::from(self.core.get_cache_helper_ref()))
    }

    fn get_core_cache_helper(&self) -> Result<Option<Self::CacheHelper>> {
        Ok(Option::from(self.core.get_cache_helper()))
    }

    type Terms = <<Self as CodecReader>::FieldsProducer as Fields>::Terms;

    fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
        CodecReader::terms(self, field)
    }

    type NumericDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::NumericDocValues;

    fn get_numeric_doc_values(&self, field: &str) -> Result<Option<Self::NumericDocValues>> {
        CodecReader::get_numeric_doc_values(self, field)
    }

    type BinaryDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::BinaryDocValues;

    fn get_binary_doc_values(&self, field: &str) -> Result<Option<Self::BinaryDocValues>> {
        CodecReader::get_binary_doc_values(self, field)
    }

    type SortedDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedDocValues;

    fn get_sorted_doc_values(&self, field: &str) -> Result<Option<Self::SortedDocValues>> {
        CodecReader::get_sorted_doc_values(self, field)
    }

    type SortedNumericDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedNumericDocValues;

    fn get_sorted_numeric_doc_values(
        &self,
        field: &str,
    ) -> Result<Option<Self::SortedNumericDocValues>> {
        CodecReader::get_sorted_numeric_doc_values(self, field)
    }

    type SortedSetDocValues =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::SortedSetDocValues;

    fn get_sorted_set_doc_values(&self, field: &str) -> Result<Option<Self::SortedSetDocValues>> {
        CodecReader::get_sorted_set_doc_values(self, field)
    }

    type NormNumericDocValues =
        <<Self as CodecReader>::NormsProducer as NormsProducer>::NumericDocValues;

    fn get_norm_values(&self, field: &str) -> Result<Option<Self::NormNumericDocValues>> {
        CodecReader::get_norm_values(self, field)
    }

    type DocValuesSkipper =
        <<Self as CodecReader>::DocValuesProducer as DocValuesProducer>::DocValuesSkipper;

    fn get_doc_values_skipper(&self, field: &str) -> Result<Option<Self::DocValuesSkipper>> {
        CodecReader::get_doc_values_skipper(self, field)
    }

    fn get_field_infos(&self) -> Result<Arc<FieldInfos>> {
        Ok(self.field_infos.clone())
    }

    type Bits = DocBits;

    fn get_live_docs(&self) -> Result<Option<Self::Bits>> {
        match &self.live_docs {
            Some(DocBits::A(a)) => Ok(Some(DocBits::A(Arc::clone(a)))),
            Some(DocBits::B(b)) => match b {
                BitsEnum2::A(a) => Ok(Some(DocBits::B(BitsEnum2::A(Arc::clone(a))))),
                BitsEnum2::B(_) => Err(LuceneError::illegal_state(
                    "live docs should be FixedBitSet",
                )),
            },
            None => Ok(None),
        }
    }

    type PointValues = <<Self as CodecReader>::PointsReader as PointsReader>::PointValuesType;

    fn get_point_values(&self, field: &str) -> Result<Option<Self::PointValues>> {
        CodecReader::get_point_values(self, field)
    }

    fn check_integrity(&self) -> Result<()> {
        CodecReader::default_check_integrity(self)?;
        if let Some(dv) = &self.core.cfs_reader {
            dv.sub_compound_dir.check_integrity()?;
        }
        Ok(())
    }

    fn get_metadata(&self) -> Result<&LeafMetaData> {
        Ok(&self.meta_data)
    }
}
impl<D> CodecReader for SegmentReader<D>
where
    D: Directory,
{
    type StoredFieldsReader = DefaultStoredFieldsReader<D::IndexInput>;
    type TermVectorsReader = DefaultTermVectorsReader<D::IndexInput>;
    type NormsProducer = Arc<DefaultNormProducer<D::IndexInput>>;
    type DocValuesProducer = Arc<DocValuesProducers<D>>;
    type FieldsProducer = Arc<DefaultFieldsProducer<D::IndexInput>>;
    type PointsReader = Arc<DefaultPointsReader<D::IndexInput>>;

    fn get_fields_reader(&self) -> Result<Option<Self::StoredFieldsReader>> {
        self.ensure_open()?;
        Ok(Some(self.core.fields_reader_orig.clone()))
    }

    fn get_term_vectors_reader(&self) -> Result<Option<Self::TermVectorsReader>> {
        self.ensure_open()?;
        Ok(self.core.term_vectors_reader_orig.clone())
    }

    fn get_norms_reader(&self) -> Result<Option<Self::NormsProducer>> {
        self.ensure_open()?;
        Ok(self.core.norms_producer.clone())
    }

    fn get_doc_values_reader(&self) -> Result<Option<Self::DocValuesProducer>> {
        self.ensure_open()?;
        Ok(self.doc_values_producer.clone())
    }

    fn get_postings_reader(&self) -> Result<Option<Self::FieldsProducer>> {
        self.ensure_open()?;
        Ok(self.core.fields.clone())
    }

    fn get_points_reader(&self) -> Result<Option<Self::PointsReader>> {
        self.ensure_open()?;
        Ok(self.core.points_reader.clone())
    }
}
#[derive(Clone)]
pub struct CacheHelperImpl {
    cache_key: CacheKey,
}
impl CacheHelperImpl {
    fn new() -> Self {
        Self {
            cache_key: CacheKey::new(),
        }
    }
}
impl CacheHelper for CacheHelperImpl {
    fn get_key(&self) -> CacheKey {
        self.cache_key.clone()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::core::document::document::Document;
    use crate::core::index::BytesRef;

    use crate::core::index::field_infos::get_indexed_fields;
    use crate::core::index::fields::Fields;
    use crate::core::index::index_options::IndexOptions;
    use crate::core::index::index_reader::IndexReader;
    use crate::core::index::indexable_field::IndexableField;
    use crate::core::index::indexable_field_type::IndexableFieldType;
    use crate::core::index::leaf_reader::LeafReader;
    use crate::core::index::multi_doc_values::MultiDocValues;
    use crate::core::index::multi_reader::MultiReader;
    use crate::core::index::multi_terms::{get_term_postings_enum, get_terms};
    use crate::core::index::postings_enum::PostingsEnum;
    use crate::core::index::segment_reader::SegmentReader;
    use crate::core::index::stored_fields::StoredFields;
    use crate::core::index::term_vectors::TermVectors;
    use crate::core::index::terms::Terms;
    use crate::core::index::terms_enum::TermsEnum;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::core::store::IOContext;
    use crate::core::store::directory::DirEnum;
    use crate::core::util::LATEST;
    use crate::core::util::bytes_ref_iterator::BytesRefIterator;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::core::index::doc_helper::NameValue::{Str, String};
    use crate::test::core::index::doc_helper::{
        DATA, DocHelper, FIELD_2_TEXT, FIELDS, NAME_VALUES, NO_NORMS_KEY, NO_NORMS_TEXT,
        TEXT_FIELD_1_KEY, TEXT_FIELD_2_KEY,
    };
    use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, random,
    };
    use crate::test::core::util::test_util::TestUtil;
    use rand::Rng;
    use std::collections::HashSet;
    use std::sync::Arc;

    pub(crate) struct TestSegmentReader;
    impl TestSegmentReader {
        pub(crate) fn check_norms<LR>(reader: LR) -> Result<()>
        where
            LR: LeafReader + Clone,
        {
            let multi_readers = MultiReader::with_leaf_reader(vec![reader.clone()])?;
            for f in FIELDS.iter() {
                if *f.field_type().index_options() != IndexOptions::None {
                    let field_name = f.name();
                    let norms_opt = reader.get_norm_values(field_name)?;
                    assert_eq!(norms_opt.is_some(), !f.field_type().omit_norms());
                    assert_eq!(norms_opt.is_some(), !DATA.no_norms.contains_key(field_name));
                    if norms_opt.is_none() {
                        // test for norms of null
                        let norms2 = MultiDocValues::get_norm_values(&multi_readers, field_name)?;
                        assert!(norms2.is_none());
                    }
                }
            }
            Ok(())
        }
    }

    fn set_up<R: Rng + ?Sized>(
        random: &mut R,
    ) -> Result<(Arc<DirEnum>, Document, SegmentReader<DirEnum>)> {
        let dir = new_directory_shared(random)?;
        let mut documnet = Document::new();
        DocHelper::setup_doc(&mut documnet);
        let info = DocHelper::write_doc(dir.clone(), documnet.clone())?;
        let reader = SegmentReader::new(&info, LATEST.major, &IOContext::default_io_context()?)?;
        Ok((dir, documnet, reader))
    }
    #[test]
    fn test() -> Result<()> {
        let mut random = random();
        let (_dir, document, _reader) = set_up(&mut random)?;
        assert!(!NAME_VALUES.is_empty());
        assert_eq!(DocHelper::num_fields(&document), DATA.all.len());
        Ok(())
    }
    #[test]
    fn test_document() -> Result<()> {
        let mut random = random();
        let (_dir, test_doc, reader) = set_up(&mut random)?;
        assert_eq!(reader.num_docs()?, 1);

        assert!(reader.max_doc()? >= 1);
        let mut stored_fields = reader.stored_fields()?;
        let result = stored_fields.document(0)?;
        assert_eq!(
            DocHelper::num_fields(&result),
            DocHelper::num_fields(&test_doc) - DATA.unstored.len()
        );
        let fields = result.get_fields();
        for field in fields {
            assert!(NAME_VALUES.contains_key(field.name()));
        }

        Ok(())
    }
    #[test]
    fn test_get_field_name_variations() -> Result<()> {
        let mut random = random();
        let (_dir, _doc, reader) = set_up(&mut random)?;

        let mut all_field_names = HashSet::new();
        let mut indexed_field_names = HashSet::new();
        let mut not_indexed_field_names = HashSet::new();
        let mut tv_field_names = HashSet::new();
        let mut no_tv_field_names = HashSet::new();

        let field_infos = reader.get_field_infos()?;
        for field_info in field_infos.iter() {
            let name = field_info.name.to_string();
            all_field_names.insert(name.clone());

            if *field_info.get_index_options() != IndexOptions::None {
                indexed_field_names.insert(name.clone());
            } else {
                not_indexed_field_names.insert(name.clone());
            }

            if field_info.has_term_vectors() {
                tv_field_names.insert(name.clone());
            } else if *field_info.get_index_options() != IndexOptions::None {
                no_tv_field_names.insert(name.clone());
            }
        }

        assert_eq!(all_field_names.len(), DATA.all.len());
        for s in &all_field_names {
            assert!(NAME_VALUES.contains_key(s) || s.is_empty());
        }

        assert_eq!(indexed_field_names.len(), DATA.indexed.len());
        for s in &indexed_field_names {
            assert!(DATA.indexed.contains_key(s) || s.is_empty());
        }

        assert_eq!(not_indexed_field_names.len(), DATA.unindexed.len());
        assert_eq!(tv_field_names.len(), DATA.term_vector.len());
        assert_eq!(no_tv_field_names.len(), DATA.no_term_vector.len());

        Ok(())
    }
    #[test]
    fn test_terms() -> Result<()> {
        let mut random = random();
        let (_dir, _doc, reader) = set_up(&mut random)?;
        let reader = Arc::new(reader);
        let multi_reader = MultiReader::with_leaf_reader(vec![reader.clone()])?;
        let fields = get_indexed_fields(&multi_reader)?;
        for field in fields {
            let terms = get_terms(&multi_reader, &field)?;
            assert!(terms.is_some());
            let terms = terms.unwrap();
            let mut terms_enum = terms.iterator()?;
            while terms_enum.next()?.is_some() {
                let term = terms_enum.term()?;

                let field_value = match NAME_VALUES.get(&field).unwrap() {
                    String(v) => v.clone(),
                    Str(v) => v.to_string(),
                    _ => unreachable!(),
                };
                assert!(field_value.contains(&term.utf8_to_string()?));
            }
        }

        let mut term_docs = TestUtil::docs_with_reader(
            &mut random,
            &multi_reader,
            TEXT_FIELD_1_KEY,
            &BytesRef::from_string("field"),
            None,
            0,
        )?
        .expect("term_docs should be some");
        assert_ne!(term_docs.next_doc()?, NO_MORE_DOCS);

        let mut term_docs = TestUtil::docs_with_reader(
            &mut random,
            &multi_reader,
            NO_NORMS_KEY,
            &BytesRef::from_string(NO_NORMS_TEXT),
            None,
            0,
        )?
        .expect("term_docs should be some");
        assert_ne!(term_docs.next_doc()?, NO_MORE_DOCS);

        let mut positions = get_term_postings_enum(
            &multi_reader,
            TEXT_FIELD_1_KEY,
            &BytesRef::from_string("field"),
        )?
        .expect("positions should be some");
        assert_ne!(positions.next_doc()?, NO_MORE_DOCS);
        assert_eq!(positions.doc_id(), 0);
        assert!(positions.next_position()? >= 0);

        Ok(())
    }
    #[test]
    fn test_norms() -> Result<()> {
        let mut random = random();
        let (_dir, _doc, reader) = set_up(&mut random)?;
        let reader = Arc::new(reader);
        TestSegmentReader::check_norms(reader)?;
        Ok(())
    }
    #[test]
    fn test_term_vectors() -> Result<()> {
        let mut random = random();
        let (_dir, _doc, reader) = set_up(&mut random)?;
        let reader = Arc::new(reader);

        let multi_reader = MultiReader::with_leaf_reader(vec![reader.clone()])?;

        let mut term_vectors = multi_reader.term_vectors()?;
        let tv0 = term_vectors.get(0)?.expect("tv0 should exist");
        let result = tv0.terms(TEXT_FIELD_2_KEY)?;
        assert!(result.is_some());
        let result = result.unwrap();

        assert_eq!(result.size()?, 3);

        let mut terms_enum = result.iterator()?;
        while terms_enum.next()?.is_some() {
            let term = terms_enum.term()?.utf8_to_string()?;
            let freq = terms_enum.total_term_freq()? as i32;
            assert!(FIELD_2_TEXT.contains(&term));
            assert!(freq > 0);
        }

        let results = term_vectors.get(0)?.expect("results should exist");
        assert_eq!(results.size()?, 3);

        Ok(())
    }
    #[test]
    fn test_out_of_bounds_access() -> Result<()> {
        // this test is not required in Rust Lucene
        Ok(())
    }
}
