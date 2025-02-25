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
use crate::index::doc_values_update::DocValuesUpdate;
use crate::index::field_updates_buffer::FieldUpdatesBuffer;
use crate::index::term::Term;
use crate::index::BytesRef;
use crate::search::dummy::dummy_query::DummyQuery;
use crate::search::query::Query;
use crate::util::access::Access;
use crate::util::accountable::Accountable;
use crate::util::allocator_byte::{AllocatorByteEnum, DirectTrackingAllocatorByte};
use crate::util::array_util::ArrayUtil;
use crate::util::bytes_ref_hash::{
    BytesRefHash, BytesStartArrayEnum, DirectBytesStartArray, MTBytesStartArrayEnum,
    STBytesStartArrayEnum,
};
use crate::util::error::lucene_error::LuceneError;
use crate::util::{
    ByteBlockPool, Counter, CounterEnum, MTByteBlockPool, MTCounterEnum, STByteBlockPool,
    STCounterEnum,
};
use std::cell::RefCell;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::AtomicI32;
use std::sync::{Arc, Mutex};

//TODO
#[allow(unused)]
const BYTES_PER_DEL_QUERY: i64 = 0;

/// Holds buffered deletes and updates, including deletions by docID, term, or query for a single segment.
///
/// This structure is used to manage buffered pending deletes and updates that apply to the
/// segment to be flushed. Once this deletes and updates are pushed (during a flush in
/// `DocumentsWriter`), they are converted into a `FrozenBufferedUpdates` instance and
/// forwarded to the `BufferedUpdatesStream`.
///
/// # Note
/// - Instances of this structure are accessed either via a private instance on `DocumentWriterPerThread`,
///   or through synchronized code in the `DocumentsWriterDeleteQueue`.
#[allow(dead_code)]
pub(crate) struct BufferedUpdates<Q, C, B, A>
where
    Q: Query,
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    pub(crate) num_field_updates: AtomicI32,
    pub delete_terms: DeletedTerms<C, B, A>,
    pub(crate) delete_queries: HashMap<Arc<Q>, i32>,
    pub(crate) field_updates: HashMap<String, FieldUpdatesBuffer>,
    bytes_used: C,
    field_updates_bytes_used: C,
    verbose_deletes: bool,
    gen: i64,
    #[allow(unused)]
    segment_name: String,
}
impl BufferedUpdates<DummyQuery, MTCounterEnum, MTByteBlockPool, MTBytesStartArrayEnum> {
    /// Rough logic: HashMap has an array with varying load factor.
    /// Entry consists of Query key, Integer value, int hash, and Entry next.
    // TODO: memory calculation not implemented
    pub const BYTES_PER_DEL_QUERY: i32 = 0;
    pub const MAX_INT: i32 = i32::MAX;
}
#[allow(unused)]
impl<Q> BufferedUpdates<Q, MTCounterEnum, MTByteBlockPool, MTBytesStartArrayEnum>
where
    Q: Query,
{
    /// Creates a new `BufferedUpdates` instance.
    pub(crate) fn new_sync(segment_name: String) -> Self {
        Self {
            num_field_updates: AtomicI32::new(0),
            delete_terms: DeletedTerms::new_sync(),
            delete_queries: HashMap::new(),
            field_updates: HashMap::new(),
            bytes_used: Arc::new(Mutex::new(CounterEnum::new_counter(true))),
            field_updates_bytes_used: Arc::new(Mutex::new(CounterEnum::new_counter(true))),
            verbose_deletes: false,
            gen: 0,
            segment_name,
        }
    }
    pub(crate) fn add_binary_update(
        &mut self,
        update: &DocValuesUpdate,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        let buffer = match self.field_updates.entry(update.field.clone()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let new_buffer = FieldUpdatesBuffer::from_binary_update(
                    self.field_updates_bytes_used.clone(),
                    update,
                    doc_id_upto,
                )?;
                entry.insert(new_buffer)
            }
        };

        if update.has_value {
            let binary_update = update.sub_update.get_binary();
            debug_assert!(binary_update.is_some());
            buffer.add_update_with_bytes_ref(
                &update.term,
                &binary_update.unwrap().get_value(),
                doc_id_upto,
            )?;
        } else {
            buffer.add_no_value(&update.term, doc_id_upto)?;
        }

        self.num_field_updates
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    pub(crate) fn add_numeric_update(
        &mut self,
        update: &DocValuesUpdate,
        doc_id_upto: i32,
    ) -> Result<(), LuceneError> {
        let buffer = match self.field_updates.entry(update.field.clone()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let new_buffer = FieldUpdatesBuffer::from_numeric_update(
                    self.field_updates_bytes_used.clone(),
                    update,
                    doc_id_upto,
                )?;
                entry.insert(new_buffer)
            }
        };

        if update.has_value {
            let numeric_update = update.sub_update.get_numeric();
            debug_assert!(numeric_update.is_some());
            buffer.add_update_with_long(
                &update.term,
                numeric_update.unwrap().get_value(),
                doc_id_upto,
            )?;
        } else {
            buffer.add_no_value(&update.term, doc_id_upto)?;
        }

        self.num_field_updates
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    pub(crate) fn add_term(&mut self, term: &Term, doc_id_upto: i32) -> Result<(), LuceneError> {
        let current = self.delete_terms.get(term)?;
        if current != -1 && doc_id_upto < current {
            // Only record the new number if it's greater than the
            // current one.
            // This is important because if multiple
            // threads are replacing the same doc at nearly the
            // same time, it's possible that one thread that got a
            // higher docID is scheduled before the other
            // threads.
            // If we blindly replace than we can
            // incorrectly get both docs indexed.
            return Ok(());
        }
        self.delete_terms.put_sync(term, doc_id_upto)?;
        Ok(())
    }
}
impl<Q> BufferedUpdates<Q, STCounterEnum, STByteBlockPool, STBytesStartArrayEnum>
where
    Q: Query,
{
    /// Creates a new `BufferedUpdates` instance.
    pub(crate) fn new(segment_name: String) -> Self {
        Self {
            num_field_updates: AtomicI32::new(0),
            delete_terms: DeletedTerms::new(),
            delete_queries: HashMap::new(),
            field_updates: HashMap::new(),
            bytes_used: Rc::new(RefCell::new(CounterEnum::new_counter(true))),
            field_updates_bytes_used: Rc::new(RefCell::new(CounterEnum::new_counter(true))),
            verbose_deletes: false,
            gen: 0,
            segment_name,
        }
    }
}

