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
use std::cmp::{Ordering, max, min};
use std::sync::Arc;

use crate::core::index::BytesRef;
use crate::core::index::doc_values_update::{DocValuesUpdate, DocValuesUpdateBase};
use crate::core::index::term::Term;
use crate::core::util::accountable::Accountable;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bits::{Bits, BitsEnum2, MatchAllBits};
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::ram_usage_estimator::{size_of_string as ram_size_of_string, size_of_vec};
use crate::core::util::{
  BytesRefArray, Counter, IndexedBytesRefIterator, IndexedBytesRefIteratorImpl, NaturalOrder,
  SharedCounter, SortState, SortableBytesRefArray,
};

/// This struct efficiently buffers numeric and binary field updates and stores
/// terms, values, and metadata in a memory-efficient way without creating large
/// amounts of objects.
///
/// Update terms are stored without de-duplicating the update term. In general,
/// we try to optimize for several use-cases. For instance, we try to use
/// constant space for update terms field since the common case always updates
/// on the same field. Also, for `docUpTo`, we try to optimize for the case when
/// updates should be applied to all docs, i.e., when `docUpTo = i32::MAX`. In
/// other cases, each update will likely have a different `docUpTo`.
///
/// Along the same lines, this implementation optimizes the case when all
/// updates have a value. Lastly, if all updates share the same value for a
/// numeric field, we only store the value once.
#[derive(Debug)]
pub(crate) struct FieldUpdatesBuffer {
  bytes_used: SharedCounter,
  num_updates: usize,
  // we use a very simple approach and store the update term values without
  // de-duplication which is also not a common case to keep updating the
  // same value more than once... we might pay a higher price in terms of
  // memory in certain cases but will gain on CPU for those. We also use
  // a stable sort to sort to apply the terms in order
  // since by definition we store them in order.
  term_values: BytesRefArray,
  term_sort_state: Arc<SortState>,
  byte_values: Option<BytesRefArray>, /* this will be null if we are
                                       * buffering numerics  */
  docs_upto: Vec<i32>,
  numeric_values: Option<Vec<i64>>, /* this will be null if we are
                                     * buffering binaries  */
  has_values: Option<FixedBitSet>,
  max_numeric: i64,
  min_numeric: i64,
  fields: Vec<String>,
  is_numeric: bool,
  finished: bool,
}

impl FieldUpdatesBuffer {
  const SELF_SHALLOW_SIZE: i64 = 0;

  fn new(
    bytes_used: SharedCounter,
    initial_value: &DocValuesUpdate,
    doc_upto: i32,
    is_numeric: bool,
  ) -> Result<Self> {
    let mut has_values = None;
    if !initial_value.has_value {
      let bs = FixedBitSet::new(1);
      bytes_used.add_and_get(bs.ram_bytes_used()?);
      has_values = Some(bs);
    }
    bytes_used.add_and_get(Self::size_of_string(&initial_value.term.field));

    let mut buffer = FieldUpdatesBuffer {
      bytes_used: bytes_used.clone(),
      num_updates: 1,
      term_values: BytesRefArray::new(bytes_used.clone())?,
      term_sort_state: Arc::new(SortState::new(None)),
      byte_values: if is_numeric {
        None
      } else {
        Some(BytesRefArray::new(bytes_used.clone())?)
      },
      docs_upto: vec![doc_upto],
      numeric_values: if is_numeric { Some(vec![]) } else { None },
      has_values,
      max_numeric: i64::MIN,
      min_numeric: i64::MAX,
      // TODO: we should estimate the size of the fields array
      fields: vec![initial_value.term.field.clone()],
      is_numeric,
      finished: false,
    };
    buffer.term_values.append(&initial_value.term.bytes)?;
    Ok(buffer)
  }
  pub(crate) fn from_numeric_update(
    bytes_used: SharedCounter,
    initial_value: &DocValuesUpdate,
    doc_upto: i32,
  ) -> Result<Self> {
    let numeric = initial_value
      .sub_update
      .get_numeric()
      .ok_or_else(|| LuceneError::illegal_argument("Missing numeric value"))?;
    let has_values = numeric.has_value();
    let (numeric_values, max_numeric, min_numeric) = if has_values {
      let value = numeric.get_value();
      (vec![value], value, value)
    } else {
      (vec![0], i64::MIN, i64::MAX)
    };
    let mut buffer = Self::new(bytes_used, initial_value, doc_upto, true)?;
    buffer.numeric_values = Some(numeric_values);
    buffer.max_numeric = max_numeric;
    buffer.min_numeric = min_numeric;
    {
      buffer.bytes_used.add_and_get(BitUtil::LONG_BYTES as i64);
    }
    Ok(buffer)
  }

