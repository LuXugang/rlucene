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
use crate::codecs::doc_values_consumer::DocValuesConsumer;
use crate::codecs::doc_values_format::DocValuesFormat;
use crate::codecs::doc_values_producer::DocValuesProducer;
use crate::codecs::dummy::dummy_binary_doc_values::DummyBinaryDocValues;
use crate::codecs::dummy::dummy_doc_values_skipper::DummyDocValuesSkipper;
use crate::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::codecs::dummy::dummy_sorted_doc_values::DummySortedDocValues;
use crate::codecs::dummy::dummy_sorted_numeric_doc_values::DummySortedNumericDocValues;
use crate::codecs::dummy::dummy_sorted_set_doc_values::DummySortedSetDocValues;
use crate::codecs::field_infos_format::FieldInfosFormat;
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::index::BytesRef;
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_field_updates::dvfu_util::merged_iterator;
use crate::index::doc_values_field_updates::{
    BinaryDocValuesDVFU, DocValuesFieldIterator, DocValuesFieldIteratorEnum,
    DocValuesFieldUpdatesEnum, MergedIterator, NumericDocValuesDVFU,
};
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::doc_values_type::DocValuesType;
use crate::index::field_info::FieldInfo;
use crate::index::field_infos::FieldInfos;
use crate::index::leaf_reader::LeafReader;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::pending_deletes::PendingDeletes;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_reader::SegmentReader;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMapImpl;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::search::doc_id_set_iterator::EitherDocIdSetIterator;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::store::IOContext;
use crate::store::directory::Directory;
use crate::store::flush_info::FlushInfo;
use crate::store::tracking_directory_wrapper::TrackingDirectoryWrapper;
use crate::util::bits::EitherBits;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fixed_bit_set::FixedBit;
use crate::util::function::Function;
use crate::util::info_stream::InfoStream;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

pub(crate) struct ReadersAndUpdates<L, LF>
where
    L: LeafReader,
    LF: LiveDocsFormat,
{
    // Tracks how many consumers are using this instance:
    ref_count: AtomicI32, // starts at 1
    // the major version this index was created with
    index_created_version_major: i32,
    // Only set if there are doc values updates against this segment, and the index is sorted:
    sort_map: Option<Rc<DocMapImpl>>,
    ram_bytes_used: AtomicI64,
    inner: Mutex<Inner<L, LF>>,
}

pub(crate) struct Inner<L, LF>
where
    L: LeafReader,
    LF: LiveDocsFormat,
{
    // Set once (None, and then maybe set, and never set again):
    reader: Option<SegmentReader<LF>>,
    // How many further deletions we've done against
    // liveDocs vs when we loaded it or last wrote it:
    pending_deletes: PendingDeletes<L>,
    // Indicates whether this segment is currently being merged. While a segment
    // is merging, all field updates are also registered in the
    // mergingDVUpdates map. Also, calls to writeFieldUpdates merge the
    // updates with mergingDVUpdates.
    // That way, when the segment is done merging, IndexWriter can apply the
    // updates on the merged segment too.
    is_merging: bool,
    // Holds resolved (to docIDs) doc values updates that have not yet been
    // written to the index
    pending_dv_updates: HashMap<String, Vec<Rc<DocValuesFieldUpdatesEnum>>>,
    // Holds resolved (to docIDs) doc values updates that were resolved while
    // this segment was being merged; at the end of the merge we carry over
    // these updates (remapping their docIDs) to the newly merged segment
    merging_dv_updates: HashMap<String, Vec<Rc<DocValuesFieldUpdatesEnum>>>,
}

