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
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::bytes_ref_hash::{BytesRefHash, BytesStartArrayEnum, DirectBytesStartArray};
use crate::util::error::lucene_error::LuceneError;
use crate::util::{AllocatorEnum, ByteBlockPool, Counter, CounterEnum, DirectTrackingAllocator};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
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
pub struct BufferedUpdates<Q>
where
    Q: Query + Eq + Hash,
{
    num_field_updates: AtomicI32,
    delete_terms: DeletedTerms,
    delete_queries: HashMap<Q, i32>,
    field_updates: HashMap<String, FieldUpdatesBuffer>,
    bytes_used: Arc<Mutex<CounterEnum>>,
    field_updates_bytes_used: Arc<Mutex<CounterEnum>>,
    verbose_deletes: bool,
    gen: i64,
    #[allow(unused)]
    segment_name: String,
}
impl BufferedUpdates<DummyQuery> {
    /// Rough logic: HashMap has an array with varying load factor.
    /// Entry consists of Query key, Integer value, int hash, and Entry next.
    // TODO: memory calculation not implemented
    pub const BYTES_PER_DEL_QUERY: i32 = 0;
    pub const MAX_INT: i32 = i32::MAX;
}

impl<Q> BufferedUpdates<Q>
where
    Q: Query + Eq + Hash,
{
    /// Creates a new `BufferedUpdates` instance.
    pub fn new(segment_name: String) -> Self {
        Self {
            num_field_updates: AtomicI32::new(0),
            delete_terms: DeletedTerms::new(),
            delete_queries: HashMap::new(),
            field_updates: HashMap::new(),
            bytes_used: Arc::new(Mutex::new(CounterEnum::new_counter(true))),
            field_updates_bytes_used: Arc::new(Mutex::new(CounterEnum::new_counter(true))),
            verbose_deletes: false,
            gen: 0,
            segment_name,
        }
    }
    pub fn add_query(&mut self, query: Q, doc_id_upto: i32) -> Result<(), LuceneError> {
        if self.delete_queries.insert(query, doc_id_upto).is_none() {
            self.bytes_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                .add_and_get(BYTES_PER_DEL_QUERY);
        }
        Ok(())
    }
    pub fn add_term(&mut self, term: &Term, doc_id_upto: i32) -> Result<(), LuceneError> {
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
        self.delete_terms.put(term, doc_id_upto)?;
        Ok(())
    }
    pub fn add_numeric_update(
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
    pub fn add_binary_update(
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
    pub fn clear_delete_terms(&mut self) -> Result<(), LuceneError> {
        self.delete_terms.clear()?;
        Ok(())
    }
    pub fn clear(&mut self) -> Result<(), LuceneError> {
        self.delete_terms.clear()?;
        self.delete_queries.clear();
        self.num_field_updates
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.field_updates.clear();

        {
            let mut bytes_used = self
                .bytes_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            let used = -bytes_used.get();
            bytes_used.add_and_get(used);
        }

        let mut field_updates_bytes_used = self
            .field_updates_bytes_used
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
        let used = -field_updates_bytes_used.get();
        field_updates_bytes_used.add_and_get(used);
        Ok(())
    }
    pub fn any(&self) -> bool {
        self.delete_terms.size() > 0
            || !self.delete_queries.is_empty()
            || self
                .num_field_updates
                .load(std::sync::atomic::Ordering::SeqCst)
                > 0
    }
}

impl<Q> Accountable for BufferedUpdates<Q>
where
    Q: Query + Eq + Hash,
{
    fn ram_bytes_used(&self) -> i64 {
        // TODO: memory calculation not implemented
        0
    }
}
impl<Q> fmt::Display for BufferedUpdates<Q>
where
    Q: Query + Eq + Hash + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.verbose_deletes {
            write!(
                f,
                "gen={} deleteTerms={} deleteQueries={} fieldUpdates={} bytesUsed={}",
                self.gen,
                self.delete_terms,
                self.delete_queries.len(),
                self.field_updates.len(),
                self.bytes_used.lock().map_err(|_| fmt::Error)?.get()
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
            let bytes_used = self.bytes_used.lock().map_err(|_| fmt::Error)?.get();
            if bytes_used != 0 {
                s.push_str(&format!(" bytesUsed={}", bytes_used));
            }
            write!(f, "{}", s)
        }
    }
}
pub struct DeletedTerms {
    bytes_used: Arc<Mutex<CounterEnum>>,
    pool: Arc<Mutex<ByteBlockPool>>,
    delete_terms: HashMap<String, BytesRefIntMap>,
    terms_size: i32,
}

impl Default for DeletedTerms {
    fn default() -> Self {
        Self::new()
    }
}

impl DeletedTerms {
    /// Creates a new instance of `DeletedTerms`.
    pub fn new() -> Self {
        let bytes_used = Arc::new(Mutex::new(CounterEnum::new_counter(false)));
        let pool = Arc::new(Mutex::new(ByteBlockPool::new(AllocatorEnum::DTA(
            DirectTrackingAllocator::new(bytes_used.clone()),
        ))));
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
    pub fn get(&self, term: &Term) -> Result<i32, LuceneError> {
        if let Some(hash) = self.delete_terms.get(&term.field) {
            Ok(hash.get(&term.bytes)?)
        } else {
            Ok(-1)
        }
    }
    /// Puts the newest document ID of the deleted term.
    ///
    /// Inserts the term and its corresponding document ID. If the term is new, increments the `terms_size`.
    pub fn put(&mut self, term: &Term, value: i32) -> Result<(), LuceneError> {
        let hash = self
            .delete_terms
            .entry(term.field.clone())
            .or_insert_with(|| {
                // TOOD: memory calculation not implemented
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))
                    .unwrap()
                    .add_and_get(0);
                BytesRefIntMap::new(self.pool.clone(), self.bytes_used.clone()).unwrap()
            });

        if hash.put(&term.bytes, value)? {
            self.terms_size += 1;
        }

        Ok(())
    }
    pub fn clear(&mut self) -> Result<(), LuceneError> {
        self.pool
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
            .reset(false, false)?;

        {
            let mut bytes_used = self
                .bytes_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            let used = -bytes_used.get();
            bytes_used.add_and_get(used);
        }

        self.delete_terms.clear();
        self.terms_size = 0;
        Ok(())
    }

    pub fn size(&self) -> i32 {
        self.terms_size
    }

    pub fn is_empty(&self) -> bool {
        self.terms_size == 0
    }
    /// Just for test, not efficient.
    #[cfg(feature = "test_only")]
    pub fn key_set(&self) -> Result<HashSet<Term>, LuceneError> {
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
    pub fn for_each_ordered<F>(&mut self, mut consumer: F) -> Result<(), LuceneError>
    where
        F: FnMut(&Term, i32) -> Result<(), LuceneError>,
    {
        let mut delete_fields: Vec<(&String, &mut BytesRefIntMap)> =
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
                consumer(&scratch, terms.values[index as usize])?;
            }
        }
        Ok(())
    }
    #[cfg(feature = "test_only")]
    pub fn get_pool(&self) -> Arc<Mutex<ByteBlockPool>> {
        self.pool.clone()
    }
}
pub trait DeletedTermConsumer {
    fn accept(&mut self, term: &Term, doc_id: i32) -> Result<(), LuceneError>;
}
impl Accountable for DeletedTerms {
    fn ram_bytes_used(&self) -> i64 {
        // TODO: memory calculation not implemented
        0
    }
}
impl fmt::Display for DeletedTerms {
    /// Used for `BufferedUpdates::VERBOSE_DELETES`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.key_set() {
            Ok(key_set) => {
                let entries: Vec<String> = key_set
                    .iter()
                    .filter_map(|term| {
                        self.get(term)
                            .ok()
                            .map(|value| format!("{}={}", term, value))
                    })
                    .collect();
                write!(f, "{{{}}}", entries.join(", "))
            }
            Err(_) => write!(f, "{{Failed to retrieving keys}}"),
        }
    }
}