  pub(crate) fn from_binary_update(
    bytes_used: SharedCounter,
    initial_value: &DocValuesUpdate,
    doc_upto: i32,
  ) -> Result<Self> {
    let binary = initial_value
      .sub_update
      .get_binary()
      .ok_or_else(|| LuceneError::illegal_argument("Missing binary value"))?;
    let has_values = binary.has_value();
    let value = if has_values {
      binary.get_value()
    } else {
      &BytesRef::default()
    };
    let mut buffer = Self::new(bytes_used, initial_value, doc_upto, false)?;
    if has_values {
      debug_assert!(buffer.byte_values.is_some());
      buffer.byte_values.as_mut().unwrap().append(value)?;
    }
    Ok(buffer)
  }

  fn size_of_string(s: &String) -> i64 {
    ram_size_of_string(s)
  }

  pub(crate) fn get_max_numeric(&self) -> i64 {
    debug_assert!(self.is_numeric);
    if self.min_numeric == i64::MAX && self.max_numeric == i64::MIN {
      return 0;
    }
    self.max_numeric
  }

  pub(crate) fn get_min_numeric(&self) -> i64 {
    debug_assert!(self.is_numeric);
    if self.min_numeric == i64::MAX && self.max_numeric == i64::MIN {
      return 0;
    }
    self.min_numeric
  }
  pub(crate) fn add(
    &mut self,
    field: String,
    doc_upto: i32,
    ord: usize,
    has_value: bool,
  ) -> Result<()> {
    debug_assert!(!self.finished, "buffer was finished already");
    let fields_len = self.fields.len();
    if self.fields[0] != field || fields_len != 1 {
      if fields_len <= ord {
        let old_size = size_of_vec(&self.fields);
        ArrayUtil::grow_with_len(&mut self.fields, ord + 1)?;
        if fields_len == 1 {
          for i in 1..ord {
            self.fields[i] = self.fields[0].clone();
          }
        }
        self
          .bytes_used
          .add_and_get(size_of_vec(&self.fields).saturating_sub(old_size));
      }
      if self.fields[0] != field {
        self.bytes_used.add_and_get(Self::size_of_string(&field));
      }
      self.fields[ord] = field;
    }

    let docs_upto_len = self.docs_upto.len();
    if self.docs_upto[0] != doc_upto || docs_upto_len != 1 {
      if docs_upto_len <= ord {
        let old_size = size_of_vec(&self.docs_upto);
        ArrayUtil::grow_with_len(&mut self.docs_upto, ord + 1)?;
        if docs_upto_len == 1 {
          for i in 1..ord {
            self.docs_upto[i] = self.docs_upto[0];
          }
        }
        self
          .bytes_used
          .add_and_get(size_of_vec(&self.docs_upto).saturating_sub(old_size));
      }
      self.docs_upto[ord] = doc_upto;
    }

    if !has_value || self.has_values.is_some() {
      if let Some(bitset) = self.has_values.as_mut() {
        if bitset.length() <= ord {
          let old_size = bitset.ram_bytes_used()?;
          bitset.ensure_capacity(ord + 1)?;
          self
            .bytes_used
            .add_and_get(bitset.ram_bytes_used()?.saturating_sub(old_size));
        }
      } else {
        let mut new_bitset = FixedBitSet::new(ord + 1);
        new_bitset.set_with_range(0, ord);
        self.bytes_used.add_and_get(new_bitset.ram_bytes_used()?);
        self.has_values = Some(new_bitset);
      }

      if has_value {
        self.has_values.as_mut().unwrap().set(ord);
      }
    }
    Ok(())
  }
  pub fn add_update_with_long(&mut self, term: &Term, value: i64, doc_upto: i32) -> Result<()> {
    debug_assert!(self.is_numeric);
    let ord = self.append(term)?;
    let field = term.field.clone();
    self.add(field, doc_upto, ord, true)?;
    self.min_numeric = min(self.min_numeric, value);
    self.max_numeric = max(self.max_numeric, value);
    let numeric_values = self.numeric_values.as_mut().unwrap();
    let numeric_values_len = numeric_values.len();
    if numeric_values[0] != value || numeric_values_len != 1 {
      if numeric_values_len <= ord {
        let old_size = size_of_vec(numeric_values);
        ArrayUtil::grow_with_len(numeric_values, ord + 1)?;
        if numeric_values_len == 1 {
          for i in 1..ord {
            numeric_values[i] = numeric_values[0];
          }
        }
        self
          .bytes_used
          .add_and_get(size_of_vec(numeric_values).saturating_sub(old_size));
      }
      numeric_values[ord] = value;
    }
    Ok(())
  }

