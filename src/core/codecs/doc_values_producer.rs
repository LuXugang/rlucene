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
use crate::core::codecs::lucene90::lucene90_doc_values_producer::{
    Lucene90BinaryDocValuesEnum, Lucene90NumericDocValuesEnum, Lucene90SortedNumericDocValuesEnum,
};
use crate::core::codecs::lucene90_doc_values_producer::{
    BaseSortedDocValues, BaseSortedSetDocValues, DocValuesSkipperImpl, Lucene90DocValuesProducer,
};
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::binary_doc_values::Either2BinaryDocValues;
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::index::doc_values_skipper::Either2DocValuesSkipper;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::numeric_doc_values::Either2NumericDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::core::index::sorted_doc_values::Either2SortedDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_numeric_doc_values::Either2SortedNumericDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_writer::Either2SortedSetDocValues;
use crate::core::store::IndexInput;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::rc::Rc;
use std::sync::Arc;

/// A trait that produces numeric, binary, sorted, sorted set, and sorted
/// numeric doc values.
pub trait DocValuesProducer: Clone {
    type NumericDocValues: NumericDocValues;
    /// Returns [`NumericDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not [`DocValuesType::NUMERIC`](crate::core::index::doc_values_type::DocValuesType::Numeric).
    fn get_numeric(&self, _field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type BinaryDocValues: BinaryDocValues;
    /// Returns [`BinaryDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not
    /// [`DocValuesType::BINARY`](crate::core::index::doc_values_type::DocValuesType::Binary).
    /// The return value is never `null`.
    fn get_binary(&self, _field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type SortedDocValues: SortedDocValues;
    /// Returns [`SortedDocValues`] for this field. The returned instance need
    /// not be thread-safe: it will only be used by a single thread. The
    /// behavior is undefined if the doc values type of the given field is
    /// not
    /// [`DocValuesType::SORTED`](crate::core::index::doc_values_type::DocValuesType::Sorted).
    /// The return value is never `null`.
    fn get_sorted(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type SortedNumericDocValues: SortedNumericDocValues;
    /// Returns [`SortedNumericDocValues`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The behavior is undefined if the doc values type of the given field
    /// is not [`DocValuesType::SORTED_NUMERIC`](crate::core::index::doc_values_type::DocValuesType::SortedNumeric).
    /// The return value is never `null`.
    fn get_sorted_numeric(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
        Err(LuceneError::need_implemented(""))
    }

    type SortedSetDocValues: SortedSetDocValues;
    /// Returns [`SortedSetDocValues`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The behavior is undefined if the doc values type of the given field
    /// is not
    /// [`DocValuesType::SORTED_SET`](crate::core::index::doc_values_type::DocValuesType::SortedSet).
    /// The return value is never `null`.
    fn get_sorted_set(&self, _field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        Err(LuceneError::need_implemented(""))
    }
    type DocValuesSkipper: DocValuesSkipper;
    /// Returns a [`DocValuesSkipper`] for this field. The returned instance
    /// need not be thread-safe: it will only be used by a single thread.
    /// The return value is undefined if
    /// [`FieldInfo::doc_values_skip_index_type()`](FieldInfo::doc_values_skip_index_type) returns
    /// [`DocValuesSkipIndexType::NONE`](crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType::None).
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

impl<I> Clone for DocValuesProducerEnum<I>
where
    I: IndexInput,
{
    fn clone(&self) -> Self {
        unreachable!(
            "DocValuesProducerEnum does not implement the Clone logic.
The purpose of implementing the Clone trait is to make it could be used with Cow"
        )
    }
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
impl<T> DocValuesProducer for Rc<T>
where
    T: DocValuesProducer,
{
    type NumericDocValues = T::NumericDocValues;
    fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        (**self).get_numeric(field)
    }
    type BinaryDocValues = T::BinaryDocValues;
    fn get_binary(&self, field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        (**self).get_binary(field)
    }
    type SortedDocValues = T::SortedDocValues;
    fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
        (**self).get_sorted(field)
    }

    type SortedNumericDocValues = T::SortedNumericDocValues;

    fn get_sorted_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
        (**self).get_sorted_numeric(field)
    }

    type SortedSetDocValues = T::SortedSetDocValues;

    fn get_sorted_set(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        (**self).get_sorted_set(field)
    }

    type DocValuesSkipper = T::DocValuesSkipper;

    fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        (**self).get_skipper(field)
    }

    fn check_integrity(&self) -> Result<()> {
        (**self).check_integrity()
    }

    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        let v = match (**self).get_merge_instance()? {
            Some(v) => Rc::new(v),
            None => return Ok(None),
        };
        Ok(Some(v))
    }
}
macro_rules! either_docvaluesproducer {
    ($vis:vis $name:ident { A: $A:ident, B: $B:ident }) => {
        #[derive(Clone)]
        $vis enum $name<$A, $B> {
            A($A),
            B($B),
        }

        impl<$A, $B> DocValuesProducer for $name<$A, $B>
        where
            $A: DocValuesProducer,
            $B: DocValuesProducer,
        {
            type NumericDocValues =
                Either2NumericDocValues<$A::NumericDocValues, $B::NumericDocValues>;

            fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
                match self {
                    $name::A(inner) => inner.get_numeric(field).map(Either2NumericDocValues::A),
                    $name::B(inner) => inner.get_numeric(field).map(Either2NumericDocValues::B),
                }
            }

            type BinaryDocValues =
                Either2BinaryDocValues<$A::BinaryDocValues, $B::BinaryDocValues>;

            fn get_binary(&self, field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
                match self {
                    $name::A(inner) => inner.get_binary(field).map(Either2BinaryDocValues::A),
                    $name::B(inner) => inner.get_binary(field).map(Either2BinaryDocValues::B),
                }
            }

            type SortedDocValues =
                Either2SortedDocValues<$A::SortedDocValues, $B::SortedDocValues>;

            fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
                match self {
                    $name::A(inner) => inner.get_sorted(field).map(Either2SortedDocValues::A),
                    $name::B(inner) => inner.get_sorted(field).map(Either2SortedDocValues::B),
                }
            }

            type SortedNumericDocValues = Either2SortedNumericDocValues<
                $A::SortedNumericDocValues,
                $B::SortedNumericDocValues,
            >;

            fn get_sorted_numeric(
                &self,
                field: &Arc<FieldInfo>,
            ) -> Result<Self::SortedNumericDocValues> {
                match self {
                    $name::A(inner) => inner
                        .get_sorted_numeric(field)
                        .map(Either2SortedNumericDocValues::A),
                    $name::B(inner) => inner
                        .get_sorted_numeric(field)
                        .map(Either2SortedNumericDocValues::B),
                }
            }

            type SortedSetDocValues =
                Either2SortedSetDocValues<$A::SortedSetDocValues, $B::SortedSetDocValues>;

            fn get_sorted_set(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
                match self {
                    $name::A(inner) => inner.get_sorted_set(field).map(Either2SortedSetDocValues::A),
                    $name::B(inner) => inner.get_sorted_set(field).map(Either2SortedSetDocValues::B),
                }
            }

            type DocValuesSkipper =
                Either2DocValuesSkipper<$A::DocValuesSkipper, $B::DocValuesSkipper>;

            fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
                match self {
                    $name::A(inner) => inner.get_skipper(field).map(Either2DocValuesSkipper::A),
                    $name::B(inner) => inner.get_skipper(field).map(Either2DocValuesSkipper::B),
                }
            }

            fn check_integrity(&self) -> Result<()> {
                match self {
                    $name::A(inner) => inner.check_integrity(),
                    $name::B(inner) => inner.check_integrity(),
                }
            }

            fn get_merge_instance(&self) -> Result<Option<Self>>
            where
                Self: Sized,
            {
                match self {
                    $name::A(inner) => match inner.get_merge_instance()? {
                        Some(instance) => Ok(Some($name::A(instance))),
                        None => Ok(None),
                    },
                    $name::B(inner) => match inner.get_merge_instance()? {
                        Some(instance) => Ok(Some($name::B(instance))),
                        None => Ok(None),
                    },
                }
            }
        }
    };
}
either_docvaluesproducer!(pub Either2DocValuesProducer { A: A, B: B });
