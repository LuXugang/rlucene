/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::fmt::Display;

use crate::index::doc_values_type::DocValuesType;
use crate::index::term::Term;
use crate::index::BytesRef;
use crate::store::DataOutput;
use crate::util::error::lucene_error::Result;

/// An in-place update to a DocValues field.
#[derive(Clone)]
pub struct DocValuesUpdate {
    pub(crate) doc_values_type: DocValuesType,
    pub term: Term,
    pub field: String,
    // used in BufferedDeletes to apply this update only to a slice of docs.
    // It's initialized to BufferedUpdates.MAX_INT
    // since it's safe and most often used this way we save object creations.
    pub doc_id_up_to: i32,
    pub has_value: bool,
    pub sub_update: DocValuesUpdateEnum,
}
impl DocValuesUpdate {
    #[allow(unused)]
    const RAW_SIZE_IN_BYTES: i32 = 0;
    pub fn new(
        doc_values_type: DocValuesType,
        term: Term,
        field: String,
        doc_id_up_to: i32,
        sub_update: DocValuesUpdateEnum,
    ) -> Self {
        debug_assert!(doc_id_up_to >= 0, "{doc_id_up_to} must be >= 0");
        let has_value = sub_update.has_value();
        DocValuesUpdate {
            doc_values_type,
            term,
            field,
            doc_id_up_to,
            has_value,
            sub_update,
        }
    }

    #[allow(unused)]
    pub(crate) fn has_value(&self) -> bool {
        self.has_value
    }
    #[allow(dead_code)]
    fn size_in_bytes(&self) -> i32 {
        unimplemented!("Not used in Java Lucene, so we did not implement it")
    }
    #[cfg(feature = "test_only")]
    pub fn prepare_for_apply(&mut self, doc_id_upto: i32) -> Option<DocValuesUpdate> {
        if doc_id_upto == self.doc_id_up_to {
            return None;
        }
        let sub_update = self.sub_update.prepare_for_apply();
        Some(DocValuesUpdate::new(
            self.doc_values_type,
            self.term.clone(),
            self.field.clone(),
            doc_id_upto,
            sub_update,
        ))
    }
}
impl Display for DocValuesUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "term={}, field={}, value={}, docIDUpTo={}",
            self.term,
            self.field,
            self.sub_update.value_to_string(),
            self.doc_id_up_to
        )
    }
}
pub trait DocValuesUpdateBase {
    #[allow(dead_code)]
    fn value_size_in_bytes(&self) -> i64 {
        unimplemented!("Not used in Java Lucene, so we did not implement it")
    }
    fn value_to_string(&self) -> String;
    #[allow(dead_code)]
    fn write_to<D: DataOutput>(&self, _bytes: &mut BytesRef<Vec<u8>>) -> Result<()> {
        unimplemented!("Not used in Java Lucene, so we did not implement it")
    }
    fn has_value(&self) -> bool;
    #[cfg(feature = "test_only")]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum;
}
/// An in-place update to a binary DocValues field.
#[derive(Clone)]
pub struct BinaryDocValuesUpdate {
    value: Option<BytesRef<Vec<u8>>>,
}
impl BinaryDocValuesUpdate {
    #[allow(unused)]
    const RAW_VALUE_SIZE_IN_BYTES: i32 = 0;
    pub fn new(value: Option<BytesRef<Vec<u8>>>) -> Self {
        BinaryDocValuesUpdate { value }
    }
    pub fn get_value(&self) -> BytesRef<Vec<u8>> {
        debug_assert!(self.value.is_some());
        self.value.as_ref().unwrap().clone()
    }
}
impl DocValuesUpdateBase for BinaryDocValuesUpdate {
    fn value_to_string(&self) -> String {
        match &self.value {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        }
    }

    fn has_value(&self) -> bool {
        self.value.is_some()
    }

    #[cfg(feature = "test_only")]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
        DocValuesUpdateEnum::Binary(BinaryDocValuesUpdate::new(self.value.clone()))
    }
}
#[derive(Clone)]
pub struct NumericDocValuesUpdate {
    value: Option<i64>,
}
impl NumericDocValuesUpdate {
    pub fn new(value: Option<i64>) -> Self {
        NumericDocValuesUpdate { value }
    }
    pub fn get_value(&self) -> i64 {
        debug_assert!(
            self.value.is_some(),
            "getValue should only be called if this update has a value"
        );
        *self.value.as_ref().unwrap()
    }
}
impl DocValuesUpdateBase for NumericDocValuesUpdate {
    fn value_to_string(&self) -> String {
        match self.value {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        }
    }

    fn has_value(&self) -> bool {
        self.value.is_some()
    }

    #[cfg(feature = "test_only")]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
        DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(self.value))
    }
}

#[derive(Clone)]
pub enum DocValuesUpdateEnum {
    Binary(BinaryDocValuesUpdate),
    Numeric(NumericDocValuesUpdate),
}
impl DocValuesUpdateEnum {
    pub fn get_binary(&self) -> Option<&BinaryDocValuesUpdate> {
        debug_assert!(matches!(self, DocValuesUpdateEnum::Binary(_)));
        match self {
            DocValuesUpdateEnum::Binary(ref b) => Some(b),
            _ => None,
        }
    }

    pub fn get_numeric(&self) -> Option<&NumericDocValuesUpdate> {
        debug_assert!(matches!(self, DocValuesUpdateEnum::Numeric(_)));
        match self {
            DocValuesUpdateEnum::Numeric(ref n) => Some(n),
            _ => None,
        }
    }
}
impl DocValuesUpdateBase for DocValuesUpdateEnum {
    fn value_to_string(&self) -> String {
        match self {
            DocValuesUpdateEnum::Binary(b) => b.value_to_string(),
            DocValuesUpdateEnum::Numeric(n) => n.value_to_string(),
        }
    }

    fn has_value(&self) -> bool {
        match self {
            DocValuesUpdateEnum::Binary(b) => b.has_value(),
            DocValuesUpdateEnum::Numeric(n) => n.has_value(),
        }
    }

    #[cfg(feature = "test_only")]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
        match self {
            DocValuesUpdateEnum::Binary(b) => b.prepare_for_apply(),
            DocValuesUpdateEnum::Numeric(n) => n.prepare_for_apply(),
        }
    }
}
