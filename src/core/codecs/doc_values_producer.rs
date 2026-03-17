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
use crate::core::codecs::DefaultDocValuesFormat;
use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::binary_doc_values::BinaryDocValuesEnum2;
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::index::doc_values_skipper::DocValuesSkipperEnum2;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::numeric_doc_values::NumericDocValuesEnum2;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_doc_values::SortedDocValuesEnum2;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValuesEnum2;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_writer::SortedSetDocValuesEnum2;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::sync::Arc;

/// A trait that produces numeric, binary, sorted, sorted set, and sorted
/// numeric doc values.
pub trait DocValuesProducer {
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
  fn get_skipper(&self, _field: &Arc<FieldInfo>) -> Result<Option<Self::DocValuesSkipper>> {
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
pub type DefaultDocValuesProducer<I> =
  <DefaultDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>;

pub type DefaultBinary<I> = <DefaultDocValuesProducer<I> as DocValuesProducer>::BinaryDocValues;
pub type DefaultNumeric<I> = <DefaultDocValuesProducer<I> as DocValuesProducer>::NumericDocValues;
pub type DefaultSorted<I> = <DefaultDocValuesProducer<I> as DocValuesProducer>::SortedDocValues;
pub type DefaultSortedNumeric<I> =
  <DefaultDocValuesProducer<I> as DocValuesProducer>::SortedNumericDocValues;
pub type DefaultSortedSet<I> =
  <DefaultDocValuesProducer<I> as DocValuesProducer>::SortedSetDocValues;
pub type DefaultSkipper<I> = <DefaultDocValuesProducer<I> as DocValuesProducer>::DocValuesSkipper;

impl<T> DocValuesProducer for Arc<T>
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

  fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Option<Self::DocValuesSkipper>> {
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
      Some(v) => Arc::new(v),
      None => return Ok(None),
    };
    Ok(Some(v))
  }
}
macro_rules! either_docvaluesproducer {
    ($vis:vis $name:ident { A: $A:ident, B: $B:ident }) => {
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
                NumericDocValuesEnum2<$A::NumericDocValues, $B::NumericDocValues>;

            fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
                match self {
                    $name::A(inner) => inner.get_numeric(field).map(NumericDocValuesEnum2::A),
                    $name::B(inner) => inner.get_numeric(field).map(NumericDocValuesEnum2::B),
                }
            }

            type BinaryDocValues =
                BinaryDocValuesEnum2<$A::BinaryDocValues, $B::BinaryDocValues>;

            fn get_binary(&self, field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
                match self {
                    $name::A(inner) => inner.get_binary(field).map(BinaryDocValuesEnum2::A),
                    $name::B(inner) => inner.get_binary(field).map(BinaryDocValuesEnum2::B),
                }
            }

            type SortedDocValues =
                SortedDocValuesEnum2<$A::SortedDocValues, $B::SortedDocValues>;

            fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
                match self {
                    $name::A(inner) => inner.get_sorted(field).map(SortedDocValuesEnum2::A),
                    $name::B(inner) => inner.get_sorted(field).map(SortedDocValuesEnum2::B),
                }
            }

            type SortedNumericDocValues = SortedNumericDocValuesEnum2<
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
                        .map(SortedNumericDocValuesEnum2::A),
                    $name::B(inner) => inner
                        .get_sorted_numeric(field)
                        .map(SortedNumericDocValuesEnum2::B),
                }
            }

            type SortedSetDocValues =
                SortedSetDocValuesEnum2<$A::SortedSetDocValues, $B::SortedSetDocValues>;

            fn get_sorted_set(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
                match self {
                    $name::A(inner) => inner.get_sorted_set(field).map(SortedSetDocValuesEnum2::A),
                    $name::B(inner) => inner.get_sorted_set(field).map(SortedSetDocValuesEnum2::B),
                }
            }

            type DocValuesSkipper =
                DocValuesSkipperEnum2<$A::DocValuesSkipper, $B::DocValuesSkipper>;

            fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Option<Self::DocValuesSkipper>> {
                match self {
                    $name::A(inner) => inner.get_skipper(field).map(|opt| opt.map(DocValuesSkipperEnum2::A)),
                    $name::B(inner) => inner.get_skipper(field).map(|opt| opt.map(DocValuesSkipperEnum2::B)),
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
either_docvaluesproducer!(pub DocValuesProducerEnum2 { A: A, B: B });
