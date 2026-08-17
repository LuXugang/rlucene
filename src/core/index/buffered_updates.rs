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
use crate::core::index::BytesRef;
use crate::core::index::doc_values_update::DocValuesUpdate;
use crate::core::index::field_updates_buffer::FieldUpdatesBuffer;
use crate::core::index::term::Term;
use crate::core::search::query::Query;
use crate::core::util::accountable::Accountable;
use crate::core::util::allocator_byte::DirectTrackingAllocatorByte;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bytes_ref_hash::DEFAULT_CAPACITY;
use crate::core::util::bytes_ref_hash::{BytesRefHash, DirectBytesStartArray};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ram_usage_estimator::{size_of_hash_map, size_of_string, size_of_vec};
use crate::core::util::{AtomicCounter, ByteBlockPool, Counter, SharedCounter};
#[cfg(test)]
use parking_lot::Mutex;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicI32;

/// Holds buffered deletes and updates, including deletions by docID, term, or
/// query for a single segment.
///
/// This structure is used to manage buffered pending deletes and updates that
/// apply to the segment to be flushed. Once this deletes and updates are pushed
/// (during a flush in `DocumentsWriter`), they are converted into a
/// `FrozenBufferedUpdates` instance and forwarded to the
/// `BufferedUpdatesStream`.
///
/// # Note
/// - Instances of this structure are accessed either via a private instance on
///   `DocumentWriterPerThread`, or while holding the `DocumentsWriterDeleteQueue` lock.
pub(crate) struct BufferedUpdates {
  pub(crate) num_field_updates: AtomicI32,
  pub delete_terms: DeletedTerms,
  pub(crate) delete_queries: HashMap<Query, i32>,
  pub(crate) field_updates: HashMap<String, FieldUpdatesBuffer>,
  bytes_used: SharedCounter,
  pub(crate) field_updates_bytes_used: SharedCounter,
  verbose_deletes: bool,
  gen_: i64,

  segment_name: String,
}

impl BufferedUpdates {
  /// Creates a new `BufferedUpdates` instance.
  pub(crate) fn new(segment_name: &str) -> Self {
    Self {
      num_field_updates: AtomicI32::new(0),
      delete_terms: DeletedTerms::new(),
      delete_queries: HashMap::new(),
      field_updates: HashMap::new(),
      bytes_used: Arc::new(AtomicCounter::new()),
      field_updates_bytes_used: Arc::new(AtomicCounter::new()),
      verbose_deletes: false,
      gen_: 0,
      segment_name: segment_name.to_string(),
    }
  }
  pub(crate) fn add_binary_update(
    &mut self,
    update: &DocValuesUpdate,
    doc_id_upto: i32,
  ) -> Result<()> {
    let buffer = match self.field_updates.entry(update.field.clone()) {
      Occupied(entry) => entry.into_mut(),
      Vacant(entry) => {
        let new_buffer = FieldUpdatesBuffer::from_binary_update(
          self.field_updates_bytes_used.clone(),
          update,
          doc_id_upto,
        )?;
        entry.insert(new_buffer)
      },
    };

    if update.has_value {
      let binary_update = update.sub_update.get_binary();
      debug_assert!(binary_update.is_some());
      buffer.add_update_with_bytes_ref(
        &update.term,
        binary_update.unwrap().get_value(),
        doc_id_upto,
      )?;
    } else {
      buffer.add_no_value(&update.term, doc_id_upto)?;
    }

    self
      .num_field_updates
      .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
  }
  pub(crate) fn add_numeric_update(
    &mut self,
    update: &DocValuesUpdate,
    doc_id_upto: i32,
  ) -> Result<()> {
    let buffer = match self.field_updates.entry(update.field.clone()) {
      Occupied(entry) => entry.into_mut(),
      Vacant(entry) => {
        let new_buffer = FieldUpdatesBuffer::from_numeric_update(
          self.field_updates_bytes_used.clone(),
          update,
          doc_id_upto,
        )?;
        entry.insert(new_buffer)
      },
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

    self
      .num_field_updates
      .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
  }
  pub(crate) fn add_term(&mut self, term: &Term, doc_id_upto: i32) -> Result<()> {
    let current = self.delete_terms.get(term);
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
    self.delete_terms.put(term, doc_id_upto)
  }
  pub(crate) fn add_query(&mut self, query: Query, doc_id_upto: i32) -> Result<()> {
    let old_size = size_of_hash_map(&self.delete_queries);
    let query_ram_bytes_used = query.ram_bytes_used()?;
    if self.delete_queries.insert(query, doc_id_upto).is_none() {
      self.bytes_used.add_and_get(
        size_of_hash_map(&self.delete_queries)
          .saturating_sub(old_size)
          .saturating_add(query_ram_bytes_used),
      );
    }
    Ok(())
  }
  pub(crate) fn clear_delete_terms(&mut self) {
    self.delete_terms.clear()
  }
  pub(crate) fn clear(&mut self) {
    self.delete_terms.clear();
    self.delete_queries = HashMap::new();
    self
      .num_field_updates
      .store(0, std::sync::atomic::Ordering::SeqCst);
    self.field_updates.clear();

    let used = -self.bytes_used.get();
    self.bytes_used.add_and_get(used);

    let used = -self.field_updates_bytes_used.get();
    self.field_updates_bytes_used.add_and_get(used);
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

impl Accountable for BufferedUpdates {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(
      self
        .bytes_used
        .get()
        .wrapping_add(self.field_updates_bytes_used.get())
        .wrapping_add(self.delete_terms.ram_bytes_used()?),
    )
  }
}
impl fmt::Display for BufferedUpdates {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let bytes_used = self.bytes_used.get();
    if self.verbose_deletes {
      write!(
        f,
        "gen={} deleteTerms={} deleteQueries={} fieldUpdates={} bytesUsed={}",
        self.gen_,
        self.delete_terms,
        self.delete_queries.len(),
        self.field_updates.len(),
        bytes_used
      )
    } else {
      let mut s = format!("gen={}", self.gen_);
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
          self
            .num_field_updates
            .load(std::sync::atomic::Ordering::SeqCst)
        ));
      }
      if bytes_used != 0 {
        s.push_str(&format!(" bytesUsed={bytes_used}"));
      }
      write!(f, "{s}")
    }
  }
}
#[cfg(test)]
pub type BufferedUpdatesLock = Arc<Mutex<BufferedUpdates>>;