#[allow(unused)]
impl<Q, C, B, A> BufferedUpdates<Q, C, B, A>
where
    Q: Query,
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    pub(crate) fn add_query(&mut self, query: Arc<Q>, doc_id_upto: i32) -> Result<(), LuceneError> {
        if self
            .delete_queries
            .insert(query.clone(), doc_id_upto)
            .is_none()
        {
            let mut bytes_used_guard = self
                .bytes_used
                .with_ref_mut(|bytes_used| Ok(bytes_used.add_and_get(BYTES_PER_DEL_QUERY)))?;
        }
        Ok(())
    }
    pub(crate) fn clear_delete_terms(&mut self) -> Result<(), LuceneError> {
        self.delete_terms.clear()
    }
    pub(crate) fn clear(&mut self) -> Result<(), LuceneError> {
        self.delete_terms.clear()?;
        self.delete_queries.clear();
        self.num_field_updates
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.field_updates.clear();

        self.bytes_used.with_ref_mut(|bytes_used| {
            let used = -bytes_used.get();
            Ok(bytes_used.add_and_get(used))
        })?;

        self.field_updates_bytes_used
            .with_ref_mut(|field_updates_bytes_used| {
                let used = -field_updates_bytes_used.get();
                field_updates_bytes_used.add_and_get(used);
                Ok(())
            })
    }
    pub(crate) fn any(&self) -> bool {
        self.delete_terms.size() > 0
            || !self.delete_queries.is_empty()
            || self
                .num_field_updates
                .load(std::sync::atomic::Ordering::SeqCst)
                > 0
    }
}

