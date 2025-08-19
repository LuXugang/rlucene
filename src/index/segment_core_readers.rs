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
use crate::store::buffered_checksum_index_input::BufferedChecksumIndexInput;
use crate::store::directory::Directory;
use crate::store::{EitherIndexInput, IOContext, IndexInput};
use crate::util::error::lucene_error::{LuceneError, Result};

use parking_lot::Mutex;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

type CfsOrBaseInput<D> = EitherIndexInput<
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
        context: Rc<IOContext>,
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
                EitherDir::Cfs(cfs_reader.as_mut().unwrap())
            } else {
                EitherDir::Base(&mut *dir.lock())
            };

            let segment = si.info.name.to_string();
            let core_field_infos = Rc::new(codec.field_infos_format().read(
                &mut cfs_dir,
                &si.info,
                "",
                &context,
            )?);

            let fields_reader_orig = codec.stored_fields_format().fields_reader(
                &mut cfs_dir,
                &si.info,
                core_field_infos.clone(),
                &context,
            )?;

            let term_vectors_reader_orig = if core_field_infos.has_term_vectors() {
                Some(codec.term_vectors_format().vectors_reader(
                    &mut cfs_dir,
                    &si.info,
                    core_field_infos.clone(),
                    &context,
                )?)
            } else {
                None
            };

            let read_state =
                SegmentReadState::new(&mut cfs_dir, core_field_infos.clone(), context.clone());

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

pub enum EitherDir<'a, D>
where
    D: Directory,
{
    Base(&'a mut D),
    Cfs(&'a mut CompoundDirectory<Lucene90CompoundReader<D>>),
}

impl<D> Display for EitherDir<'_, D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EitherDir::Base(dir) => write!(f, "{}", dir),
            EitherDir::Cfs(cfs) => write!(f, "{}", cfs),
        }
    }
}

impl<'a, D> Directory for EitherDir<'a, D>
where
    D: Directory,
{
    fn list_all(&self) -> Result<Vec<String>> {
        match self {
            EitherDir::Base(dir) => dir.list_all(),
            EitherDir::Cfs(cfs) => cfs.list_all(),
        }
    }

    fn delete_file(&mut self, name: &str) -> Result<()> {
        match self {
            EitherDir::Base(dir) => dir.delete_file(name),
            EitherDir::Cfs(cfs) => cfs.delete_file(name),
        }
    }

    fn file_length(&self, name: &str) -> Result<i64> {
        match self {
            EitherDir::Base(dir) => dir.file_length(name),
            EitherDir::Cfs(cfs) => cfs.file_length(name),
        }
    }

    fn create_output(&mut self, _name: &str, _context: &IOContext) -> Result<Self::IndexOutput> {
        match self {
            EitherDir::Base(dir) => dir.create_output(_name, _context),
            EitherDir::Cfs(cfs) => cfs.create_output(_name, _context),
        }
    }

    type IndexOutput = D::IndexOutput;

    fn create_temp_output(
        &mut self,
        _prefix: &str,
        _suffix: &str,
        _context: &IOContext,
    ) -> Result<Self::IndexOutput> {
        match self {
            EitherDir::Base(dir) => dir.create_temp_output(_prefix, _suffix, _context),
            EitherDir::Cfs(cfs) => cfs.create_temp_output(_prefix, _suffix, _context),
        }
    }

    fn sync(&mut self, names: &[&str]) -> Result<()> {
        match self {
            EitherDir::Base(dir) => dir.sync(names),
            EitherDir::Cfs(cfs) => cfs.sync(names),
        }
    }

    fn sync_metadata(&mut self) -> Result<()> {
        match self {
            EitherDir::Base(dir) => dir.sync_metadata(),
            EitherDir::Cfs(cfs) => cfs.sync_metadata(),
        }
    }

    fn rename(&mut self, source: &str, dest: &str) -> Result<()> {
        match self {
            EitherDir::Base(dir) => dir.rename(source, dest),
            EitherDir::Cfs(cfs) => cfs.rename(source, dest),
        }
    }

    type IndexInput = CfsOrBaseInput<D>;

    fn open_input(&self, name: &str, context: &IOContext) -> Result<Self::IndexInput> {
        match self {
            EitherDir::Base(dir) => {
                let input = dir.open_input(name, context)?;
                Ok(EitherIndexInput::S(input))
            },
            EitherDir::Cfs(cfs) => {
                let input = cfs.open_input(name, context)?;
                Ok(EitherIndexInput::F(input))
            },
        }
    }

    fn open_checksum_input(
        &self,
        name: &str,
    ) -> Result<BufferedChecksumIndexInput<Self::IndexInput>> {
        let input = self.open_input(name, &IOContext::default_io_context()?)?;
        Ok(BufferedChecksumIndexInput::new(input))
    }

    type Lock = D::Lock;

    fn obtain_lock(&mut self, name: &str) -> Result<Self::Lock> {
        match self {
            EitherDir::Base(dir) => dir.obtain_lock(name),
            EitherDir::Cfs(cfs) => cfs.obtain_lock(name),
        }
    }

    fn copy_from<T: Directory>(
        &mut self,
        from: Arc<Mutex<T>>,
        src: &str,
        dest: &str,
        context: &IOContext,
    ) -> Result<()> {
        match self {
            EitherDir::Base(dir) => dir.copy_from(from, src, dest, context),
            EitherDir::Cfs(cfs) => cfs.copy_from(from, src, dest, context),
        }
    }

    fn delete_files_ignoring_exceptions(&mut self, files: &[String]) {
        match self {
            EitherDir::Base(dir) => dir.delete_files_ignoring_exceptions(files),
            EitherDir::Cfs(cfs) => cfs.delete_files_ignoring_exceptions(files),
        }
    }

    fn get_pending_deletions(&mut self) -> Result<HashSet<String>> {
        match self {
            EitherDir::Base(dir) => dir.get_pending_deletions(),
            EitherDir::Cfs(cfs) => cfs.get_pending_deletions(),
        }
    }

    fn is_fs_directory(&self) -> bool {
        match self {
            EitherDir::Base(dir) => dir.is_fs_directory(),
            EitherDir::Cfs(cfs) => cfs.is_fs_directory(),
        }
    }
}
