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
use crate::index::docs_with_field_set::DocsWithFieldSet;
use crate::index::field_info::FieldInfo;
use crate::util::accountable::Accountable;
use crate::util::array_util::ArrayUtil;
use crate::util::error::lucene_error::Result;
use crate::util::packed::packed_long_values::{PackedLongValues, PackedLongValuesBuilder};
use crate::util::packed::PackedInts;
use crate::util::{Counter, CounterEnumBorrow};
use std::rc::Rc;
/// Buffers up pending `[i64]` per doc, sorts, then flushes when segment flushes.
pub(crate) struct SortedNumericDocValuesWriter {
    pending: PackedLongValuesBuilder, // stream of all values
    pending_counts: Option<PackedLongValuesBuilder>, // count of values per doc
    docs_with_field: DocsWithFieldSet,
    iw_bytes_used: CounterEnumBorrow,
    bytes_used: i64, // this only tracks differences in 'pending' and 'pendingCounts'
    field_info: Rc<FieldInfo>,
    current_doc: i32,
    current_values: Vec<i64>,
    current_upto: usize,

    final_values: Option<PackedLongValues>,
    final_values_count: Option<PackedLongValues>,
}

impl SortedNumericDocValuesWriter {
    pub fn new(field_info: Rc<FieldInfo>, iw_bytes_used: CounterEnumBorrow) -> Result<Self> {
        let current_values = vec![0i64; 8];
        let docs_with_field = DocsWithFieldSet::new();
        let pending =
            PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?;

        // TODO:  memory calculation not implemented
        let bytes_used = pending.ram_bytes_used()? + docs_with_field.ram_bytes_used()?;

        iw_bytes_used.borrow_mut().add_and_get(bytes_used);

        Ok(Self {
            pending,
            pending_counts: None,
            docs_with_field,
            iw_bytes_used,
            bytes_used,
            field_info,
            current_doc: -1,
            current_values,
            current_upto: 0,
            final_values: None,
            final_values_count: None,
        })
    }

    pub fn add_value(&mut self, doc_id: i32, value: i64) -> Result<()> {
        debug_assert!(doc_id >= self.current_doc);
        if doc_id != self.current_doc {
            self.finish_current_doc()?;
            self.current_doc = doc_id;
        }
        self.add_one_value(value)?;
        self.update_bytes_used()?;
        Ok(())
    }
    // finalize currentDoc: this sorts the values in the current doc
    fn finish_current_doc(&mut self) -> Result<()> {
        if self.current_doc == -1 {
            return Ok(());
        }
        if self.current_upto > 1 {
            self.current_values[..self.current_upto].sort_unstable();
        }
        for i in 0..self.current_upto {
            self.pending.add(self.current_values[i])?;
        }
        // record the number of values for this doc
        if let Some(pending_counts) = self.pending_counts.as_mut() {
            pending_counts.add(self.current_upto as i64)?;
        } else if self.current_upto != 1 {
            let mut pending_counts =
                PackedLongValues::delta_packed_long_values_builder_default(PackedInts::COMPACT)?;
            for _ in 0..self.docs_with_field.cardinality() {
                pending_counts.add(1)?;
            }
            pending_counts.add(self.current_upto as i64)?;
            self.pending_counts = Some(pending_counts);
        }
        self.current_upto = 0;
        self.docs_with_field.add(self.current_doc)?;
        Ok(())
    }

    fn add_one_value(&mut self, value: i64) -> Result<()> {
        if self.current_upto == self.current_values.len() {
            let len = self.current_values.len();
            ArrayUtil::grow_with_len(&mut self.current_values, len + 1);
        }
        self.current_values[self.current_upto] = value;
        self.current_upto += 1;
        Ok(())
    }

    fn update_bytes_used(&mut self) -> Result<()> {
        let pending_counts_usage = match &self.pending_counts {
            Some(c) => c.ram_bytes_used()?,
            None => 0,
        };
        // TODO: memory calculation not implemented
        let new_bytes_used = self.pending.ram_bytes_used()?
            + pending_counts_usage
            + self.docs_with_field.ram_bytes_used()?;

        self.iw_bytes_used
            .borrow_mut()
            .add_and_get(new_bytes_used - self.bytes_used);
        self.bytes_used = new_bytes_used;
        Ok(())
    }
}

pub(crate) mod sndvw_util {}