impl<Q, C, B, A> Accountable for BufferedUpdates<Q, C, B, A>
where
    Q: Query,
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    fn ram_bytes_used(&self) -> i64 {
        // TODO: memory calculation not implemented
        0
    }
}
impl<Q, C, B, A> fmt::Display for BufferedUpdates<Q, C, B, A>
where
    Q: Query + fmt::Display,
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes_used = self.bytes_used.with_ref(|bytes_used| Ok(bytes_used.get()));
        if bytes_used.is_err() {
            return write!(
                f,
                "Failed to acquire lock for bytes_used, can not call to_string()."
            );
        }
        if self.verbose_deletes {
            write!(
                f,
                "gen={} deleteTerms={} deleteQueries={} fieldUpdates={} bytesUsed={}",
                self.gen,
                self.delete_terms,
                self.delete_queries.len(),
                self.field_updates.len(),
                bytes_used.unwrap()
            )
        } else {
            let mut s = format!("gen={}", self.gen);
            if !self.delete_terms.is_empty() {
                s.push_str(&format!(
                    " {} unique deleted terms",
                    self.delete_terms.size()
                ));
            }
            if !self.delete_queries.is_empty() {
                s.push_str(&format!(" {} deleted queries", self.delete_queries.len()));
            }
            if self
                .num_field_updates
                .load(std::sync::atomic::Ordering::SeqCst)
                != 0
            {
                s.push_str(&format!(
                    " {} field updates",
                    self.num_field_updates
                        .load(std::sync::atomic::Ordering::SeqCst)
                ));
            }
            let bytes_used = bytes_used.unwrap();
            if bytes_used != 0 {
                s.push_str(&format!(" bytesUsed={}", bytes_used));
            }
            write!(f, "{}", s)
        }
    }
}
pub(crate) struct DeletedTerms<C, B, A>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    bytes_used: C,
    pool: B,
    delete_terms: HashMap<String, BytesRefIntMap<C, B, A>>,
    terms_size: i32,
}

impl DeletedTerms<MTCounterEnum, MTByteBlockPool, MTBytesStartArrayEnum> {
    pub(crate) fn new_sync() -> Self {
        let bytes_used = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
        let allocator =
            AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(bytes_used.clone()));
        let pool = Arc::new(Mutex::new(ByteBlockPool::new_sync(allocator)));
        Self::new_impl(pool, bytes_used)
    }
    /// Puts the newest document ID of the deleted term.
    ///
    /// Inserts the term and its corresponding document ID. If the term is new, increments the `terms_size`.
    pub(crate) fn put_sync(&mut self, term: &Term, value: i32) -> Result<(), LuceneError> {
        let hash = self
            .bytes_used
            .with_ref_mut(|bytes_used| -> Result<_, LuceneError> {
                match self.delete_terms.entry(term.field.clone()) {
                    Vacant(vacant) => {
                        // TODO: memory calculation not implemented
                        bytes_used.add_and_get(0);
                        let new_map =
                            BytesRefIntMap::new_sync(self.pool.clone(), self.bytes_used.clone())?;
                        Ok(vacant.insert(new_map))
                    }
                    Occupied(occupied) => Ok(occupied.into_mut()),
                }
            })?;
        if hash.put(&term.bytes, value)? {
            self.terms_size += 1;
        }
        Ok(())
    }
}
impl DeletedTerms<STCounterEnum, STByteBlockPool, STBytesStartArrayEnum> {
    pub(crate) fn new() -> Self {
        let bytes_used = Rc::new(RefCell::new(CounterEnum::new_counter(false)));
        let allocator =
            AllocatorByteEnum::DTA(DirectTrackingAllocatorByte::new(bytes_used.clone()));
        let pool = Rc::new(RefCell::new(ByteBlockPool::new(allocator)));
        Self::new_impl(pool, bytes_used)
    }
    /// Puts the newest document ID of the deleted term.
    ///
    /// Inserts the term and its corresponding document ID. If the term is new, increments the `terms_size`.
    #[allow(unused)]
    pub(crate) fn put(&mut self, term: &Term, value: i32) -> Result<(), LuceneError> {
        let hash = self
            .bytes_used
            .with_ref_mut(|bytes_used| -> Result<_, LuceneError> {
                match self.delete_terms.entry(term.field.clone()) {
                    Vacant(vacant) => {
                        // TODO: memory calculation not implemented
                        bytes_used.add_and_get(0);
                        let new_map =
                            BytesRefIntMap::new(self.pool.clone(), self.bytes_used.clone())?;
                        Ok(vacant.insert(new_map))
                    }
                    Occupied(occupied) => Ok(occupied.into_mut()),
                }
            })?;
        if hash.put(&term.bytes, value)? {
            self.terms_size += 1;
        }
        Ok(())
    }
}

