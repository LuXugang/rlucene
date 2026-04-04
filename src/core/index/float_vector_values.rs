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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::index::dummy::dummy_doc_index_iterator::DummyDocIndexIterator;
use crate::core::index::knn_vector_values::{BitsImpl1, DocIndexIteratorEnum2, KnnVectorValues};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::search::dummy::dummy_vector_scorer::DummyVectorScorer;
use crate::core::search::vector_scorer::VectorScorer;
use crate::core::search::vector_scorer::VectorScorerEnum2;
use crate::core::util::bits::Bits;
use crate::core::util::bits::BitsEnum2;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;

/// This class provides access to per-document floating point vector values indexed as KnnFloatVectorField
pub trait FloatVectorValues: KnnVectorValues {
  /// Return the vector value for the given vector ordinal which must be in [0, size() - 1],
  /// otherwise IndexOutOfBoundsException is thrown. The returned array may be shared across calls.
  ///
  /// # Returns
  /// the vector value
  fn vector_value(&self, ord: usize) -> Result<Cow<'_, VectorValueEnum>>;

  type FloatVectorValues: FloatVectorValues;
  /// Creates a copy of this [`KnnVectorValues`] when an independent instance is
  /// needed.
  ///
  /// This is useful when multiple vector values need to be accessed at the same
  /// time, in order to avoid overwriting the underlying vector buffer returned by
  /// a single instance.
  ///
  /// Returning `Some(...)` means that a new independent object was created.
  ///
  /// Returning `None` means that no new object was created, and callers should
  /// continue using `self` directly.
  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Err(LuceneError::unsupported_operation(""))
  }

  type VectorScorer: VectorScorer;
  fn scorer(&self, _target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_encoding(&self) -> VectorEncoding {
    VectorEncoding::FLOAT32(4)
  }

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    Err(LuceneError::unsupported_operation(""))
  }
}

#[macro_export]
macro_rules! either_float_vector_values {
    (
        $vis:vis $name:ident {
            iter = $iter_ty:ident,
            bits = $bits_ty:ident,
            scorer = $scorer_ty:ident;
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> $crate::core::index::knn_vector_values::KnnVectorValues for $name<$( $T ),+>
        where
            $( $T: $crate::core::index::float_vector_values::FloatVectorValues ),+
        {
            #[inline]
            fn dimension(&self) -> usize {
                match self { $( Self::$Variant(inner) => inner.dimension(), )+ }
            }

            #[inline]
            fn size(&self) -> usize {
                match self { $( Self::$Variant(inner) => inner.size(), )+ }
            }

            type KnnVectorValues =
                $crate::core::index::dummy::dummy_knn_vector_values::DummyKnnVectorsWriter;

            #[inline]
            fn get_encoding(&self) -> $crate::core::index::vector_encoding::VectorEncoding {
                match self {
                    $( Self::$Variant(inner) => $crate::core::index::knn_vector_values::KnnVectorValues::get_encoding(inner), )+
                }
            }

            type Bits<AcceptDocs> =
                $bits_ty<$( < $T as $crate::core::index::knn_vector_values::KnnVectorValues >::Bits<AcceptDocs> ),+>
            where
                AcceptDocs: $crate::core::util::bits::Bits;

            #[inline]
            fn get_accept_ords<AcceptDocs>(&self, accept_docs: Option<AcceptDocs>) -> Option<Self::Bits<AcceptDocs>>
            where
                AcceptDocs: $crate::core::util::bits::Bits,
            {
                match self {
                    $( Self::$Variant(inner) => inner.get_accept_ords(accept_docs).map($bits_ty::$Variant), )+
                }
            }

            type DocIndexIterator =
                $iter_ty<$( < $T as $crate::core::index::knn_vector_values::KnnVectorValues >::DocIndexIterator ),+>;

            #[inline]
            fn iterator(&mut self) -> $crate::core::util::error::lucene_error::Result<Self::DocIndexIterator> {
                match self {
                    $( Self::$Variant(inner) => inner.iterator().map($iter_ty::$Variant), )+
                }
            }
        }

        impl<$( $T ),+> $crate::core::index::float_vector_values::FloatVectorValues for $name<$( $T ),+>
        where
            $( $T: $crate::core::index::float_vector_values::FloatVectorValues ),+
        {
            #[inline]
            fn vector_value(&self, ord: usize) -> $crate::core::util::error::lucene_error::Result<std::borrow::Cow<'_, $crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>> {
                match self { $( Self::$Variant(inner) => inner.vector_value(ord), )+ }
            }

            type FloatVectorValues =
                $name<$( < $T as $crate::core::index::float_vector_values::FloatVectorValues >::FloatVectorValues ),+>;

            #[inline]
            fn float_copy(&self) -> $crate::core::util::error::lucene_error::Result<Option<Self::FloatVectorValues>> {
                match self {
                    $( Self::$Variant(inner) => inner.float_copy().map(|opt| opt.map($name::$Variant)), )+
                }
            }

            type VectorScorer =
                $scorer_ty<$( < $T as $crate::core::index::float_vector_values::FloatVectorValues >::VectorScorer ),+>;

            #[inline]
            fn scorer(&self, target: Vec<f32>) -> $crate::core::util::error::lucene_error::Result<Option<Self::VectorScorer>> {
                match self {
                    $( Self::$Variant(inner) => inner.scorer(target).map(|opt| opt.map($scorer_ty::$Variant)), )+
                }
            }

            #[inline]
            fn get_encoding(&self) -> $crate::core::index::vector_encoding::VectorEncoding {
                match self { $( Self::$Variant(inner) => $crate::core::index::float_vector_values::FloatVectorValues::get_encoding(inner), )+ }
            }

            #[inline]
            fn get_vectors_mut(&mut self) -> $crate::core::util::error::lucene_error::Result<&mut Vec<$crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>> {
                match self { $( Self::$Variant(inner) => inner.get_vectors_mut(), )+ }
            }

            #[inline]
            fn get_vectors(&self) -> $crate::core::util::error::lucene_error::Result<&[$crate::core::codecs::knn_field_vectors_writer::VectorValueEnum]> {
                match self { $( Self::$Variant(inner) => inner.get_vectors(), )+ }
            }
        }
    };
}

