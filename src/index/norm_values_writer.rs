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
use crate::index::docs_with_field_set::DocsWithFieldSetEnum;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::packed::packed_long_values::{PackedLongValues, PackedLongValuesIterator};
pub struct NormValuesWriter;

struct BufferedNorms<'a> {
    iter: PackedLongValuesIterator<'a>,
    doc_with_field: DocsWithFieldSetEnum<'a>,
    value: i64,
}
impl<'a> BufferedNorms<'a> {
    pub(crate) fn new(
        values: &'a PackedLongValues,
        doc_with_field: DocsWithFieldSetEnum<'a>,
    ) -> Result<Self> {
        Ok(Self {
            iter: values.iterator()?,
            doc_with_field,
            value: 0,
        })
    }
}

impl DocValuesIterator for BufferedNorms<'_> {
    fn advance_exact(&mut self, _target: i32) -> Result<bool> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl DocIdSetIterator for BufferedNorms<'_> {
    fn doc_id(&self) -> i32 {
        self.doc_with_field.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        let doc_id = self.doc_with_field.next_doc()?;
        if doc_id != NO_MORE_DOCS {
            self.value = self.iter.next_value()?;
        }
        Ok(doc_id)
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn cost(&self) -> Result<i64> {
        self.doc_with_field.cost()
    }
}

impl<'a> NumericDocValues for BufferedNorms<'a> {
    fn long_value(&mut self) -> Result<i64> {
        Ok(self.value)
    }
}