#[allow(unused)]
impl<C, B, A> DeletedTerms<C, B, A>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    /// Creates a new instance of `DeletedTerms`.
    fn new_impl(pool: B, bytes_used: C) -> Self {
        Self {
            bytes_used,
            pool,
            delete_terms: HashMap::new(),
            terms_size: 0,
        }
    }
    /// Gets the newest document ID of the deleted term.
    ///
    /// Returns the newest document ID if the term exists, otherwise returns `-1`.
    pub(crate) fn get(&self, term: &Term) -> Result<i32, LuceneError> {
        if let Some(hash) = self.delete_terms.get(&term.field) {
            hash.get(&term.bytes)
        } else {
            Ok(-1)
        }
    }
    pub(crate) fn clear(&mut self) -> Result<(), LuceneError> {
        let mut pool = self.pool.with_ref_mut(|p| Ok(p.reset(false, false)))?;

        self.bytes_used.with_ref_mut(|bytes_used| {
            let used = -bytes_used.get();
            let _ = bytes_used.add_and_get(used);
            Ok(())
        })?;
        self.delete_terms.clear();
        self.terms_size = 0;
        Ok(())
    }

    pub(crate) fn size(&self) -> i32 {
        self.terms_size
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.terms_size == 0
    }
    /// Just for test, not efficient.
    pub(crate) fn key_set(&self) -> Result<HashSet<Term>, LuceneError> {
        let mut set = HashSet::new();
        for (field, hash) in &self.delete_terms {
            for bytes in hash.key_set()? {
                set.insert(Term::new(field.clone(), bytes));
            }
        }
        Ok(set)
    }

    /// Consume all terms in a sorted order.
    ///
    /// Note: This is a destructive operation as it calls `BytesRefHash::sort()`.
    pub(crate) fn for_each_ordered<F>(&mut self, mut consumer: F) -> Result<(), LuceneError>
    where
        F: FnMut(&Term, i32) -> Result<(), LuceneError>,
    {
        let mut delete_fields: Vec<(&String, &mut BytesRefIntMap<C, B, A>)> =
            self.delete_terms.iter_mut().collect();
        delete_fields.sort_by(|a, b| a.0.cmp(b.0));

        let mut scratch = Term::new("".to_string(), BytesRef::new());
        for (field, terms) in delete_fields {
            scratch.field = field.clone();
            terms.bytes_ref_hash.sort()?;
            let indices = &terms.bytes_ref_hash.ids;
            for i in 0..terms.bytes_ref_hash.count {
                let index = indices[i as usize];
                terms.bytes_ref_hash.get(index, &mut scratch.bytes)?;
                consumer(&scratch, terms.values[index as usize]);
            }
        }
        Ok(())
    }
    #[cfg(feature = "test_only")]
    pub(crate) fn get_pool(&self) -> B {
        self.pool.clone()
    }
}
#[allow(unused)]
pub trait DeletedTermConsumer {
    fn accept(&mut self, term: &Term, doc_id: i32) -> Result<(), LuceneError>;
}
impl<C, B, A> Accountable for DeletedTerms<C, B, A>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    fn ram_bytes_used(&self) -> i64 {
        // TODO: memory calculation not implemented
        0
    }
}
impl<C, B, A> fmt::Display for DeletedTerms<C, B, A>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    /// Used for `BufferedUpdates::VERBOSE_DELETES`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_set = match self.key_set() {
            Ok(ks) => ks,
            Err(e) => return write!(f, "Error retrieving key_set: {}", e),
        };
        let mut entries = Vec::new();
        for term in key_set.iter() {
            let value = match self.get(term) {
                Ok(v) => v,
                Err(e) => return write!(f, "Error retrieving value for {}: {}", term, e),
            };
            entries.push(format!("{}={}", term, value));
        }

        write!(f, "{{{}}}", entries.join(", "))
    }
}

