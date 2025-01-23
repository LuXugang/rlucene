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
use crate::index::doc_values_update::{DocValuesUpdate, DocValuesUpdateBase};
use crate::index::term::Term;
use crate::index::BytesRef;
use crate::util::accountable::Accountable;
use crate::util::bit_set::BitSet;
use crate::util::bit_util::BitUtil;
use crate::util::bits::{Bits, MatchAllBits};
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::LuceneError;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::{
    BytesRefArray, Counter, CounterEnum, IndexedBytesRefIteratorImpl, NaturalOrder, SortState,
    SortableBytesRefArray,
};
use std::cmp::{max, min, Ordering};
use std::sync::{Arc, Mutex};

/// This struct efficiently buffers numeric and binary field updates and stores terms, values, and
/// metadata in a memory-efficient way without creating large amounts of objects.
///
/// Update terms are stored without de-duplicating the update term. In general, we try to optimize
/// for several use-cases. For instance, we try to use constant space for update terms field since
/// the common case always updates on the same field. Also, for `docUpTo`, we try to optimize for the
/// case when updates should be applied to all docs, i.e., when `docUpTo = i32::MAX`. In other cases,
/// each update will likely have a different `docUpTo`.
///
/// Along the same lines, this implementation optimizes the case when all updates have a value.
/// Lastly, if all updates share the same value for a numeric field, we only store the value once.
#[allow(unused)]
pub struct FieldUpdatesBuffer {
    bytes_used: Arc<Mutex<CounterEnum>>,
    num_updates: i32,
    // we use a very simple approach and store the update term values without de-duplication
    // which is also not a common case to keep updating the same value more than once...
    // we might pay a higher price in terms of memory in certain cases but will gain
    // on CPU for those. We also use a stable sort to sort to apply the terms in order
    // since by definition we store them in order.
    term_values: BytesRefArray,
    term_sort_state: Arc<SortState>,
    byte_values: Option<BytesRefArray>, // this will be null if we are buffering numerics
    docs_up_to: Vec<i32>,
    numeric_values: Option<Vec<i64>>, // this will be null if we are buffering binaries
    has_values: Option<FixedBitSet>,
    max_numeric: i64,
    min_numeric: i64,
    fields: Vec<String>,
    is_numeric: bool,
    finished: bool,
}

