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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::hnsw::random_vector_scorer::{
  RandomVectorScorer, RandomVectorScorerEnum2, RandomVectorScorerEnum3,
};

/// A supplier that creates  [`RandomVectorScorer`] from an ordinal.
pub trait RandomVectorScorerSupplier {
  type Scorer<'a>: RandomVectorScorer
  where
    Self: 'a;
  /// This creates a [`RandomVectorScorer`] for scoring random nodes in
  /// batches against the given ordinal.
  ///
  /// # Arguments
  ///
  /// * `ord` - The ordinal of the node to compare.
  ///
  /// # Returns
  ///
  /// A new [`RandomVectorScorer`].
  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>>;

  type RandomVectorScorerSupplier: RandomVectorScorerSupplier;
  /// Make a copy of the supplier, which will copy the underlying
  /// `vectorValues` so the copy is safe to be used in other threads.
  fn copy(&self) -> Result<Self::RandomVectorScorerSupplier>
  where
    Self: Sized;

  fn get_vector(&self) -> Result<&[VectorValueEnum]> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn get_vector_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
    Err(LuceneError::unsupported_operation(""))
  }
}
impl<T> RandomVectorScorerSupplier for &T
where
  T: RandomVectorScorerSupplier,
{
  type Scorer<'a>
    = T::Scorer<'a>
  where
    Self: 'a;

  fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
    (**self).scorer(ord)
  }

  type RandomVectorScorerSupplier = T::RandomVectorScorerSupplier;

  fn copy(&self) -> Result<Self::RandomVectorScorerSupplier>
  where
    Self: Sized,
  {
    (**self).copy()
  }

  fn get_vector(&self) -> Result<&[VectorValueEnum]> {
    (**self).get_vector()
  }
}

macro_rules! either_random_vector_scorer_supplier {
    (
        $vis:vis $name:ident {
            scorer = $scorer_enum:ident;
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> RandomVectorScorerSupplier for $name<$( $T ),+>
        where
            $( $T: RandomVectorScorerSupplier ),+
        {
            type Scorer<'a> =
                $scorer_enum<$( < $T as RandomVectorScorerSupplier >::Scorer<'a> ),+>
            where
                Self: 'a;

            type RandomVectorScorerSupplier =
                $name<$( < $T as RandomVectorScorerSupplier >::RandomVectorScorerSupplier ),+>;

            fn scorer(&self, ord: usize) -> Result<Self::Scorer<'_>> {
                match self {
                    $( Self::$Variant(inner) => inner.scorer(ord).map($scorer_enum::$Variant), )+
                }
            }

            fn copy(&self) -> Result<Self::RandomVectorScorerSupplier>
            where
                Self: Sized,
            {
                match self {
                    $( Self::$Variant(inner) => inner.copy().map(Self::RandomVectorScorerSupplier::$Variant), )+
                }
            }

            fn get_vector(&self) -> Result<&[VectorValueEnum]> {
                match self {
                    $( Self::$Variant(inner) => inner.get_vector(), )+
                }
            }

            fn get_vector_mut(&mut self) -> Result<&mut Vec<VectorValueEnum>> {
                match self {
                    $( Self::$Variant(inner) => inner.get_vector_mut(), )+
                }
            }
        }
    };
}

either_random_vector_scorer_supplier!(
    pub RandomVectorScorerSupplierEnum2 {
        scorer = RandomVectorScorerEnum2;
        A: A,
        B: B,
    }
);

either_random_vector_scorer_supplier!(
    pub RandomVectorScorerSupplierEnum3 {
        scorer = RandomVectorScorerEnum3;
        A: A,
        B: B,
        C: C,
    }
);
