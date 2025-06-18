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
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;
use crate::util::error::lucene_error;
use crate::util::error::lucene_error::LuceneError;
use std::cell::Cell;

pub(crate) struct NumericDocValuesWriter;

pub mod ndvw_util {
    use crate::index::numeric_doc_values::NumericDocValues;
    use crate::index::numeric_doc_values_writer::NumericDVs;
    use crate::index::sorter::DocMap;
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::util::bit_set::BitSet;
    use crate::util::error::lucene_error::Result;
    use crate::util::fixed_bit_set::FixedBitSet;

    pub(crate) fn sort_doc_values<DV, M>(
        max_doc: i32,
        sort_map: &M,
        old_doc_values: &mut DV,
        dense: bool,
    ) -> Result<NumericDVs<FixedBitSet>>
    where
        DV: NumericDocValues,
        M: DocMap,
    {
        let mut docs_with_field = if !dense {
            Some(FixedBitSet::new(max_doc))
        } else {
            None
        };

        let mut values = vec![0i64; max_doc as usize];

        loop {
            let doc_id = old_doc_values.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }

            let new_doc_id = sort_map.old_to_new(doc_id);
            if let Some(bits) = &mut docs_with_field {
                bits.set(new_doc_id);
            }

            values[new_doc_id as usize] = old_doc_values.long_value()?;
        }
        Ok(NumericDVs::new(values, docs_with_field))
    }
}

pub(crate) struct SortingNumericDocValues<T>
where
    T: BitSet,
{
    dvs: NumericDVs<T>,
    doc_id: i32,
    cost: Cell<i64>,
}

impl<T> SortingNumericDocValues<T>
where
    T: BitSet,
{
    pub(crate) fn new(dvs: NumericDVs<T>) -> Self {
        Self {
            dvs,
            doc_id: -1,
            cost: Cell::new(-1),
        }
    }
}

impl<T> DocValuesIterator for SortingNumericDocValues<T> where T: BitSet {}

impl<T> DocIdSetIterator for SortingNumericDocValues<T>
where
    T: BitSet,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> lucene_error::Result<i32> {
        if self.doc_id + 1 == self.dvs.max_doc() {
            self.doc_id = NO_MORE_DOCS;
        } else {
            self.doc_id = self.dvs.advance(self.doc_id + 1);
        }
        Ok(self.doc_id)
    }

    fn advance(&mut self, _target: i32) -> lucene_error::Result<i32> {
        Err(LuceneError::unsupported_operation("use nextDoc() instead"))
    }

    fn cost(&self) -> lucene_error::Result<i64> {
        if self.cost.get() == -1 {
            self.cost.set(self.dvs.cost());
        }
        Ok(self.cost.get())
    }
}

impl<T> NumericDocValues for SortingNumericDocValues<T>
where
    T: BitSet,
{
    fn long_value(&mut self) -> lucene_error::Result<i64> {
        Ok(self.dvs.values[self.doc_id as usize])
    }
}
pub(crate) struct NumericDVs<T>
where
    T: BitSet,
{
    pub values: Vec<i64>,
    pub docs_with_field: Option<T>,
    pub max_doc: i32,
}

impl<T> NumericDVs<T>
where
    T: BitSet,
{
    pub fn new(values: Vec<i64>, docs_with_field: Option<T>) -> Self {
        debug_assert!(values.len() <= i32::MAX as usize);
        let max_doc = values.len() as i32;
        Self {
            values,
            docs_with_field,
            max_doc,
        }
    }

    pub(crate) fn max_doc(&self) -> i32 {
        self.max_doc
    }

    fn advance_exact(&self, target: i32) -> bool {
        match &self.docs_with_field {
            Some(bits) => bits.get(target),
            None => true,
        }
    }
    pub(crate) fn advance(&self, target: i32) -> i32 {
        if let Some(bits) = &self.docs_with_field {
            bits.next_set_bit(target)
        } else {
            // Only called when target is less than maxDoc
            target
        }
    }
    pub(crate) fn cost(&self) -> i64 {
        match &self.docs_with_field {
            Some(bits) => bits.cardinality() as i64,
            None => self.max_doc as i64,
        }
    }
}