impl FieldUpdatesBuffer {
    #[allow(unused)]
    const SELF_SHALLOW_SIZE: i64 = 0;
    #[allow(unused)]
    const STRING_SHALLOW_SIZE: i64 = 0;
    pub fn new(
        bytes_used: Arc<Mutex<CounterEnum>>,
        initial_value: DocValuesUpdate,
        doc_upto: i32,
        is_numeric: bool,
    ) -> Result<Self, LuceneError> {
        let has_values = if !initial_value.has_value {
            Some(FixedBitSet::new(1))
        } else {
            None
        };
        {
            let mut bytes_used_guard = bytes_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            bytes_used_guard.add_and_get(Self::size_of_string(&initial_value.term.field));
            if !initial_value.has_value {
                bytes_used_guard.add_and_get(has_values.as_ref().unwrap().ram_bytes_used());
            }
        }
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
            docs_up_to: vec![doc_upto],
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
    pub fn from_numeric_update(
        bytes_used: Arc<Mutex<CounterEnum>>,
        initial_value: DocValuesUpdate,
        doc_up_to: i32,
    ) -> Result<Self, LuceneError> {
        let numeric = initial_value
            .sub_update
            .get_numeric()
            .ok_or_else(|| LuceneError::illegal_argument("Missing numeric value".to_string()))?;
        let has_values = numeric.has_value();
        let (numeric_values, max_numeric, min_numeric) = if has_values {
            let value = numeric.get_value();
            (vec![value], value, value)
        } else {
            (vec![0], i64::MIN, i64::MAX)
        };
        let mut buffer = Self::new(bytes_used, initial_value, doc_up_to, true)?;
        buffer.numeric_values = Some(numeric_values);
        buffer.max_numeric = max_numeric;
        buffer.min_numeric = min_numeric;
        {
            let mut bytes_used_guard = buffer
                .bytes_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?;
            bytes_used_guard.add_and_get(BitUtil::LONG_BYTES as i64);
        }
        Ok(buffer)
    }

    pub fn from_binary_update(
        bytes_used: Arc<Mutex<CounterEnum>>,
        initial_value: DocValuesUpdate,
        doc_up_to: i32,
    ) -> Result<Self, LuceneError> {
        let binary = initial_value
            .sub_update
            .get_binary()
            .ok_or_else(|| LuceneError::illegal_argument("Missing binary value".to_string()))?;
        let has_values = binary.has_value();
        let value = if has_values {
            binary.get_value()
        } else {
            BytesRef::default()
        };
        let mut buffer = Self::new(bytes_used, initial_value, doc_up_to, false)?;
        if has_values {
            debug_assert!(buffer.byte_values.is_some());
            buffer.byte_values.as_mut().unwrap().append(&value)?;
        }
        Ok(buffer)
    }

    fn size_of_string(_s: &str) -> i64 {
        //TODO: memory calculation not implemented
        0
    }

    pub fn get_max_numeric(&self) -> i64 {
        debug_assert!(self.is_numeric);
        if self.min_numeric == i64::MAX && self.max_numeric == i64::MIN {
            return 0;
        }
        self.max_numeric
    }

    pub fn get_min_numeric(&self) -> i64 {
        debug_assert!(self.is_numeric);
        if self.min_numeric == i64::MAX && self.max_numeric == i64::MIN {
            return 0;
        }
        self.min_numeric
    }
    pub fn add(
        &mut self,
        field: String,
        doc_upto: i32,
        ord: usize,
        has_value: bool,
    ) -> Result<(), LuceneError> {
        debug_assert!(!self.finished, "buffer was finished already");
        debug_assert!(
            ord <= i32::MAX as usize,
            "ord must be <= Integer.MAX_VALUE,Keep consistent with Java Lucene"
        );
        let fields_len = self.fields.len();
        if self.fields[0] != field || fields_len != 1 {
            if fields_len <= ord {
                // TODO: ArrayUtil.grow not implemented, so we roughly implement it here
                self.fields.resize(ord + 1, self.fields[0].clone());
                // TODO: memory calculation not implemented
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(0);
            }
            if self.fields[0] != field {
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(field.len() as i64);
            }
            self.fields[ord] = field;
        }

        if self.docs_up_to[0] != doc_upto || self.docs_up_to.len() != 1 {
            if self.docs_up_to.len() <= ord {
                // TODO: ArrayUtil.grow not implemented, so we roughly implement it here
                self.docs_up_to.resize(ord + 1, self.docs_up_to[0]);
                // TODO: memory calculation not implemented
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(0);
            }
            self.docs_up_to[ord] = doc_upto;
        }

        if !has_value || self.has_values.is_some() {
            if self.has_values.is_none() {
                let mut new_bitset = FixedBitSet::new(ord as i32 + 1);
                new_bitset.set_with_range(0, ord as i32);
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(new_bitset.ram_bytes_used());
                self.has_values = Some(new_bitset);
            } else if self.has_values.as_ref().unwrap().length() as usize <= ord {
                let bitset = self.has_values.as_mut().unwrap();
                FixedBitSet::ensure_capacity(bitset, (ord + 1) as i32);
                // TODO: memory calculation not implemented
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(0);
            }
            if has_value {
                self.has_values.as_mut().unwrap().set(ord as i32);
            }
        }
        Ok(())
    }
    pub fn add_update_with_long(
        &mut self,
        term: Term,
        value: i64,
        doc_up_to: i32,
    ) -> Result<(), LuceneError> {
        debug_assert!(self.is_numeric);
        let ord = self.append(&term)?;
        let field = term.field.clone();
        self.add(field, doc_up_to, ord as usize, true)?;
        self.min_numeric = min(self.min_numeric, value);
        self.max_numeric = max(self.max_numeric, value);
        let numeric_values = self.numeric_values.as_mut().unwrap();
        if numeric_values[0] != value || numeric_values.len() != 1 {
            if numeric_values.len() <= ord as usize {
                numeric_values.resize(ord as usize + 1, numeric_values[0]);
                // TODO: memory calculation not implemented
                self.bytes_used
                    .lock()
                    .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                    .add_and_get(0);
            }
            numeric_values[ord as usize] = value;
        }
        Ok(())
    }

    pub fn add_no_value(&mut self, term: Term, doc_up_to: i32) -> Result<(), LuceneError> {
        let ord = self.append(&term)?;
        self.add(term.field.clone(), doc_up_to, ord as usize, false)
    }
    pub fn add_update_with_bytes_ref(
        &mut self,
        term: Term,
        value: &BytesRef,
        doc_up_to: i32,
    ) -> Result<(), LuceneError> {
        debug_assert!(!self.is_numeric);
        let ord = self.append(&term)?;
        self.byte_values.as_mut().unwrap().append(value)?;
        self.add(term.field.clone(), doc_up_to, ord as usize, true)?;
        Ok(())
    }

    pub fn append(&mut self, term: &Term) -> Result<i32, LuceneError> {
        self.term_values.append(&term.bytes)?;
        let ord = self.num_updates;
        self.num_updates += 1;
        Ok(ord)
    }
    pub fn finish(&mut self) -> Result<(), LuceneError> {
        if self.finished {
            return Err(LuceneError::illegal_state(
                "Buffer was finished already".to_string(),
            ));
        }
        self.finished = true;
        let sorted_terms =
            self.has_single_value() && self.has_values.is_none() && self.fields.len() == 1;
        if sorted_terms {
            self.term_sort_state = Arc::new(self.term_values.sort(NaturalOrder::default(), true)?);
            debug_assert!(self.assert_term_and_doc_in_order());
            // TODO: memory calculation not implemented
            self.bytes_used
                .lock()
                .map_err(|_| LuceneError::illegal_state("Failed to acquire lock.".to_string()))?
                .add_and_get(0);
        }

        Ok(())
    }
    pub fn assert_term_and_doc_in_order(&mut self) -> bool {
        // it's used for debug_assert! , so we roughly copy data
        let mut iterator = self
            .term_values
            .iterator_with_state(self.term_sort_state.clone());
        let mut last: Option<BytesRef> = None;
        let mut last_ord = 0;

        let result: Result<(), LuceneError> = (|| {
            while let Some(current) = iterator.next()? {
                if let Some(last_term) = &last {
                    let cmp = current.cmp(last_term);
                    debug_assert_ne!(cmp, Ordering::Less, "term in reverse order");
                    let last_doc_up_to = self.docs_up_to
                        [Self::get_array_index(self.docs_up_to.len() as i32, last_ord) as usize];
                    let current_doc_up_to = self.docs_up_to[Self::get_array_index(
                        self.docs_up_to.len() as i32,
                        iterator.ord(),
                    ) as usize];
                    debug_assert!(
                        cmp != Ordering::Equal || last_doc_up_to <= current_doc_up_to,
                        "doc id in reverse order"
                    );
                }
                last = Some(current.clone());
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
    pub fn iterator(&self) -> Result<BufferedUpdateIterator, LuceneError> {
        if !self.finished {
            return Err(LuceneError::illegal_state(
                "Buffer was not finished".to_string(),
            ));
        }
        Ok(BufferedUpdateIterator::new(self))
    }
    pub fn is_numeric(&self) -> bool {
        debug_assert!(self.is_numeric || self.byte_values.is_some());
        self.is_numeric
    }
    pub fn has_single_value(&self) -> bool {
        // we only do this optimization for numerics so far.
        self.is_numeric && self.numeric_values.as_ref().unwrap().len() == 1
    }
    pub fn get_numeric_value(&self, idx: i32) -> i64 {
        if let Some(ref has_values) = self.has_values {
            if !has_values.get(idx) {
                return 0;
            }
        }
        assert!(self.numeric_values.is_some());
        let length = self.numeric_values.as_ref().unwrap().len();
        debug_assert!(length <= i32::MAX as usize);
        self.numeric_values.as_ref().unwrap()[Self::get_array_index(length as i32, idx) as usize]
    }
    pub fn get_array_index(array_length: i32, index: i32) -> i32 {
        assert!(
            array_length == 1 || array_length > index,
            "illegal array index length: {} index: {}",
            array_length,
            index
        );
        std::cmp::min(array_length - 1, index)
    }
}
/// An iterator that iterates over all updates in insertion order.
pub struct BufferedUpdateIterator<'a> {
    term_values_iterator: IndexedBytesRefIteratorImpl<'a>,
    look_ahead_term_iterator: Option<IndexedBytesRefIteratorImpl<'a>>,
    byte_values_iterator: Option<IndexedBytesRefIteratorImpl<'a>>,
    buffered_update: BufferedUpdate,
    updates_with_value: Option<BitsEnum<'a>>,
    fields_length: i32,
    docs_upto_length: i32,
    numeric_values_length: i32,
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
            BitsEnum::Fixed(item)
        } else {
            BitsEnum::All(MatchAllBits::new(field_updates_buffer.num_updates))
        };
        let fields_length = field_updates_buffer.fields.len();
        let docs_upto_length = field_updates_buffer.docs_up_to.len();
        let numeric_values_length = if field_updates_buffer.is_numeric {
            let length = field_updates_buffer.numeric_values.as_ref().unwrap().len();
            debug_assert!(length <= i32::MAX as usize);
            length as i32
        } else {
            0
        };
        debug_assert!(fields_length <= i32::MAX as usize);
        debug_assert!(docs_upto_length <= i32::MAX as usize);
        BufferedUpdateIterator {
            term_values_iterator,
            look_ahead_term_iterator,
            byte_values_iterator,
            buffered_update: BufferedUpdate::default(),
            updates_with_value: Some(updates_with_value),
            fields_length: fields_length as i32,
            docs_upto_length: docs_upto_length as i32,
            numeric_values_length,
            field_updates_buffer,
        }
    }
    /// If all updates update a single field to the same value, then we can apply these updates in
    /// the term order instead of the request order as both will yield the same result. This
    /// optimization allows us to iterate the term dictionary faster and de-duplicate updates.
    pub fn is_sorted_terms(&self) -> bool {
        self.field_updates_buffer.term_sort_state.indices.is_some()
    }
    /// Moves to the next BufferedUpdate or return null if all updates are consumed. The returned
    /// instance is a shared instance and must be fully consumed before the next call to this method.
    pub fn next_value(&mut self) -> Result<Option<BufferedUpdate>, LuceneError> {
        let mut buffered_update = BufferedUpdate::default();
        let next_term = self.next_term()?;

        if let Some(next) = next_term {
            let idx = self.term_values_iterator.ord();
            self.buffered_update.term_value = Some(next.clone());
            buffered_update.term_value = Some(next);
            buffered_update.has_value = self.updates_with_value.as_ref().unwrap().get(idx);
            buffered_update.term_field = self.field_updates_buffer.fields
                [FieldUpdatesBuffer::get_array_index(self.fields_length, idx) as usize]
                .clone();
            buffered_update.doc_up_to = self.field_updates_buffer.docs_up_to
                [FieldUpdatesBuffer::get_array_index(self.docs_upto_length, idx) as usize];

            if buffered_update.has_value {
                if self.field_updates_buffer.is_numeric {
                    buffered_update.numeric_value =
                        self.field_updates_buffer.numeric_values.as_ref().unwrap()
                            [FieldUpdatesBuffer::get_array_index(self.numeric_values_length, idx)
                                as usize];
                    buffered_update.binary_value = None;
                } else {
                    debug_assert!(self.numeric_values_length == 0);
                    buffered_update.binary_value =
                        self.byte_values_iterator.as_mut().unwrap().next()?;
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

    fn next_term(&mut self) -> Result<Option<BytesRef>, LuceneError> {
        if let Some(look_ahead_term_iterator) = &mut self.look_ahead_term_iterator {
            if self.buffered_update.term_value.is_none() {
                look_ahead_term_iterator.next()?;
            }
            let mut last_term: Option<BytesRef>;
            let mut ahead_term: Option<BytesRef>;
            loop {
                ahead_term = look_ahead_term_iterator.next()?;
                last_term = self.term_values_iterator.next()?;

                if let Some(ahead) = ahead_term {
                    if let Some(last) = &last_term {
                        // Shortcut to avoid equals, we did a stable sort before, so aheadTerm can only equal
                        // lastTerm when aheadTerm has a lager ord.
                        if look_ahead_term_iterator.ord() > self.term_values_iterator.ord()
                            && ahead == *last
                        {
                            continue;
                        }
                    }
                }
                break;
            }
            Ok(last_term)
        } else {
            self.term_values_iterator.next()
        }
    }
}
/// # Warning
/// this struct should not be use in map or other data-structures that use hashCode / equals
#[derive(Default, Clone)]
pub struct BufferedUpdate {
    /// the max document ID this update should be applied to.
    pub doc_up_to: i32,
    /// a numeric value or 0 if this buffer holds binary updates.
    pub numeric_value: i64,
    /// a binary value or null if this buffer holds numeric updates.
    pub binary_value: Option<BytesRef>,
    /// true if this update has a value.
    pub has_value: bool,
    /// The update terms field. This will never be null.
    pub term_field: String,
    /// The update terms value. This will never be null.
    pub term_value: Option<BytesRef>,
}

impl BufferedUpdate {
    pub fn new(
        doc_up_to: i32,
        numeric_value: i64,
        binary_value: Option<BytesRef>,
        has_value: bool,
        term_field: String,
        term_value: Option<BytesRef>,
    ) -> Self {
        BufferedUpdate {
            doc_up_to,
            numeric_value,
            binary_value,
            has_value,
            term_field,
            term_value,
        }
    }
}
pub enum BitsEnum<'a> {
    All(MatchAllBits),
    Fixed(&'a FixedBitSet),
}
impl BitsEnum<'_> {
    fn get(&self, idx: i32) -> bool {
        match self {
            BitsEnum::All(all) => all.get(idx),
            BitsEnum::Fixed(fixed) => fixed.get(idx),
        }
    }
}