  pub(crate) fn add_no_value(&mut self, term: &Term, doc_upto: i32) -> Result<()> {
    let ord = self.append(term)?;
    self.add(term.field.clone(), doc_upto, ord, false)
  }
  pub(crate) fn add_update_with_bytes_ref(
    &mut self,
    term: &Term,
    value: &BytesRef<Vec<u8>>,
    doc_upto: i32,
  ) -> Result<()> {
    debug_assert!(!self.is_numeric);
    let ord = self.append(term)?;
    self
      .byte_values
      .as_mut()
      .ok_or_else(|| LuceneError::illegal_state("byte_values is None"))?
      .append(value)?;
    self.add(term.field.clone(), doc_upto, ord, true)?;
    Ok(())
  }

  fn append(&mut self, term: &Term) -> Result<usize> {
    self.term_values.append(&term.bytes)?;
    let ord = self.num_updates;
    self.num_updates += 1;
    Ok(ord)
  }
  pub(crate) fn finish(&mut self) -> Result<()> {
    if self.finished {
      return Err(LuceneError::illegal_state("Buffer was finished already"));
    }
    self.finished = true;
    let sorted_terms =
      self.has_single_value() && self.has_values.is_none() && self.fields.len() == 1;
    if sorted_terms {
      self.term_sort_state = Arc::new(self.term_values.sort(NaturalOrder, true)?);
      debug_assert!(self.assert_term_and_doc_in_order());
      self
        .bytes_used
        .add_and_get(self.term_sort_state.ram_bytes_used()?);
    }

    Ok(())
  }
  fn assert_term_and_doc_in_order(&mut self) -> bool {
    // it's used for debug_assert! , so we roughly copy data
    let mut iterator = self
      .term_values
      .iterator_with_state(self.term_sort_state.clone());
    let mut last = None;
    let mut last_ord = 0;

    let result: Result<()> = (|| {
      while let Some(current) = iterator.next()? {
        let current = current.into_owned();
        if let Some(last_term) = &last {
          let cmp = current.cmp(last_term);
          debug_assert_ne!(cmp, Ordering::Less, "term in reverse order");
          let last_doc_upto = self.docs_upto[Self::get_array_index(self.docs_upto.len(), last_ord)];
          let current_doc_upto =
            self.docs_upto[Self::get_array_index(self.docs_upto.len(), iterator.ord())];
          debug_assert!(
            cmp != Ordering::Equal || last_doc_upto <= current_doc_upto,
            "doc id in reverse order"
          );
        }
        last = Some(current);
        last_ord = iterator.ord();
      }
      Ok(())
    })();
    debug_assert!(
      result.is_ok(),
      "assert_term_and_doc_in_order failed: {:?}",
      result.err()
    );
    true
  }
  pub(crate) fn iterator(&self) -> Result<BufferedUpdateIterator<'_>> {
    if !self.finished {
      return Err(LuceneError::illegal_state("Buffer was not finished"));
    }
    Ok(BufferedUpdateIterator::new(self))
  }
  pub(crate) fn is_numeric(&self) -> bool {
    debug_assert!(self.is_numeric || self.byte_values.is_some());
    self.is_numeric
  }
  pub(crate) fn has_single_value(&self) -> bool {
    // we only do this optimization for numerics so far.
    self.is_numeric && self.numeric_values.as_ref().unwrap().len() == 1
  }
  pub(crate) fn get_numeric_value(&self, idx: usize) -> Result<i64> {
    if let Some(ref has_values) = self.has_values
      && !has_values.get(idx)?
    {
      return Ok(0);
    }
    debug_assert!(self.numeric_values.is_some());
    let length = self.numeric_values.as_ref().unwrap().len();
    Ok(self.numeric_values.as_ref().unwrap()[Self::get_array_index(length, idx)])
  }
  fn get_array_index(array_length: usize, index: usize) -> usize {
    debug_assert!(
      array_length == 1 || array_length > index,
      "illegal array index length: {array_length} index: {index}"
    );
    min(array_length - 1, index)
  }
}
/// An iterator that iterates over all updates in insertion order.
pub struct BufferedUpdateIterator<'a> {
  term_values_iterator: IndexedBytesRefIteratorImpl<'a>,
  look_ahead_term_iterator: Option<IndexedBytesRefIteratorImpl<'a>>,
  byte_values_iterator: Option<IndexedBytesRefIteratorImpl<'a>>,
  buffered_update: BufferedUpdate,
  updates_with_value: Option<UpdateBits<'a>>,
  fields_length: usize,
  docs_upto_length: usize,
  numeric_values_length: usize,
  field_updates_buffer: &'a FieldUpdatesBuffer,
}