pub(crate) struct DeletedTerms {
  bytes_used: SharedCounter,
  pool: ByteBlockPool,
  delete_terms: HashMap<String, BytesRefIntMap>,
  terms_size: i32,
}
impl DeletedTerms {
  pub(crate) fn new() -> Self {
    let bytes_used = Arc::new(AtomicCounter::new());
    let allocator = DirectTrackingAllocatorByte::new(bytes_used.clone());
    let pool = ByteBlockPool::new(allocator);
    Self::new_impl(pool, bytes_used)
  }
  pub(crate) fn put(&mut self, term: &Term, value: i32) -> Result<()> {
    let hash = match self.delete_terms.entry(term.field.clone()) {
      Vacant(vacant) => {
        self.bytes_used.add_and_get(size_of_string(vacant.key()));
        let new_map = BytesRefIntMap::new(self.bytes_used.clone())?;
        vacant.insert(new_map)
      },
      Occupied(occupied) => occupied.into_mut(),
    };
    if hash.put(&term.bytes, value, &mut self.pool)? {
      self.terms_size += 1;
    }
    Ok(())
  }
  /// Creates a new instance of `DeletedTerms`.
  fn new_impl(pool: ByteBlockPool, bytes_used: SharedCounter) -> Self {
    Self {
      bytes_used,
      pool,
      delete_terms: HashMap::new(),
      terms_size: 0,
    }
  }
  /// Gets the newest document ID of the deleted term.
  ///
  /// Returns the newest document ID if the term exists, otherwise returns
  /// `-1`.
  pub(crate) fn get(&self, term: &Term) -> i32 {
    if let Some(hash) = self.delete_terms.get(&term.field) {
      hash.get(&term.bytes, &self.pool)
    } else {
      -1
    }
  }
  pub(crate) fn clear(&mut self) {
    self.pool.reset(false, false);
    let used = -self.bytes_used.get();
    self.bytes_used.add_and_get(used);
    self.delete_terms.clear();
    self.terms_size = 0;
  }

  pub(crate) fn size(&self) -> i32 {
    self.terms_size
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.terms_size == 0
  }
  /// Just for test, not efficient.
  pub(crate) fn key_set(&self) -> Result<HashSet<Term>> {
    let mut set = HashSet::new();
    for (field, hash) in &self.delete_terms {
      for bytes in hash.key_set(&self.pool)? {
        set.insert(Term::new(field.clone(), bytes));
      }
    }
    Ok(set)
  }