either_float_vector_values!(
    pub FloatVectorValuesEnum2 {
        iter = DocIndexIteratorEnum2,
        bits = BitsEnum2,
        scorer = VectorScorerEnum2;
        A: A, B: B,
    }
);

/// Checks the Vector Encoding of a field
pub fn check_field<LR: LeafReader>(reader: &LR, field: &str) -> Result<()> {
  if let Some(fi) = reader.get_field_infos()?.field_info_by_name(field)
    && fi.has_vector_values()
    && *fi.get_vector_encoding() != VectorEncoding::FLOAT32(4)
  {
    return Err(LuceneError::illegal_state(format!(
      "Unexpected vector encoding ({:?}) for field {}(expected={:?})",
      fi.get_vector_encoding(),
      field,
      VectorEncoding::FLOAT32(4)
    )));
  }
  Ok(())
}

/// Creates a [`FloatVectorValues`] from a list of float arrays.
///
/// # Arguments
/// * `vectors` - the list of float arrays
/// * `dim` - the dimension of the vectors
///
/// # Returns
/// a [`FloatVectorValues`] instance
pub fn from_floats(dim: usize) -> FloatVectorValuesImpl {
  FloatVectorValuesImpl::new(dim)
}

#[derive(Clone)] // TODO IMPORTANT CLone is Ok?
pub struct FloatVectorValuesImpl {
  vectors: Vec<VectorValueEnum>,
  dim: usize,
}

impl FloatVectorValuesImpl {
  pub(crate) fn new(dim: usize) -> Self {
    Self {
      vectors: Vec::new(),
      dim,
    }
  }
}

impl KnnVectorValues for FloatVectorValuesImpl {
  fn dimension(&self) -> usize {
    self.dim
  }

  fn size(&self) -> usize {
    self.vectors.len()
  }

  type KnnVectorValues = FloatVectorValuesImpl;

  fn get_encoding(&self) -> VectorEncoding {
    FloatVectorValues::get_encoding(self)
  }

  type Bits<B>
    = BitsImpl1<B>
  where
    B: Bits;

  fn get_accept_ords<B>(&self, accept_docs: Option<B>) -> Option<Self::Bits<B>>
  where
    B: Bits,
  {
    self.default_get_accept_ords(accept_docs)
  }

  type DocIndexIterator = DummyDocIndexIterator;
}

impl FloatVectorValues for FloatVectorValuesImpl {
  fn vector_value(&self, target_ord: usize) -> Result<Cow<'_, VectorValueEnum>> {
    Ok(Cow::Borrowed(&self.vectors[target_ord]))
  }

  type FloatVectorValues = Self;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    Ok(None)
  }

  type VectorScorer = DummyVectorScorer;

  fn get_vectors_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    Ok(&mut self.vectors)
  }

  fn get_vectors(&self) -> Result<&[VectorValueEnum]> {
    Ok(&self.vectors)
  }
}