impl<'a> BufferedUpdateIterator<'a> {
  pub fn new(field_updates_buffer: &'a FieldUpdatesBuffer) -> Self {
    let term_values_iterator = field_updates_buffer
      .term_values
      .iterator_with_state(field_updates_buffer.term_sort_state.clone());
    let look_ahead_term_iterator = if field_updates_buffer.term_sort_state.indices.is_some() {
      Some(
        field_updates_buffer
          .term_values
          .iterator_with_state(field_updates_buffer.term_sort_state.clone()),
      )
    } else {
      None
    };
    let byte_values_iterator = if field_updates_buffer.is_numeric {
      None
    } else {
      debug_assert!(field_updates_buffer.byte_values.is_some());
      Some(
        field_updates_buffer
          .byte_values
          .as_ref()
          .unwrap()
          .iterator(),
      )
    };
    let updates_with_value = if let Some(item) = &field_updates_buffer.has_values {
      UpdateBits::B(item)
    } else {
      UpdateBits::A(MatchAllBits::new(field_updates_buffer.num_updates))
    };
    let fields_length = field_updates_buffer.fields.len();
    let docs_upto_length = field_updates_buffer.docs_upto.len();
    let numeric_values_length = if field_updates_buffer.is_numeric {
      field_updates_buffer.numeric_values.as_ref().unwrap().len()
    } else {
      0
    };
    debug_assert!(docs_upto_length <= i32::MAX as usize);
    BufferedUpdateIterator {
      term_values_iterator,
      look_ahead_term_iterator,
      byte_values_iterator,
      buffered_update: BufferedUpdate::default(),
      updates_with_value: Some(updates_with_value),
      fields_length,
      docs_upto_length,
      numeric_values_length,
      field_updates_buffer,
    }
  }
  /// If all updates update a single field to the same value, then we can
  /// apply these updates in the term order instead of the request order
  /// as both will yield the same result. This optimization allows us to
  /// iterate the term dictionary faster and de-duplicate updates.
  pub(crate) fn is_sorted_terms(&self) -> bool {
    self.field_updates_buffer.term_sort_state.indices.is_some()
  }
  /// Moves to the next BufferedUpdate or return None if all updates are
  /// consumed. The returned instance is a shared instance and must be
  /// fully consumed before the next call to this method.
  pub(crate) fn next_value(&mut self) -> Result<Option<BufferedUpdate>> {
    let mut buffered_update = BufferedUpdate::default();
    let next_term = self.next_term()?;

    if let Some(next) = next_term {
      let idx = self.term_values_iterator.ord();
      self.buffered_update.term_value = Some(next.clone());
      buffered_update.term_value = Some(next);
      buffered_update.has_value = self.updates_with_value.as_ref().unwrap().get(idx)?;
      buffered_update.term_field = self.field_updates_buffer.fields
        [FieldUpdatesBuffer::get_array_index(self.fields_length, idx)]
      .clone();
      buffered_update.doc_upto = self.field_updates_buffer.docs_upto
        [FieldUpdatesBuffer::get_array_index(self.docs_upto_length, idx)];

      if buffered_update.has_value {
        if self.field_updates_buffer.is_numeric {
          buffered_update.numeric_value =
            self.field_updates_buffer.numeric_values.as_ref().unwrap()
              [FieldUpdatesBuffer::get_array_index(self.numeric_values_length, idx)];
          buffered_update.binary_value = None;
        } else {
          debug_assert!(self.numeric_values_length == 0);
          match &mut self.byte_values_iterator {
            Some(iterator) => match iterator.next()? {
              Some(bytes_ref) => {
                buffered_update.binary_value = Some(bytes_ref.into_owned());
              },
              None => {
                buffered_update.binary_value = None;
              },
            },
            None => {
              buffered_update.binary_value = None;
            },
          }
        }
      } else {
        buffered_update.binary_value = None;
        buffered_update.numeric_value = 0;
      }
      Ok(Some(buffered_update))
    } else {
      Ok(None)
    }
  }