#[allow(unused)]
struct BytesRefIntMap<C, B, A>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    counter: C,
    pub(crate) bytes_ref_hash: BytesRefHash<C, B, A>,
    values: Vec<i32>,
}
impl BytesRefIntMap<MTCounterEnum, MTByteBlockPool, MTBytesStartArrayEnum> {
    // TODO: memory calculation not implemented
    const INIT_RAM_BYTES: i64 = 0;
}

impl BytesRefIntMap<MTCounterEnum, MTByteBlockPool, MTBytesStartArrayEnum> {
    fn new_sync(pool: MTByteBlockPool, counter: MTCounterEnum) -> Result<Self, LuceneError> {
        let bytes_ref_hash = BytesRefHash::from_bytes_start_array(
            pool,
            BytesRefHash::DEFAULT_CAPACITY,
            Arc::new(Mutex::new(BytesStartArrayEnum::Direct(
                DirectBytesStartArray::with_counter_sync(
                    BytesRefHash::DEFAULT_CAPACITY,
                    counter.clone(),
                ),
            ))),
        )?;
        Self::new_impl(counter, bytes_ref_hash)
    }
}
impl BytesRefIntMap<STCounterEnum, STByteBlockPool, STBytesStartArrayEnum> {
    fn new(pool: STByteBlockPool, counter: STCounterEnum) -> Result<Self, LuceneError> {
        let bytes_ref_hash = BytesRefHash::from_bytes_start_array(
            pool,
            BytesRefHash::DEFAULT_CAPACITY,
            Rc::new(RefCell::new(BytesStartArrayEnum::Direct(
                DirectBytesStartArray::with_counter(
                    BytesRefHash::DEFAULT_CAPACITY,
                    counter.clone(),
                ),
            ))),
        )?;
        Self::new_impl(counter, bytes_ref_hash)
    }
}

#[allow(unused)]
impl<C, B, A> BytesRefIntMap<C, B, A>
where
    C: Access<CounterEnum>,
    B: Access<ByteBlockPool<C>>,
    A: Access<BytesStartArrayEnum<C>>,
{
    fn new_impl(counter: C, bytes_ref_hash: BytesRefHash<C, B, A>) -> Result<Self, LuceneError> {
        let values = vec![0; BytesRefHash::DEFAULT_CAPACITY as usize];

        counter.with_ref_mut(|c| Ok(c.add_and_get(BytesRefIntMap::INIT_RAM_BYTES)))?;

        Ok(Self {
            counter,
            bytes_ref_hash,
            values,
        })
    }
    fn key_set(&self) -> Result<HashSet<BytesRef>, LuceneError> {
        let mut scratch = BytesRef::new();
        let mut set = HashSet::new();

        for i in 0..self.bytes_ref_hash.size() {
            self.bytes_ref_hash.get(i, &mut scratch)?;
            set.insert(BytesRef::deep_copy_of(&scratch));
        }
        Ok(set)
    }
    fn put(&mut self, key: &BytesRef, value: i32) -> Result<bool, LuceneError> {
        debug_assert!(value >= 0, "Value must be non-negative.");
        let e = self.bytes_ref_hash.add(key)?;
        if e < 0 {
            self.values[(-e - 1) as usize] = value;
            Ok(false)
        } else {
            if e as usize >= self.values.len() {
                let origin_length = self.values.len();
                ArrayUtil::grow_with_len(&mut self.values, e + 1)?;
                // TODO: memory calculation not implemented
                self.counter
                    .with_ref_mut(|c| Ok(c.add_and_get(origin_length as i64)))?;
            }
            self.values[e as usize] = value;
            Ok(true)
        }
    }
    fn get(&self, key: &BytesRef) -> Result<i32, LuceneError> {
        let e = self.bytes_ref_hash.find(key)?;
        if e == -1 {
            Ok(-1)
        } else {
            Ok(self.values[e as usize])
        }
    }
}
/// for single-threaded scenarios
pub type STBytesRefIntMap = BytesRefIntMap<STCounterEnum, STByteBlockPool, STBytesStartArrayEnum>;
/// for multi-threaded scenarios
pub type MTBytesRefIntMap = BytesRefIntMap<MTCounterEnum, MTByteBlockPool, MTBytesStartArrayEnum>;

