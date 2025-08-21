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
use crate::codecs::lucene90::lucene90_doc_values_producer::{
    Lucene90BinaryDocValuesEnum, Lucene90NumericDocValuesEnum, Lucene90SortedNumericDocValuesEnum,
};
use crate::codecs::lucene90_doc_values_producer::{
    BaseSortedDocValues, BaseSortedSetDocValues, DocValuesSkipperImpl, Lucene90DocValuesProducer,
};
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values_skipper::DocValuesSkipper;
use crate::index::field_info::FieldInfo;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::sorted_set_doc_values_writer::Either2SortedSetDocValues;
use crate::store::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

/// A trait that produces numeric, binary, sorted, sorted set, and sorted
/// numeric doc values.
pub trait DocValuesProducer {
    type NumericDocValues: NumericDocValues;
    /// Returns [`NumericDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not [`DocValuesType::NUMERIC`](crate::index::doc_values_type::DocValuesType::Numeric).
    fn get_numeric(&self, _field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type BinaryDocValues: BinaryDocValues;
    /// Returns [`BinaryDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not
    /// [`DocValuesType::BINARY`](crate::index::doc_values_type::DocValuesType::Binary).
    /// The return value is never `null`.
    fn get_binary(&self, _field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type SortedDocValues: SortedDocValues;
    /// Returns [`SortedDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not
    /// [`DocValuesType::SORTED`](crate::index::doc_values_type::DocValuesType::Sorted).
    /// The return value is never `null`.
    fn get_sorted(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type SortedNumericDocValues: SortedNumericDocValues;
    /// Returns [`SortedNumericDocValues`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The behavior is undefined if the doc values type of the given field
    /// is not [`DocValuesType::SORTED_NUMERIC`](crate::index::doc_values_type::DocValuesType::SortedNumeric).
    /// The return value is never `null`.
    fn get_sorted_numeric(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
        Err(LuceneError::need_implemented(""))
    }

    type SortedSetDocValues: SortedSetDocValues;
    /// Returns [`SortedSetDocValues`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The behavior is undefined if the doc values type of the given field
    /// is not
    /// [`DocValuesType::SORTED_SET`](crate::index::doc_values_type::DocValuesType::SortedSet).
    /// The return value is never `null`.
    fn get_sorted_set(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type DocValuesSkipper: DocValuesSkipper;
    /// Returns a [`DocValuesSkipper`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The return value is undefined if
    /// [`FieldInfo::doc_values_skip_index_type()`](FieldInfo::doc_values_skip_index_type) returns
    /// [`DocValuesSkipIndexType::NONE`](crate::index::doc_values_skip_index_type::DocValuesSkipIndexType::None).
    fn get_skipper(&self, _field: &Arc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        Err(LuceneError::need_implemented(""))
    }
    /// Checks consistency of this producer.
    ///
    /// Note that this may be costly in terms of I/O, e.g. it may involve
    /// computing a checksum value against large data files.
    fn check_integrity(&self) -> Result<()> {
        Err(LuceneError::need_implemented(""))
    }
    /// Returns an instance optimized for merging. This instance may only be consumed in the thread
    /// that called [`get_merge_instance()`](DocValuesProducer::get_merge_instance).
    /// The default implementation returns `self`.
    /// # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        Ok(None)
    }
}

pub enum DocValuesProducerEnum<I>
where
    I: IndexInput,
{
    Lucene90(Lucene90DocValuesProducer<I>),
}
impl<I> DocValuesProducer for DocValuesProducerEnum<I>
where
    I: IndexInput,
{
    type NumericDocValues = Lucene90NumericDocValuesEnum<I>;

    fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_numeric(field),
        }
    }

    type BinaryDocValues = Lucene90BinaryDocValuesEnum<I>;

    fn get_binary(&self, _field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_binary(_field),
        }
    }

    type SortedDocValues = BaseSortedDocValues<I>;

    fn get_sorted(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_sorted(_field),
        }
    }

    type SortedNumericDocValues = Lucene90SortedNumericDocValuesEnum<I>;

    fn get_sorted_numeric(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_sorted_numeric(_field),
        }
    }

    type SortedSetDocValues = Either2SortedSetDocValues<
        SingletonSortedSetDocValues<BaseSortedDocValues<I>>,
        BaseSortedSetDocValues<I>,
    >;

    fn get_sorted_set(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_sorted_set(_field),
        }
    }

    type DocValuesSkipper = DocValuesSkipperImpl<I>;

    fn get_skipper(&self, _field: &Arc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_skipper(_field),
        }
    }

    fn check_integrity(&self) -> Result<()> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.check_integrity(),
        }
    }

    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => {
                let merge_instance = lucene90.get_merge_instance()?;
                if let Some(instance) = merge_instance {
                    Ok(Some(DocValuesProducerEnum::Lucene90(instance)))
                } else {
                    Ok(None)
                }
            },
        }
    }
}