impl<L, LF> ReadersAndUpdates<L, LF>
where
    L: LeafReader,
    LF: LiveDocsFormat,
{
    pub(crate) fn new(
        index_created_version_major: i32,
        pending_deletes: PendingDeletes<L>,
    ) -> Self {
        let inner = Mutex::new(Inner {
            reader: None,
            pending_deletes,
            is_merging: false,
            pending_dv_updates: HashMap::new(),
            merging_dv_updates: HashMap::new(),
        });
        Self {
            ref_count: AtomicI32::new(1),
            index_created_version_major,
            sort_map: None,
            ram_bytes_used: AtomicI64::new(0),
            inner,
        }
    }
    pub fn inc_ref(&self) {
        let rc = self.ref_count.fetch_add(1, Ordering::SeqCst) + 1;
        debug_assert!(rc > 1);
    }

    pub fn dec_ref(&self) {
        let rc = self.ref_count.fetch_sub(1, Ordering::SeqCst) - 1;
        debug_assert!(rc >= 0);
    }

    pub fn ref_count(&self) -> i32 {
        let rc = self.ref_count.load(Ordering::SeqCst);
        debug_assert!(rc >= 0);
        rc
    }
    pub(crate) fn get_del_count<D>(&self, info: &SegmentCommitInfo<D>) -> i32
    where
        D: Directory,
    {
        self.inner.lock().pending_deletes.get_del_count(info)
    }

    fn assert_no_dup_gen(
        &self,
        field_updates: &[Rc<DocValuesFieldUpdatesEnum>],
        update: &DocValuesFieldUpdatesEnum,
    ) -> bool {
        let dup = field_updates
            .iter()
            .any(|old_update| old_update.del_gen() == update.del_gen());
        debug_assert!(!dup, "duplicate delGen={}", update.del_gen());
        true
    }
    /// Adds a new resolved (meaning it maps docIDs to new values) doc values packet.
    /// We buffer these in RAM and write to disk when too much RAM is used or when a merge needs to kick off, or a commit/refresh.
    pub fn add_dv_update(&self, update: DocValuesFieldUpdatesEnum) -> Result<()> {
        let mut inner = self.inner.lock();
        if !update.get_finished()? {
            return Err(LuceneError::illegal_argument("call finish first"));
        }

        let field = update.field().to_string();
        let update_bytes = update.ram_bytes_used()?;

        let field_updates = inner.pending_dv_updates.entry(field.clone()).or_default();

        debug_assert!(self.assert_no_dup_gen(field_updates, &update));
        let update = Rc::new(update);
        self.ram_bytes_used
            .fetch_add(update_bytes, Ordering::Relaxed);

        field_updates.push(update.clone());

        if inner.is_merging {
            inner
                .merging_dv_updates
                .entry(field)
                .or_default()
                .push(update);
        }
        Ok(())
    }

    pub(crate) fn get_num_dv_updates(&self) -> i64 {
        let inner = self.inner.lock();
        inner
            .pending_dv_updates
            .values()
            .map(|v| v.len() as i64)
            .sum()
    }

    pub fn release<D>(&self, sr: &SegmentReader<LF>, info: &SegmentCommitInfo<D>) -> Result<()>
    where
        D: Directory,
    {
        // TODO
        Ok(())
    }

    pub fn delete(&self, doc_id: i32) -> Result<bool> {
        let mut inner = self.inner.lock();

        if inner.reader.is_none() && inner.pending_deletes.must_init_on_delete() {
            // TODO
        }

        inner.pending_deletes.delete(doc_id)
    }

    pub fn drop_readers(&self) -> Result<()> {
        let mut inner = self.inner.lock();

        if let Some(reader) = inner.reader.take() {
            // TODO
        }
        self.dec_ref();
        Ok(())
    }
    /// Returns a snapshot of the live docs.
    pub fn get_live_docs(&self) -> Option<EitherBits<Arc<L::Bits>, Arc<FixedBit>>> {
        let mut inner = self.inner.lock();
        inner.pending_deletes.get_live_docs()
    }

    /// Returns the live-docs bits excluding documents that are not live due to soft-deletes.
    pub fn get_hard_live_docs(&self) -> Option<EitherBits<Arc<L::Bits>, Arc<FixedBit>>> {
        let mut inner = self.inner.lock();
        inner.pending_deletes.get_hard_live_docs()
    }
    pub fn drop_changes(&self) {
        // Discard (don't save) changes when we are dropping
        // the reader; this is used only on the sub-readers
        // after a successful merge.  If deletes had
        // accumulated on those sub-readers while the merge
        // is running, by now we have carried forward those
        // deletes onto the newly merged segment, so we can
        // discard them on the sub-readers:
        let mut inner = self.inner.lock();
        inner.pending_deletes.drop_changes();
        self.drop_merging_updates(Some(&mut inner));
    }
    // Commit live docs (writes new _X_N.del files) and field updates (writes new
    // _X_N updates files) to the directory; returns true if it wrote any file
    // and false if there were no new deletes or updates to write:
    pub fn write_live_docs<D>(
        &self,
        dir: Arc<Mutex<D>>,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<bool>
    where
        D: Directory,
    {
        let mut inner = self.inner.lock();
        inner.pending_deletes.write_live_docs(dir, info)
    }

    pub fn handle_dv_updates<D, F>(
        &self,
        infos: &FieldInfos,
        dir: Arc<Mutex<D>>,
        dv_format: &F,
        reader: &mut SegmentReader<LF>,
        field_files: &mut HashMap<i32, HashSet<String>>,
        max_del_gen: i64,
        info_stream: &mut impl InfoStream,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<()>
    where
        D: Directory,
        F: DocValuesFormat,
    {
        let inner = self.inner.lock();

        for (field, updates) in inner.pending_dv_updates.iter() {
            let ty = updates[0].tp();
            debug_assert!(
                matches!(ty, DocValuesType::Numeric | DocValuesType::Binary),
                "unsupported type: {:?}",
                ty
            );

            let mut updates_to_apply = Vec::new();
            let mut bytes: i64 = 0;

            for update in updates {
                if update.del_gen() <= max_del_gen {
                    // safe to apply this one
                    bytes += update.ram_bytes_used()?;
                    updates_to_apply.push(update.clone());
                }
            }

            if updates_to_apply.is_empty() {
                // nothing to apply yet
                continue;
            }

            if info_stream.enabled("BD") {
                info_stream.message(
                    "BD",
                    &format!(
                        "now write {} pending numeric DV updates for field={}, seg={}, bytes={:.3} MB",
                        updates_to_apply.len(),
                        field,
                        info,
                        (bytes as f64) / 1024.0 / 1024.0
                    ),
                );
            }

            let next_doc_values_gen = info.get_next_doc_values_gen();
            let segment_suffix = num_bigint::BigInt::from(next_doc_values_gen)
                .to_str_radix(36)
                .to_string();
            let updates_context =
                IOContext::with_flush(FlushInfo::new(info.info.max_doc()?, bytes))?;

            let field_info = infos
                .field_info_by_name(field)
                .ok_or_else(|| LuceneError::illegal_argument("fieldInfo is None"))?;
            field_info.set_doc_values_gen(next_doc_values_gen)?;

            let field_infos = Rc::new(FieldInfos::new(vec![field_info.clone()])?);

            let mut tracking_dir = TrackingDirectoryWrapper::new(dir.clone());

            let mut state = SegmentWriteState::with_suffix(
                None,
                &mut tracking_dir,
                field_infos,
                &updates_context,
                &segment_suffix,
            );

            {
                let mut fields_consumer = dv_format.fields_consumer(&mut state, &info.info)?;

                let update_supplier = FunctionImpl::new(field_info.clone(), updates_to_apply);

                inner
                    .pending_deletes
                    .on_doc_values_update(&field_info, update_supplier.apply(&field_info)?);
                if *ty == DocValuesType::Binary {
                    let mut v = DocValuesProducerBinary::new(
                        update_supplier,
                        field,
                        reader,
                        field_info.clone(),
                    );
                    fields_consumer.add_binary_field(&field_info, &mut v)?
                } else {
                    let mut v = DocValuesProducerNumeric::new(
                        update_supplier,
                        field,
                        reader,
                        field_info.clone(),
                    );
                    fields_consumer.add_numeric_field(&field_info, &mut v)?;
                }

                drop(fields_consumer);
            }

            info.advance_doc_values_gen();
            debug_assert!(!field_files.contains_key(&field_info.number));
            field_files.insert(field_info.number, state.directory.take_created_files());
        }
        Ok(())
    }

    fn write_field_infos_gen<D, F>(
        &self,
        field_infos: &FieldInfos,
        dir: Arc<Mutex<D>>,
        infos_format: &F,
        info: &mut SegmentCommitInfo<D>,
    ) -> Result<HashSet<String>>
    where
        D: Directory,
        F: FieldInfosFormat,
    {
        let next_field_infos_gen = info.get_next_field_infos_gen();
        let segment_suffix = num_bigint::BigInt::from(next_field_infos_gen).to_str_radix(36);
        // we write approximately that many bytes (based on Lucene46DVF):
        // HEADER + FOOTER: 40
        // 90 bytes per-field (over estimating long name and attributes map)
        let est_infos_size = 40 + 90 * (field_infos.size() as i64);
        // IOContext for a flush with estimated size
        let flush_info = FlushInfo::new(info.info.max_doc()?, est_infos_size);
        let infos_context = IOContext::with_flush(flush_info)?;
        // separately also track which files were created for this gen
        let mut tracking_dir = TrackingDirectoryWrapper::new(dir);
        infos_format.write(
            &mut tracking_dir,
            &info.info,
            &segment_suffix,
            field_infos,
            &infos_context,
        )?;
        info.advance_field_infos_gen();
        Ok(tracking_dir.take_created_files())
    }

    /// Drops all merging updates.
    /// Called from IndexWriter after this segment finished merging (whether successfully or not).
    pub fn drop_merging_updates(&self, inner: Option<&mut Inner<L, LF>>) {
        let inner = match inner {
            Some(inner) => inner,
            None => &mut *self.inner.lock(),
        };
        inner.merging_dv_updates.clear();
        inner.is_merging = false;
    }
}
enum CurrentSource {
    OnDisk,
    Update,
}
/// This class merges the current on-disk DV with an incoming update DV instance and merges the two instances giving the incoming update precedence in terms of values,
/// in other words the values of the update always wins over the on-disk version.
struct MergedDocValues<DI>
where
    DI: DocValuesIterator,
{
    // merged docID
    doc_id_out: i32,
    // docID from our original doc values
    doc_id_on_disk: i32,
    // docID from our updates
    update_doc_id: i32,

    on_disk_doc_values: Option<DI>,
    update_doc_values: EitherDocIdSetIterator<
        BinaryDocValuesDVFU<MergedIterator<DocValuesFieldIteratorEnum>>,
        NumericDocValuesDVFU<MergedIterator<DocValuesFieldIteratorEnum>>,
    >,
    current_values_supplier: Option<CurrentSource>,
}
impl<DI> MergedDocValues<DI>
where
    DI: DocValuesIterator,
{
    pub fn new(
        on_disk_doc_values: Option<DI>,
        update_doc_values: EitherDocIdSetIterator<
            BinaryDocValuesDVFU<MergedIterator<DocValuesFieldIteratorEnum>>,
            NumericDocValuesDVFU<MergedIterator<DocValuesFieldIteratorEnum>>,
        >,
    ) -> Self {
        Self {
            doc_id_out: -1,
            doc_id_on_disk: -1,
            update_doc_id: -1,
            on_disk_doc_values,
            update_doc_values,
            current_values_supplier: None,
        }
    }
}
impl<DI> DocIdSetIterator for MergedDocValues<DI>
where
    DI: DocValuesIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc_id_out
    }

    fn next_doc(&mut self) -> Result<i32> {
        let mut has_value = false;

        while !has_value {
            if self.doc_id_on_disk == self.doc_id_out {
                match self.on_disk_doc_values.as_mut() {
                    Some(dv) => {
                        self.doc_id_on_disk = dv.next_doc()?;
                    },
                    None => {
                        self.doc_id_on_disk = NO_MORE_DOCS;
                    },
                }
            }

            if self.update_doc_id == self.doc_id_out {
                self.update_doc_id = self.update_doc_values.next_doc()?;
            }

            if self.doc_id_on_disk < self.update_doc_id {
                // no update to this doc - we use the on-disk values
                self.doc_id_out = self.doc_id_on_disk;
                self.current_values_supplier = Some(CurrentSource::OnDisk);
                has_value = true;
            } else {
                self.doc_id_out = self.update_doc_id;
                if self.doc_id_out != NO_MORE_DOCS {
                    self.current_values_supplier = Some(CurrentSource::Update);
                    has_value = match self.update_doc_values {
                        EitherDocIdSetIterator::F(ref mut dv) => dv.iterator.has_value(),
                        EitherDocIdSetIterator::S(ref mut dv) => dv.iterator.has_value(),
                    };
                } else {
                    has_value = true;
                }
            }
        }
        Ok(self.doc_id_out)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        self.on_disk_doc_values.as_ref().unwrap().cost()
    }
}