#[cfg(test)]
mod tests {
    use crate::index::buffered_updates::{BufferedUpdates, DeletedTerms};
    use crate::index::term::Term;
    use crate::index::BytesRef;
    use crate::search::term_query::TermQuery;
    use crate::test::util::lucene_test_case::{at_least, random};

    use crate::util::accountable::Accountable;
    use crate::util::error::lucene_error::LuceneError;
    use rand::{Rng, RngCore};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    pub struct TestBufferedUpdates;

    #[test]
    fn test_ram_bytes_used() -> Result<(), LuceneError> {
        let mut random = random();
        let mut bu = BufferedUpdates::new_sync("seg1".to_string());

        // TODO
        // assert_eq!(bu.ram_bytes_used(), 0);
        assert!(!bu.any());

        let queries = at_least(&mut random, 1);
        for _ in 0..queries {
            let doc_id_upto = if random.random_bool(0.5) {
                i32::MAX
            } else {
                random.random_range(0..100000)
            };
            let value = format!("{}", random.random_range(0..100));
            let term = Term::new("id".to_string(), BytesRef::from_string(&value));
            bu.add_query(Arc::new(TermQuery::new(term.clone())), doc_id_upto)?;
        }

        let terms = at_least(&mut random, 1);
        for _ in 0..terms {
            let doc_id_upto = if random.random_bool(0.5) {
                i32::MAX
            } else {
                random.random_range(0..100000)
            };
            let value = format!("{}", random.random_range(0..100));
            let term = Term::new("id".to_string(), BytesRef::from_string(&value));
            bu.add_term(&term, doc_id_upto)?;
        }

        assert!(
            bu.any(),
            "We have added a lot of docIds, terms, and queries, but `any()` returned false."
        );

        // TODO
        // let total_used = bu.ram_bytes_used();
        // assert!(total_used > 0);

        bu.clear_delete_terms()?;
        assert!(
            bu.any(),
            "Only terms and docIds are cleaned, the queries should still be in memory."
        );
        // TODO
        // assert!(
        //     total_used > bu.ram_bytes_used(),
        //     "Terms are cleaned, so memory usage should decrease."
        // );

        bu.clear()?;
        assert!(!bu.any());
        // TODO
        // assert_eq!(bu.ram_bytes_used()?, 0);

        Ok(())
    }
    #[test]
    fn test_deleted_terms() -> Result<(), LuceneError> {
        let mut random = random();
        let iters = at_least(&mut random, 10);
        let fields = ["a".to_string(), "b".to_string(), "c".to_string()];
        let mut actual = DeletedTerms::new();

        for _ in 0..iters {
            let mut expected = HashMap::new();
            assert!(actual.is_empty());

            let term_count = at_least(&mut random, 5000);
            let max_bytes_num = random.random_range(1..=3);

            for _ in 0..term_count {
                let byte_num = random.random_range(1..=max_bytes_num);
                let mut bytes = vec![0u8; byte_num];
                random.fill_bytes(&mut bytes);

                let field = &fields[random.random_range(0..fields.len())];
                let term = Term::new(field.clone(), BytesRef::from_bytes(bytes));
                let value = random.random_range(0..10_000_000);

                expected.insert(term.clone(), value);
                actual.put(&term, value)?;
            }

            assert_eq!(expected.len(), actual.size() as usize);

            for (term, expected_value) in &expected {
                assert_eq!(*expected_value, actual.get(term)?);
            }

            let mut expected_sorted: Vec<(Term, i32)> = expected
                .iter()
                .map(|(term, doc_id)| (Term::new(term.field.clone(), term.bytes.clone()), *doc_id))
                .collect();
            expected_sorted.sort_by_key(|entry| entry.0.clone());

            let mut actual_sorted: Vec<_> = Vec::new();
            actual.for_each_ordered(|term, doc_id| {
                let copy = Term::new(term.field.clone(), term.bytes.clone());
                actual_sorted.push((copy, doc_id));
                Ok(())
            })?;

            assert_eq!(expected_sorted, actual_sorted);

            actual.clear()?;
            assert_eq!(actual.size(), 0);
            assert_eq!(actual.ram_bytes_used(), 0);
            let pool_guard = actual.get_pool();
            let pool = pool_guard.borrow();
            assert_eq!(pool.buffer_upto, -1);
        }

        Ok(())
    }
}
