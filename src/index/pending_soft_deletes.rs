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
use crate::codecs::field_infos_format::FieldInfosFormat;
use crate::codecs::{Codec, CompoundFormat, get_default_code};
use crate::index::field_infos::FieldInfos;
use crate::index::pending_deletes::{LiveDocsBits, PendingDeletes};
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_reader::SegmentReader;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use num_bigint::BigInt;
use parking_lot::Mutex;
use std::sync::Arc;

pub(crate) struct PendingSoftDeletes<D>
where
    D: Directory,
{
    pub(crate) field: Option<String>,
    pub(crate) dv_generation: i64,
    pub(crate) hard_deletes: PendingDeletes<D>,
    pub(crate) base: PendingDeletes<D>,
}
impl<D> PendingSoftDeletes<D>
where
    D: Directory,
{
    pub(crate) fn new(field: Option<String>, info: &SegmentCommitInfo<D>) -> Result<Self> {
        let base = PendingDeletes::new(info)?;
        let hard_deletes = PendingDeletes::new(info)?;
        Ok(Self {
            field,
            dv_generation: -2,
            hard_deletes,
            base,
        })
    }

    pub(crate) fn from_reader(
        field: Option<String>,
        reader: &SegmentReader<D>,
        info: &SegmentCommitInfo<D>,
    ) -> Result<Self> {
        let base = PendingDeletes::from_reader(reader, info)?;
        let hard_deletes = PendingDeletes::from_reader(reader, info)?;
        Ok(Self {
            field,
            dv_generation: -2,
            hard_deletes,
            base,
        })
    }
    pub(crate) fn delete(&mut self, doc_id: i32, info: &mut SegmentCommitInfo<D>) -> Result<bool> {
        match self.field {
            Some(_) => {
                // we need to fetch this first it might be a shared instance with
                let mutable_bits = self.base.get_mutable_bits()?;
                // hardDeletes
                if self.hard_deletes.delete(doc_id)? {
                    if mutable_bits.get_and_clear(doc_id) {
                        // delete it here too!
                        debug_assert!(!self.hard_deletes.delete(doc_id)?);
                    } else {
                        // if it was deleted subtract the delCount
                        self.base.pending_delete_count -= 1;
                        debug_assert!(self.assert_pending_deletes(info)?);
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            None => self.base.delete(doc_id),
        }
    }
    pub(crate) fn num_pending_deletes(&self) -> i32 {
        match self.field {
            Some(_) => self.base.num_pending_deletes() + self.hard_deletes.num_pending_deletes(),
            None => self.base.num_pending_deletes(),
        }
    }

    pub(crate) fn write_live_docs(
        &mut self,
        dir: Arc<Mutex<D>>,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<bool> {
        if self.field.is_none() {
            return self.base.write_live_docs(dir, info);
        }

        // we need to set this here to make sure our stats in SCI are up-to-date otherwise we might hit
        // an assertion
        // when the hard deletes are set since we need to account for docs that used to be only
        // soft-delete but now hard-deleted
        info.set_soft_del_count(info.get_soft_del_count() + self.base.pending_delete_count)?;
        self.base.drop_changes();
        // delegate the write to the hard deletes - it will only write if somebody used it.
        self.hard_deletes.write_live_docs(dir, info)
    }
    pub(crate) fn drop_changes(&mut self) {
        match self.field {
            Some(_) => {
                // don't reset anything here - this is called after a merge (successful or not) to prevent
                // rewriting the deleted docs to disk. we only pass it on and reset the number of pending
                // deletes
                self.hard_deletes.drop_changes();
            },
            None => {
                self.base.drop_changes();
            },
        }
    }

    fn assert_pending_deletes(&self, info: &mut SegmentCommitInfo<D>) -> Result<bool> {
        let sum = self.base.pending_delete_count + info.get_soft_del_count();
        debug_assert!(sum >= 0, "illegal pending delete count: {sum}");
        debug_assert!(info.info.max_doc()? >= self.base.get_del_count(info));
        Ok(true)
    }

    fn ensure_initialized<F>(&self, _reader_io_supplier: F)
    where
        F: Fn() -> Arc<SegmentReader<D>>,
    {
        todo!()
    }

    pub(crate) fn is_fully_deleted<F>(
        &self,
        _reader_io_supplier: F,
        info: &SegmentCommitInfo<D>,
    ) -> Result<bool>
    where
        F: Fn() -> Arc<SegmentReader<D>>,
    {
        if self.field.is_none() {
            return self.base.is_fully_deleted(_reader_io_supplier, info);
        }
        // initialize to ensure we have accurate counts - only needed in the soft-delete case
        self.ensure_initialized(_reader_io_supplier);
        todo!()
    }

    pub(crate) fn read_field_infos(&self, info: &SegmentCommitInfo<D>) -> Result<FieldInfos> {
        let seg_info = &info.info;
        let codec = get_default_code();
        if !info.has_field_updates() {
            // updates always outside of CFS
            if seg_info.get_use_compound_file() {
                let cfs = codec
                    .compound_format()
                    .get_compound_reader(&mut *seg_info.dir.lock(), seg_info)?;
                codec.field_infos_format().read(
                    &cfs,
                    seg_info,
                    "",
                    &IOContext::read_once_io_context()?,
                )
            } else {
                codec.field_infos_format().read(
                    &mut *seg_info.dir.lock(),
                    seg_info,
                    "",
                    &IOContext::read_once_io_context()?,
                )
            }
        } else {
            let segment_suffix = BigInt::from(info.get_field_infos_gen())
                .to_str_radix(36)
                .to_string();
            codec.field_infos_format().read(
                &mut *seg_info.dir.lock(),
                seg_info,
                &segment_suffix,
                &IOContext::read_once_io_context()?,
            )
        }
    }

    pub(crate) fn get_hard_live_docs(&mut self) -> Option<LiveDocsBits<D>> {
        match self.field {
            Some(_) => self.hard_deletes.get_live_docs(),
            None => self.base.get_hard_live_docs(),
        }
    }
    pub(crate) fn must_init_on_delete(&self) -> bool {
        match self.field {
            Some(_) => !self.base.live_docs_initialized,
            None => self.base.must_init_on_delete(),
        }
    }
}
impl<D> std::fmt::Display for PendingSoftDeletes<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(seg={:?} numPendingDeletes={} field={:?} dvGeneration={} hardDeletes={})",
            std::any::type_name::<Self>(),
            self.base.info_id,
            self.base.pending_delete_count,
            self.field,
            self.dv_generation,
            self.hard_deletes
        )
    }
}
pub(crate) mod pending_soft_deletes_util {
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::util::bits::Bits;
    use crate::util::error::lucene_error::Result;
    pub(crate) fn count_soft_deletes(
        soft_deleted_docs: Option<&mut impl DocIdSetIterator>,
        hard_deletes: Option<&impl Bits>,
    ) -> Result<i32> {
        let mut count = 0;
        if let Some(docs) = soft_deleted_docs {
            loop {
                let doc = docs.next_doc()?;
                if doc == NO_MORE_DOCS {
                    break;
                }
                if hard_deletes.is_none_or(|bits| bits.get(doc)) {
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}
