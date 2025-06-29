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
use std::rc::Rc;

use crate::codecs::lucene90::lucene90_doc_values_enums::Lucene90NumericDocValuesEnums;
use crate::codecs::lucene90_doc_values_enums::{
    Lucene90BinaryDocValuesEnum, Lucene90SortedNumericDocValuesEnums,
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
use crate::store::IndexInput;
use crate::util::either_enums::EitherSortedSetDocValues;
use crate::util::error::lucene_error::{LuceneError, Result};

/// A trait that produces numeric, binary, sorted, sorted set, and sorted
/// numeric doc values.
pub trait DocValuesProducer {
    type NumericDocValues: NumericDocValues;
    /// Returns [`NumericDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not [`DocValuesType::NUMERIC`](crate::index::doc_values_type::DocValuesType::Numeric).
    fn get_numeric(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::NumericDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type BinaryDocValues: BinaryDocValues;
    /// Returns [`BinaryDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not
    /// [`DocValuesType::BINARY`](crate::index::doc_values_type::DocValuesType::Binary).
    /// The return value is never `null`.
    fn get_binary(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type SortedDocValues: SortedDocValues;
    /// Returns [`SortedDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not
    /// [`DocValuesType::SORTED`](crate::index::doc_values_type::DocValuesType::Sorted).
    /// The return value is never `null`.
    fn get_sorted(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::SortedDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type SortedNumericDocValues: SortedNumericDocValues;
    /// Returns [`SortedNumericDocValues`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The behavior is undefined if the doc values type of the given field
    /// is not [`DocValuesType::SORTED_NUMERIC`](crate::index::doc_values_type::DocValuesType::SortedNumeric).
    /// The return value is never `null`.
    fn get_sorted_numeric(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> Result<Self::SortedNumericDocValues> {
        Err(LuceneError::need_implemented(""))
    }

    type SortedSetDocValues: SortedSetDocValues;
    /// Returns [`SortedSetDocValues`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The behavior is undefined if the doc values type of the given field
    /// is not
    /// [`DocValuesType::SORTED_SET`](crate::index::doc_values_type::DocValuesType::SortedSet).
    /// The return value is never `null`.
    fn get_sorted_set(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type DocValuesSkipper: DocValuesSkipper;
    /// Returns a [`DocValuesSkipper`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The return value is undefined if
    /// [`FieldInfo::doc_values_skip_index_type()`](FieldInfo::doc_values_skip_index_type) returns
    /// [`DocValuesSkipIndexType::NONE`](crate::index::doc_values_skip_index_type::DocValuesSkipIndexType::None).
    fn get_skipper(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        Err(LuceneError::need_implemented(""))
    }
    /// Checks consistency of this producer.
    ///
    /// Note that this may be costly in terms of I/O, e.g. it may involve
    /// computing a checksum value against large data files.
    fn check_integrity(&self) -> Result<()> {
        Err(LuceneError::need_implemented(""))
    }
}

// pub trait DocValuesProducerBase<I> where I: IndexInput {
//     type DocValuesProducer = DocValuesProducer<>
//     /// Returns an instance optimized for merging. This instance may only be
// consumed in the thread     /// that called
// [`get_merge_instance()`](DocValuesProducer::get_merge_instance).     ///
//     /// The default implementation returns `self`.
//     /// # Note
//     /// Returning None means returning itself.
//     fn get_merge_instance(&mut self) ->
// Result<Option<DocValuesProducerEnum<I>>> {         Ok(None)
//     }
// }
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
    type NumericDocValues = Lucene90NumericDocValuesEnums<I>;

    fn get_numeric(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::NumericDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_numeric(_field),
        }
    }

    type BinaryDocValues = Lucene90BinaryDocValuesEnum<I>;

    fn get_binary(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_binary(_field),
        }
    }

    type SortedDocValues = BaseSortedDocValues<I>;

    fn get_sorted(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::SortedDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_sorted(_field),
        }
    }

    type SortedNumericDocValues = Lucene90SortedNumericDocValuesEnums<I>;

    fn get_sorted_numeric(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> Result<Self::SortedNumericDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_sorted_numeric(_field),
        }
    }

    type SortedSetDocValues = EitherSortedSetDocValues<
        SingletonSortedSetDocValues<BaseSortedDocValues<I>>,
        BaseSortedSetDocValues<I>,
    >;

    fn get_sorted_set(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_sorted_set(_field),
        }
    }

    type DocValuesSkipper = DocValuesSkipperImpl<I>;

    fn get_skipper(&mut self, _field: &Rc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.get_skipper(_field),
        }
    }

    fn check_integrity(&self) -> Result<()> {
        match self {
            DocValuesProducerEnum::Lucene90(lucene90) => lucene90.check_integrity(),
        }
    }
}
