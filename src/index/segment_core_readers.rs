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
use crate::codecs::compound_directory::CompoundDirectory;
use crate::codecs::field_infos_format::FieldInfosFormat;
use crate::codecs::fields_producer::FieldsProducerEnum;
use crate::codecs::lucene90_compound_reader::Lucene90CompoundReader;
use crate::codecs::norms_format::NormsFormat;
use crate::codecs::norms_producer::NormsProducerEnum;
use crate::codecs::points_format::PointsFormat;
use crate::codecs::points_reader::PointsReaderEnum;
use crate::codecs::postings_format::PostingsFormat;
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_reader::StoredFieldsReaderEnum;
use crate::codecs::term_vectors_format::TermVectorsFormat;
use crate::codecs::term_vectors_reader::TermVectorsReaderEnum;
use crate::codecs::{Codec, CompoundFormat, get_default_code};
use crate::index::field_infos::FieldInfos;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_read_state::SegmentReadState;
use crate::store::directory::{Directory, Either2Directory};
use crate::store::{Either2IndexInput, IOContext, IndexInput};
use crate::util::error::lucene_error::{LuceneError, Result};

use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

pub(crate) type CfsOrBaseInput<D> = Either2IndexInput<
    <<D as Directory>::IndexInput as IndexInput>::Slice,
    <D as Directory>::IndexInput,
>;

/// Holds core readers that are shared (unchanged) when SegmentReader is cloned or reopened
pub(crate) struct SegmentCoreReaders<D>
where
    D: Directory,
{
    pub(crate) r#ref: AtomicI32,
    pub(crate) fields: Option<FieldsProducerEnum<CfsOrBaseInput<D>>>,
    pub(crate) norms_producer: Option<NormsProducerEnum<CfsOrBaseInput<D>>>,
    pub(crate) fields_reader_orig: StoredFieldsReaderEnum<CfsOrBaseInput<D>>,
    pub(crate) term_vectors_reader_orig: Option<TermVectorsReaderEnum<CfsOrBaseInput<D>>>,
    pub(crate) points_reader: Option<PointsReaderEnum<CfsOrBaseInput<D>>>,
    pub(crate) cfs_reader: Option<CompoundDirectory<Lucene90CompoundReader<D>>>,
    pub(crate) segment: String,
    /// fieldinfos for this core: means gen=-1. this is the exact fieldinfos these codec components saw at write.
    /// in the case of DV updates, SR may hold a newer version.
    pub(crate) core_field_infos: Rc<FieldInfos>,
}

impl<D> SegmentCoreReaders<D>
where
    D: Directory,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        dir: Arc<Mutex<D>>,
        si: SegmentCommitInfo<D>,
        context: &IOContext,
    ) -> Result<Self> {
        let codec = get_default_code();
        let use_compound_file = si.info.get_use_compound_file();

        (|| {
            let mut cfs_reader = if use_compound_file {
                Some(
                    codec
                        .compound_format()
                        .get_compound_reader(&mut *dir.lock(), &si.info)?,
                )
            } else {
                None
            };

            let mut cfs_dir = if cfs_reader.is_some() {
                Either2Directory::A(cfs_reader.as_mut().unwrap())
            } else {
                Either2Directory::B(&mut *dir.lock())
            };

            let segment = si.info.name.to_string();
            let core_field_infos = Rc::new(codec.field_infos_format().read(
                &mut cfs_dir,
                &si.info,
                "",
                context,
            )?);

            let fields_reader_orig = codec.stored_fields_format().fields_reader(
                &mut cfs_dir,
                &si.info,
                core_field_infos.clone(),
                context,
            )?;

            let term_vectors_reader_orig = if core_field_infos.has_term_vectors() {
                Some(codec.term_vectors_format().vectors_reader(
                    &mut cfs_dir,
                    &si.info,
                    core_field_infos.clone(),
                    context,
                )?)
            } else {
                None
            };

            let read_state = SegmentReadState::new(&mut cfs_dir, core_field_infos.clone(), context);

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
                Some(codec.points_format().fields_reader(&read_state)?)
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
    pub(crate) fn dec_ref(&mut self) -> Result<()> {
        self.r#ref.load(Ordering::Acquire);
        0;
        todo!()
    }
}
type Either2Dir<'a, D> =
    Either2Directory<&'a mut D, &'a mut CompoundDirectory<Lucene90CompoundReader<D>>>;
