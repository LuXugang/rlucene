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
use crate::core::index::index_reader::Identity;
use crate::core::util::HasIdentity;
use crate::core::util::bits::{Bits, BitsEnum2};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;

/// A trait for scoring random nodes in batches against an abstract query.
pub trait RandomVectorScorer {
  /// Returns the score between the query and the provided node.
  ///
  /// # Arguments
  ///
  /// * `node` - a random node in the graph
  ///
  /// # Errors
  ///
  /// Returns an error if the scoring fails (e.g., I/O error).
  fn score(&self, node: usize) -> Result<f32>;

  /// Returns the maximum possible ordinal for this scorer.
  fn max_ord(&self) -> usize;

  /// Translates a vector ordinal to the correct document ID.  
  /// By default, this is an identity function.
  ///
  /// # Arguments
  ///
  /// * `ord` - The vector ordinal.
  ///
  /// # Returns
  ///
  /// The document ID for the given vector ordinal.
  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    Ok(ord)
  }

  type Bits<'a, B>: Bits
  where
    B: Bits,
    Self: 'a;
  /// Returns the [`Bits`] representing live documents.  
  /// By default, this is an identity function.
  ///
  /// # Arguments
  ///
  /// * `accept_docs` - The accept docs.
  ///
  /// # Returns
  ///
  /// The accept docs.
  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Result<Option<Self::Bits<'a, B>>>
  where
    B: Bits;
}

pub enum RandomVectorScorerBits3<A, B, C> {
  A(A),
  B(B),
  C(C),
}

impl<A, B, C> Clone for RandomVectorScorerBits3<A, B, C>
where
  A: Clone,
  B: Clone,
  C: Clone,
{
  fn clone(&self) -> Self {
    match self {
      Self::A(bits) => Self::A(bits.clone()),
      Self::B(bits) => Self::B(bits.clone()),
      Self::C(bits) => Self::C(bits.clone()),
    }
  }
}

impl<A, B, C> HasIdentity for RandomVectorScorerBits3<A, B, C>
where
  A: HasIdentity,
  B: HasIdentity,
  C: HasIdentity,
{
  fn identity(&self) -> &Identity {
    match self {
      Self::A(bits) => bits.identity(),
      Self::B(bits) => bits.identity(),
      Self::C(bits) => bits.identity(),
    }
  }
}

impl<A, B, C> Bits for RandomVectorScorerBits3<A, B, C>
where
  A: Bits,
  B: Bits,
  C: Bits,
{
  fn get(&self, index: usize) -> Result<bool> {
    match self {
      Self::A(bits) => bits.get(index),
      Self::B(bits) => bits.get(index),
      Self::C(bits) => bits.get(index),
    }
  }

  fn length(&self) -> usize {
    match self {
      Self::A(bits) => bits.length(),
      Self::B(bits) => bits.length(),
      Self::C(bits) => bits.length(),
    }
  }

  fn copy_of(&self) -> Result<FixedBitSet> {
    match self {
      Self::A(bits) => bits.copy_of(),
      Self::B(bits) => bits.copy_of(),
      Self::C(bits) => bits.copy_of(),
    }
  }

  fn to_string(&self) -> String {
    match self {
      Self::A(bits) => bits.to_string(),
      Self::B(bits) => bits.to_string(),
      Self::C(bits) => bits.to_string(),
    }
  }
}

macro_rules! either_random_vector_scorer {
    (
        $vis:vis $name:ident {
            bits_param = $bits_param:ident;
            bits = $bits_ty:ty;
            accept_ords = |$self_:ident, $accept_docs:ident| $accept_ords_expr:expr;
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> RandomVectorScorer for $name<$( $T ),+>
        where
            $( $T: RandomVectorScorer ),+
        {
            fn score(&self, node: usize) -> Result<f32> {
                match self {
                    $( Self::$Variant(inner) => inner.score(node), )+
                }
            }

            fn max_ord(&self) -> usize {
                match self {
                    $( Self::$Variant(inner) => inner.max_ord(), )+
                }
            }

            fn ord_to_doc(&self, ord: usize) -> Result<usize> {
                match self {
                    $( Self::$Variant(inner) => inner.ord_to_doc(ord), )+
                }
            }

            type Bits<'a, $bits_param>
                = $bits_ty
            where
                $bits_param: Bits,
                Self: 'a;

            fn get_accept_ords<'a, $bits_param>(
                &'a $self_,
                $accept_docs: Option<$bits_param>,
            ) -> Result<Option<Self::Bits<'a, $bits_param>>>
            where
                $bits_param: Bits,
            {
                $accept_ords_expr
            }
        }
    };
}

either_random_vector_scorer!(
    pub RandomVectorScorerEnum2 {
        bits_param = Q;
        bits = BitsEnum2<A::Bits<'a, Q>, B::Bits<'a, Q>>;
        accept_ords = |self, accept_docs| {
            Ok(match self {
                Self::A(inner) => inner.get_accept_ords(accept_docs)?.map(BitsEnum2::A),
                Self::B(inner) => inner.get_accept_ords(accept_docs)?.map(BitsEnum2::B),
            })
        };
        A: A,
        B: B,
    }
);

either_random_vector_scorer!(
    pub RandomVectorScorerEnum3 {
        bits_param = Q;
        bits = RandomVectorScorerBits3<A::Bits<'a, Q>, B::Bits<'a, Q>, C::Bits<'a, Q>>;
        accept_ords = |self, accept_docs| {
            Ok(match self {
                Self::A(inner) => inner
                    .get_accept_ords(accept_docs)?
                    .map(RandomVectorScorerBits3::A),
                Self::B(inner) => inner
                    .get_accept_ords(accept_docs)?
                    .map(RandomVectorScorerBits3::B),
                Self::C(inner) => inner
                    .get_accept_ords(accept_docs)?
                    .map(RandomVectorScorerBits3::C),
            })
        };
        A: A,
        B: B,
        C: C,
    }
);
