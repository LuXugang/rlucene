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
use crate::codecs::doc_values_enum::doc_values::{
    BinaryDocValuesEnum, DocValuesSkipperEnum, NumericDocValuesEnum, SortedDocValuesEnum,
    SortedNumericDocValuesEnum, SortedSetDocValuesEnum,
};
use crate::index::field_info::FieldInfo;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
/// A trait that produces numeric, binary, sorted, sorted set, and sorted numeric doc values.
pub trait DocValuesProducer<I>
where
    I: IndexInput,
{
    /// Returns [`NumericDocValues`](crate::index::numeric_doc_values::NumericDocValues) for this field. The returned instance need not be thread-safe:
    /// it will only be used by a single thread. The behavior is undefined if the doc values type of
    /// the given field is not [`DocValuesType::NUMERIC`](crate::index::doc_values_type::DocValuesType::Numeric).
    fn get_numeric(&mut self, field: &FieldInfo) -> Result<NumericDocValuesEnum<I>>;
    /// Returns [`BinaryDocValues`](crate::index::binary_doc_values::BinaryDocValues) for this field. The returned instance need not be thread-safe:
    /// it will only be used by a single thread. The behavior is undefined if the doc values type of
    /// the given field is not [`DocValuesType::BINARY`](crate::index::doc_values_type::DocValuesType::Binary). The return value is never `null`.
    fn get_binary(&mut self, field: &FieldInfo) -> Result<BinaryDocValuesEnum<I>>;

    /// Returns [`SortedDocValues`](crate::index::sorted_doc_values::SortedDocValues) for this field. The returned instance need not be thread-safe:
    /// it will only be used by a single thread. The behavior is undefined if the doc values type of
    /// the given field is not [`DocValuesType::SORTED`](crate::index::doc_values_type::DocValuesType::Sorted). The return value is never `null`.
    fn get_sorted(&mut self, field: &FieldInfo) -> Result<SortedDocValuesEnum<I>>;
    /// Returns [`SortedNumericDocValues`](crate::index::sorted_numeric_doc_values::SortedNumericDocValues) for this field. The returned instance need not be
    /// thread-safe: it will only be used by a single thread. The behavior is undefined if the doc
    /// values type of the given field is not [`DocValuesType::SORTED_NUMERIC`](crate::index::doc_values_type::DocValuesType::SortedNumeric). The return value is
    /// never `null`.
    fn get_sorted_numeric(&mut self, field: &FieldInfo) -> Result<SortedNumericDocValuesEnum<I>>;

    /// Returns [`SortedSetDocValues`](crate::index::sorted_set_doc_values::SortedSetDocValues) for this field. The returned instance need not be
    /// thread-safe: it will only be used by a single thread. The behavior is undefined if the doc
    /// values type of the given field is not [`DocValuesType::SORTED_SET`](crate::index::doc_values_type::DocValuesType::SortedSet). The return value is
    /// never `null`.
    fn get_sorted_set(&mut self, field: &FieldInfo) -> Result<SortedSetDocValuesEnum<I>>;
    /// Returns a [`DocValuesSkipper`](crate::index::doc_values_skipper::DocValuesSkipper) for this field. The returned instance need not be
    /// thread-safe: it will only be used by a single thread. The return value is undefined if
    /// [`FieldInfo::doc_values_skip_index_type()`](FieldInfo::doc_values_skip_index_type) returns
    /// [`DocValuesSkipIndexType::NONE`](crate::index::doc_values_skip_index_type::DocValuesSkipIndexType::None).
    fn get_skipper(&mut self, field: &FieldInfo) -> Result<DocValuesSkipperEnum<I>>;
    /// Checks consistency of this producer.
    ///
    /// Note that this may be costly in terms of I/O, e.g. it may involve computing a checksum value
    /// against large data files.
    fn check_integrity(&mut self) -> Result<()>;

    /// Returns an instance optimized for merging. This instance may only be consumed in the thread
    /// that called [`get_merge_instance()`](DocValuesProducer::get_merge_instance).
    ///
    /// The default implementation returns `self`.
    /// # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&mut self) -> Option<DocValuesProducerEnum>;
}
pub enum DocValuesProducerEnum {}