impl<DI> DocValuesIterator for MergedDocValues<DI>
where
    DI: DocValuesIterator,
{
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

struct BinaryDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    merged_doc_values: MergedDocValues<<SegmentReader<LF> as LeafReader>::BinaryDocValues>,
}
impl<LF> BinaryDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn new(
        merged_doc_values: MergedDocValues<<SegmentReader<LF> as LeafReader>::BinaryDocValues>,
    ) -> Self {
        Self { merged_doc_values }
    }
}

impl<LF> DocValuesIterator for BinaryDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.merged_doc_values.advance_exact(target)
    }
}

impl<LF> DocIdSetIterator for BinaryDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn doc_id(&self) -> i32 {
        self.merged_doc_values.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.merged_doc_values.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.merged_doc_values.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.merged_doc_values.cost()
    }
}

impl<LF> BinaryDocValues for BinaryDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        match self.merged_doc_values.current_values_supplier {
            Some(CurrentSource::OnDisk) => {
                if let Some(dv) = &mut self.merged_doc_values.on_disk_doc_values {
                    dv.binary_value()
                } else {
                    Err(LuceneError::illegal_state(
                        "no on-disk doc values available",
                    ))
                }
            },
            Some(CurrentSource::Update) => match self.merged_doc_values.update_doc_values {
                EitherDocIdSetIterator::F(ref mut dv) => dv.binary_value(),
                EitherDocIdSetIterator::S(_) => Err(LuceneError::illegal_state(
                    "update doc values should be BinaryDocValuesDVFU",
                )),
            },
            None => Err(LuceneError::illegal_state("no current values supplier set")),
        }
    }
}
struct NumericDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    merged_doc_values: MergedDocValues<<SegmentReader<LF> as LeafReader>::NumericDocValues>,
}