pub struct BytesRefIntMap {
    counter: Arc<Mutex<CounterEnum>>,
    pub(crate) bytes_ref_hash: BytesRefHash,
    values: Vec<i32>,
}

impl BytesRefIntMap {
    // TODO: memory calculation not implemented
    const INIT_RAM_BYTES: i64 = 0;

    pub fn new(
        pool: Arc<Mutex<ByteBlockPool>>,
        counter: Arc<Mutex<CounterEnum>>,
    ) -> Result<Self, LuceneError> {
        let bytes_ref_hash = BytesRefHash::from_bytes_start_array(
            pool,
            BytesRefHash::DEFAULT_CAPACITY,
            Arc::new(Mutex::new(BytesStartArrayEnum::Direct(
                DirectBytesStartArray::with_counter(
                    BytesRefHash::DEFAULT_CAPACITY,
                    counter.clone(),
                ),
            ))),
        )?;
        let values = vec![0; BytesRefHash::DEFAULT_CAPACITY as usize];

        counter
            .lock()
            .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
            .add_and_get(Self::INIT_RAM_BYTES);

        Ok(Self {
            counter,
            bytes_ref_hash,
            values,
        })
    }
    pub fn key_set(&self) -> Result<HashSet<BytesRef>, LuceneError> {
        let mut scratch = BytesRef::new();
        let mut set = HashSet::new();

        for i in 0..self.bytes_ref_hash.size() {
            self.bytes_ref_hash.get(i, &mut scratch)?;
            set.insert(BytesRef::deep_copy_of(&scratch));
        }
        Ok(set)
    }
    pub fn put(&mut self, key: &BytesRef, value: i32) -> Result<bool, LuceneError> {
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
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(origin_length as i64);
            }
            self.values[e as usize] = value;
            Ok(true)
        }
    }
    pub fn get(&self, key: &BytesRef) -> Result<i32, LuceneError> {
        let e = self.bytes_ref_hash.find(key)?;
        if e == -1 {
            Ok(-1)
        } else {
            Ok(self.values[e as usize])
        }
    }
}