  fn next_term(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
    if let Some(look_ahead_term_iterator) = &mut self.look_ahead_term_iterator {
      if self.buffered_update.term_value.is_none() {
        look_ahead_term_iterator.next()?;
      }
      let mut last_term;
      let mut ahead_term;
      loop {
        ahead_term = look_ahead_term_iterator.next()?;
        match self.term_values_iterator.next()? {
          Some(term) => {
            last_term = Some(term.into_owned());
          },
          None => {
            last_term = None;
          },
        }

        if let Some(ahead) = ahead_term {
          let ahead = ahead.into_owned();
          // Shortcut to avoid equals, we did a stable sort before, so
          // aheadTerm can only equal
          // lastTerm when aheadTerm has a lager ord.
          if look_ahead_term_iterator.ord() > self.term_values_iterator.ord()
            && ahead == *last_term.as_mut().unwrap()
          {
            continue;
          }
        }
        break;
      }
      Ok(last_term)
    } else {
      match self.term_values_iterator.next()? {
        Some(term) => Ok(Some(term.into_owned())),
        None => Ok(None),
      }
    }
  }
}
/// # Warning
/// This struct should not be used as a map key or in data structures that depend on `Hash` and `Eq`.
#[derive(Default, Clone)]

pub struct BufferedUpdate {
  /// the max document ID this update should be applied to.
  pub doc_upto: i32,
  /// a numeric value or 0 if this buffer holds binary updates.
  pub numeric_value: i64,
  /// a binary value or None if this buffer holds numeric updates.
  pub binary_value: Option<BytesRef<Vec<u8>>>,
  /// true if this update has a value.
  pub has_value: bool,
  /// The update terms field. This will never be None.
  pub term_field: String,
  /// The update terms value. This will never be None.
  pub term_value: Option<BytesRef<Vec<u8>>>,
}

impl BufferedUpdate {
  pub fn new(
    doc_upto: i32,
    numeric_value: i64,
    binary_value: Option<BytesRef<Vec<u8>>>,
    has_value: bool,
    term_field: String,
    term_value: Option<BytesRef<Vec<u8>>>,
  ) -> Self {
    BufferedUpdate {
      doc_upto,
      numeric_value,
      binary_value,
      has_value,
      term_field,
      term_value,
    }
  }
  pub(crate) fn get_binary_value(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.binary_value.as_ref()
  }
}

type UpdateBits<'a> = BitsEnum2<MatchAllBits, &'a FixedBitSet>;