impl<LF> NumericDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn new(
        merged_doc_values: MergedDocValues<<SegmentReader<LF> as LeafReader>::NumericDocValues>,
    ) -> Self {
        Self { merged_doc_values }
    }
}

impl<LF> DocValuesIterator for NumericDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.merged_doc_values.advance_exact(target)
    }
}

impl<LF> DocIdSetIterator for NumericDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn doc_id(&self) -> i32 {
        self.merged_doc_values.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.merged_doc_values.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.merged_doc_values.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.merged_doc_values.cost()
    }
}

impl<LF> NumericDocValues for NumericDocValuesImpl<LF>
where
    LF: LiveDocsFormat,
{
    fn long_value(&mut self) -> Result<i64> {
        match self.merged_doc_values.current_values_supplier {
            Some(CurrentSource::OnDisk) => {
                if let Some(dv) = &mut self.merged_doc_values.on_disk_doc_values {
                    dv.long_value()
                } else {
                    Err(LuceneError::illegal_state(
                        "no on-disk doc values available",
                    ))
                }
            },
            Some(CurrentSource::Update) => match self.merged_doc_values.update_doc_values {
                EitherDocIdSetIterator::F(_) => Err(LuceneError::illegal_state(
                    "update doc values should be BinaryDocValuesDVFU",
                )),
                EitherDocIdSetIterator::S(ref mut dv) => dv.long_value(),
            },
            None => Err(LuceneError::illegal_state("no current values supplier set")),
        }
    }
}

