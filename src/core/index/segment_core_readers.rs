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
use crate::core::codecs::compound_directory::CompoundDirectoryEnum;
use crate::core::codecs::field_infos_format::FieldInfosFormat;
use crate::core::codecs::fields_producer::DefaultFieldsProducer;
use crate::core::codecs::norms_format::NormsFormat;
use crate::core::codecs::norms_producer::DefaultNormProducer;
use crate::core::codecs::points_format::PointsFormat;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::codecs::stored_fields_format::StoredFieldsFormat;

use crate::core::codecs::term_vectors_format::TermVectorsFormat;

use crate::core::codecs::{Codec, CompoundFormat, DefaultCompoundReader, get_default_code};
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};

use crate::core::codecs::points_reader::DefaultPointsReader;
use crate::core::codecs::stored_fields_reader::DefaultStoredFieldsReader;
use crate::core::codecs::term_vectors_reader::DefaultTermVectorsReader;
use crate::core::index::index_reader::{CacheHelper, CacheKey};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

/// Holds core readers that are shared (unchanged) when SegmentReader is cloned or reopened
pub(crate) struct SegmentCoreReaders<D>
where
    D: Directory,
{
    pub(crate) r#ref: AtomicI32,
    pub(crate) fields: Option<DefaultFieldsProducer<D::IndexInput>>,
    pub(crate) norms_producer: Option<DefaultNormProducer<D::IndexInput>>,
    pub(crate) fields_reader_orig: DefaultStoredFieldsReader<D::IndexInput>,
    pub(crate) term_vectors_reader_orig: Option<DefaultTermVectorsReader<D::IndexInput>>,
    pub(crate) points_reader: Option<DefaultPointsReader<D::IndexInput>>,
    pub(crate) cfs_reader: Option<DefaultCompoundReader<D>>,
    pub(crate) segment: String,
    /// fieldinfos for this core: means gen=-1. this is the exact fieldinfos these codec components saw at write.
    /// in the case of DV updates, SR may hold a newer version.
    pub(crate) core_field_infos: Arc<FieldInfos>,
    pub(crate) cache_helper: SegmentCoreReadersCacheHelperImpl,
}

impl<D> SegmentCoreReaders<D>
where
    D: Directory,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(dir: &D, si: &SegmentCommitInfo<D>, context: &IOContext) -> Result<Self> {
        let codec = get_default_code();
        let use_compound_file = si.info.get_use_compound_file();

        (|| {
            let cfs_reader = if use_compound_file {
                Some(codec.compound_format().get_compound_reader(dir, &si.info)?)
            } else {
                None
            };

            let cfs_dir = match cfs_reader.as_ref() {
                Some(reader) => CompoundDirectoryEnum::A(reader),
                None => CompoundDirectoryEnum::B(dir),
            };

            let segment = si.info.name.to_string();
            let core_field_infos = Arc::new(
                codec
                    .field_infos_format()
                    .read(&cfs_dir, &si.info, "", context)?,
            );

            let fields_reader_orig = codec.stored_fields_format().fields_reader(
                &cfs_dir,
                &si.info,
                core_field_infos.clone(),
                context,
            )?;

            let term_vectors_reader_orig = if core_field_infos.has_term_vectors() {
                Some(codec.term_vectors_format().vectors_reader(
                    &cfs_dir,
                    &si.info,
                    core_field_infos.clone(),
                    context,
                )?)
            } else {
                None
            };

            let read_state = SegmentReadState::new(&cfs_dir, core_field_infos.clone(), context);

            let fields = if core_field_infos.has_postings() {
                Some(
                    codec
                        .postings_format()
                        .fields_producer(&read_state, &si.info)?,
                )
            } else {
                None
            };

            let norms_producer = if core_field_infos.has_norms() {
                Some(codec.norms_format().norms_producer(&read_state, &si.info)?)
            } else {
                None
            };
            let points_reader = if core_field_infos.has_point_values() {
                Some(codec.points_format().fields_reader(&read_state, &si.info)?)
            } else {
                None
            };

            Ok(SegmentCoreReaders {
                r#ref: AtomicI32::new(1),
                fields,
                norms_producer,
                fields_reader_orig,
                term_vectors_reader_orig,
                points_reader,
                cfs_reader,
                segment,
                core_field_infos,
                cache_helper: SegmentCoreReadersCacheHelperImpl::new(),
            })
        })()
    }

    pub(crate) fn get_ref_count(&self) -> i32 {
        self.r#ref.load(Ordering::Acquire)
    }

    pub(crate) fn inc_ref(&self) -> Result<()> {
        loop {
            let count = self.r#ref.load(Ordering::Acquire);

            if count == 0 {
                return Err(LuceneError::already_closed(
                    "SegmentCoreReaders is already closed".to_string(),
                ));
            }
            if count == i32::MAX {
                return Err(LuceneError::illegal_state("ref_count overflow".to_string()));
            }

            match self.r#ref.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }
    }
    pub(crate) fn dec_ref(&self) -> Result<()> {
        self.r#ref.load(Ordering::Acquire);
        // TODO
        Ok(())
    }
    pub(crate) fn get_cache_helper_ref(&self) -> &SegmentCoreReadersCacheHelperImpl {
        &self.cache_helper
    }
    pub(crate) fn get_cache_helper(&self) -> SegmentCoreReadersCacheHelperImpl {
        self.cache_helper.clone()
    }
}
#[derive(Clone)]
pub struct SegmentCoreReadersCacheHelperImpl {
    cache_key: CacheKey,
}
impl Default for SegmentCoreReadersCacheHelperImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl SegmentCoreReadersCacheHelperImpl {
    pub fn new() -> Self {
        Self {
            cache_key: CacheKey::new(),
        }
    }
}
impl CacheHelper for SegmentCoreReadersCacheHelperImpl {
    fn get_key(&self) -> CacheKey {
        self.cache_key.clone()
    }
}
