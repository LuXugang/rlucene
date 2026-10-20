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
use crate::core::index::vector_buffer::VectorElement;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::random_vector_scorer::{RandomVectorScorer, RandomVectorScorerEnum2};
use crate::core::util::ram_usage_estimator::size_of_vec;

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

  type RandomVectorScorerSupplier: RandomVectorScorerSupplier + Send;
  /// Make a copy of the supplier, which will copy the underlying
  /// `vectorValues` so the copy is safe to be used in other threads.
  fn copy(&self) -> Result<Self::RandomVectorScorerSupplier>
  where
    Self: Sized;

  fn ram_bytes_used(&self) -> Result<i64> {
    Ok(0)
  }
}

/// Mutable, homogeneous storage used while building an in-memory graph.
/// Encoding-independent and off-heap suppliers only require RandomVectorScorerSupplier.
pub trait BufferedVectorScorerSupplier: RandomVectorScorerSupplier {
  type Element: VectorElement;
  fn get_vector(&self) -> Result<&[Vec<Self::Element>]>;
  fn get_vector_mut(&mut self) -> Result<&mut Vec<Vec<Self::Element>>>;
}

pub(crate) fn vector_values_ram_bytes_used<T>(vectors: &[Vec<T>], capacity: usize) -> Result<i64> {
  let capacity = i64::try_from(capacity).unwrap_or(i64::MAX);
  let mut size = (std::mem::size_of::<Vec<T>>() as i64).saturating_mul(capacity);
  for vector in vectors {
    size = size.saturating_add(size_of_vec(vector));
  }
  Ok(size)
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

  fn ram_bytes_used(&self) -> Result<i64> {
    (**self).ram_bytes_used()
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

            fn ram_bytes_used(&self) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.ram_bytes_used(), )+
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