struct DocValuesProducerBinary<'a, LF: LiveDocsFormat> {
    update_supplier: FunctionImpl,
    field: &'a str,
    reader: &'a mut SegmentReader<LF>,
    field_info: Arc<FieldInfo>,
}
impl<'a, LF: LiveDocsFormat> DocValuesProducerBinary<'a, LF> {
    pub fn new(
        update_supplier: FunctionImpl,
        field: &'a str,
        reader: &'a mut SegmentReader<LF>,
        field_info: Arc<FieldInfo>,
    ) -> Self {
        Self {
            update_supplier,
            field,
            reader,
            field_info,
        }
    }
}

impl<'a, LF: LiveDocsFormat> DocValuesProducer for DocValuesProducerBinary<'a, LF> {
    type NumericDocValues = DummyNumericDocValues;
    type BinaryDocValues = BinaryDocValuesImpl<LF>;

    fn get_binary(&self, _field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        let iterator = match self.update_supplier.apply(&self.field_info)? {
            Some(it) => it,
            None => {
                return Err(LuceneError::illegal_argument(
                    "iterator should never None here",
                ));
            },
        };
        let merged_doc_values = MergedDocValues::new(
            self.reader.get_binary_doc_values(self.field)?,
            EitherDocIdSetIterator::F(BinaryDocValuesDVFU::new(iterator)),
        );
        Ok(BinaryDocValuesImpl::new(merged_doc_values))
    }

