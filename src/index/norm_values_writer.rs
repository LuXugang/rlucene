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
use crate::index::docs_with_field_set::DocsWithFieldSetEnum;
use crate::util::bit_set::BitSet;
use crate::util::error::lucene_error::Result;
use crate::util::packed::packed_long_values::{PackedLongValues, PackedLongValuesIterator};

pub struct NormValuesWriter;

struct BufferedNorms<'a> {
    iter: PackedLongValuesIterator<'a>,
    doc_with_field: DocsWithFieldSetEnum<'a>,
}
impl<'a> BufferedNorms<'a> {
    pub(crate) fn new(
        values: &'a PackedLongValues,
        doc_with_field: DocsWithFieldSetEnum<'a>,
    ) -> Result<Self> {
        Ok(Self {
            iter: values.iterator()?,
            doc_with_field,
        })
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
    fn advance(&self, target: i32) -> i32 {
        if let Some(bits) = &self.docs_with_field {
            bits.next_set_bit(target)
        } else {
            // Only called when target is less than maxDoc
            target
        }
    }
    fn cost(&self) -> i64 {
        match &self.docs_with_field {
            Some(bits) => bits.cardinality() as i64,
            None => self.max_doc as i64,
        }
    }
}