  /// Consume all terms in a sorted order.
  ///
  /// Note: This is a destructive operation as it calls
  /// `BytesRefHash::sort()`.
  #[allow(clippy::type_complexity)]
  pub(crate) fn for_each_ordered<F>(&mut self, mut consumer: F) -> Result<()>
  where
    F: FnMut(&Term, i32) -> Result<()>,
  {
    let mut delete_fields: Vec<(&String, &mut BytesRefIntMap)> =
      self.delete_terms.iter_mut().collect();
    delete_fields.sort_by(|a, b| a.0.cmp(b.0));

    let mut scratch = Term::new("", BytesRef::new());
    for (field, terms) in delete_fields {
      scratch.field = field.clone();
      terms.bytes_ref_hash.sort(&self.pool)?;
      let indices = &terms.bytes_ref_hash.ids;
      for i in 0..terms.bytes_ref_hash.count {
        let index = indices[i as usize];
        terms
          .bytes_ref_hash
          .get(index, &mut scratch.bytes, &self.pool)?;
        consumer(&scratch, terms.values[index as usize])?;
      }
    }
    Ok(())
  }
  #[cfg(test)]
  pub(crate) fn get_pool(&self) -> &ByteBlockPool {
    &self.pool
  }
}

pub trait DeletedTermConsumer {
  fn accept(&mut self, term: &Term, doc_id: i32) -> Result<()>;
}
impl Accountable for DeletedTerms {
  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(self.bytes_used.get())
  }
}
impl fmt::Display for DeletedTerms {
  /// Used for `BufferedUpdates::VERBOSE_DELETES`.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut entries = Vec::new();
    for term in self.key_set().map_err(|_| fmt::Error)?.iter() {
      entries.push(format!("{}={}", term, self.get(term)));
    }

    write!(f, "{{{}}}", entries.join(", "))
  }
}

struct BytesRefIntMap {
  counter: SharedCounter,
  pub(crate) bytes_ref_hash: BytesRefHash<DirectBytesStartArray>,
  values: Vec<i32>,
}

impl BytesRefIntMap {
  pub fn new(counter: SharedCounter) -> Result<Self> {
    let bytes_ref_hash = BytesRefHash::from_bytes_start_array(
      DEFAULT_CAPACITY,
      DirectBytesStartArray::with_counter(DEFAULT_CAPACITY as usize, counter.clone()),
    )?;
    Ok(BytesRefIntMap::new_impl(counter, bytes_ref_hash))
  }
  fn new_impl(counter: SharedCounter, bytes_ref_hash: BytesRefHash<DirectBytesStartArray>) -> Self {
    let values = vec![0; DEFAULT_CAPACITY as usize];

    counter.add_and_get(size_of_vec(&values));

    Self {
      counter,
      bytes_ref_hash,
      values,
    }
  }
  fn key_set(&self, pool: &ByteBlockPool) -> Result<HashSet<BytesRef<Vec<u8>>>> {
    let mut scratch = BytesRef::new();
    let mut set = HashSet::new();

    for i in 0..self.bytes_ref_hash.size() {
      self.bytes_ref_hash.get(i, &mut scratch, pool)?;
      set.insert(BytesRef::deep_copy_of(&scratch));
    }
    Ok(set)
  }
  fn put(&mut self, key: &BytesRef<Vec<u8>>, value: i32, pool: &mut ByteBlockPool) -> Result<bool> {
    debug_assert!(value >= 0, "Value must be non-negative.");
    let e = self.bytes_ref_hash.add(key, pool)?;
    if e < 0 {
      self.values[(-e - 1) as usize] = value;
      Ok(false)
    } else {
      if e as usize >= self.values.len() {
        let old_size = size_of_vec(&self.values);
        ArrayUtil::grow_with_len(&mut self.values, (e + 1) as usize)?;
        self
          .counter
          .add_and_get(size_of_vec(&self.values).saturating_sub(old_size));
      }
      self.values[e as usize] = value;
      Ok(true)
    }
  }
  fn get(&self, key: &BytesRef<Vec<u8>>, pool: &ByteBlockPool) -> i32 {
    let e = self.bytes_ref_hash.find(key, pool);
    if e == -1 { -1 } else { self.values[e as usize] }
  }
}

pub const MAX_INT: i32 = i32::MAX;