    type SortedDocValues = DummySortedDocValues;
    type SortedNumericDocValues = DummySortedNumericDocValues;
    type SortedSetDocValues = DummySortedSetDocValues;
    type DocValuesSkipper = DummyDocValuesSkipper;
}
struct DocValuesProducerNumeric<'a, LF: LiveDocsFormat> {
    update_supplier: FunctionImpl,
    field: &'a str,
    reader: &'a mut SegmentReader<LF>,
    field_info: Arc<FieldInfo>,
}

impl<'a, LF: LiveDocsFormat> DocValuesProducerNumeric<'a, LF> {
    pub fn new(
        update_supplier: FunctionImpl,
        field: &'a str,
        reader: &'a mut SegmentReader<LF>,
        field_info: Arc<FieldInfo>,
    ) -> Self {
        Self {
            update_supplier,
            field,
            reader,
            field_info,
        }
    }
}

impl<'a, LF: LiveDocsFormat> DocValuesProducer for DocValuesProducerNumeric<'a, LF> {
    type NumericDocValues = NumericDocValuesImpl<LF>;
    fn get_numeric(&self, _field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        let iterator = match self.update_supplier.apply(&self.field_info)? {
            Some(it) => it,
            None => {
                return Err(LuceneError::illegal_argument(
                    "iterator should never None here",
                ));
            },
        };

        let merged_doc_values = MergedDocValues::new(
            self.reader.get_numeric_doc_values(self.field)?,
            EitherDocIdSetIterator::S(NumericDocValuesDVFU::new(iterator)),
        );
        // Merge sort of the original doc values with updated doc values:
        Ok(NumericDocValuesImpl::new(merged_doc_values))
    }
    type BinaryDocValues = DummyBinaryDocValues;
    type SortedDocValues = DummySortedDocValues;
    type SortedNumericDocValues = DummySortedNumericDocValues;
    type SortedSetDocValues = DummySortedSetDocValues;

    type DocValuesSkipper = DummyDocValuesSkipper;
}

struct FunctionImpl {
    field_info: Arc<FieldInfo>,
    updates_to_apply: Vec<Rc<DocValuesFieldUpdatesEnum>>,
}
impl FunctionImpl {
    fn new(
        field_info: Arc<FieldInfo>,
        updates_to_apply: Vec<Rc<DocValuesFieldUpdatesEnum>>,
    ) -> Self {
        Self {
            field_info,
            updates_to_apply,
        }
    }
}
impl Function<Arc<FieldInfo>, Option<MergedIterator<DocValuesFieldIteratorEnum>>> for FunctionImpl {
    fn apply(
        &self,
        info: &Arc<FieldInfo>,
    ) -> Result<Option<MergedIterator<DocValuesFieldIteratorEnum>>> {
        if !std::ptr::eq(info, &self.field_info) {
            return Err(LuceneError::illegal_argument(format!(
                "expected field info for field: {} but got: {}",
                self.field_info.name, info.name
            )));
        }

        let mut subs = vec![];
        for v in &self.updates_to_apply {
            subs.push(v.iterator()?)
        }
        merged_iterator(subs)
    }
}
