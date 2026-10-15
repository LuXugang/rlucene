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
//! Owned search types without carrying the lifetime of the reader itself.
use crate::core::index::leaf_reader::LeafReader;
use crate::core::search::similarities_impl::similarities::Similarity;
pub trait ScorerTypes: 'static {
  type Similarity: Similarity;
  type Posting: crate::core::index::postings_enum::PostingsEnum + 'static;
  type Impacts: crate::core::index::impacts_enum::ImpactsEnum + 'static;
  type Norm: crate::core::index::numeric_doc_values::NumericDocValues + 'static;
  type Numeric: crate::core::index::numeric_doc_values::NumericDocValues + 'static;
  type Binary: crate::core::index::binary_doc_values::BinaryDocValues + 'static;
  type Sorted: crate::core::index::sorted_doc_values::SortedDocValues + 'static;
  type SortedNumeric: crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues
    + 'static;
  type SortedSet: crate::core::index::sorted_set_doc_values::SortedSetDocValues + 'static;
  type Skipper: crate::core::index::doc_values_skipper::DocValuesSkipper + 'static;
  type ByteIterator: crate::core::index::knn_vector_values::DocIndexIterator + 'static;
  type FloatIterator: crate::core::index::knn_vector_values::DocIndexIterator + 'static;
  type ByteScorer: crate::core::search::vector_scorer::VectorScorer + 'static;
  type FloatScorer: crate::core::search::vector_scorer::VectorScorer + 'static;
  type PointValues: crate::core::index::point_values::PointValues + 'static;
}
pub struct OwnedScorerTypes<
  P,
  I,
  N,
  D,
  B,
  S,
  SN,
  SS,
  K,
  BI,
  FI,
  BS,
  FS,
  PV,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
>(
  #[expect(
    clippy::type_complexity,
    reason = "This zero-sized marker records the fourteen associated types in ScorerTypes."
  )]
  std::marker::PhantomData<fn() -> (P, I, N, D, B, S, SN, SS, K, BI, FI, BS, FS, PV, SIM)>,
)
where
  SIM: Similarity;
impl<P, I, N, D, B, S, SN, SS, K, BI, FI, BS, FS, PV, SIM> ScorerTypes
  for OwnedScorerTypes<P, I, N, D, B, S, SN, SS, K, BI, FI, BS, FS, PV, SIM>
where
  SIM: Similarity,
  P: crate::core::index::postings_enum::PostingsEnum + 'static,
  I: crate::core::index::impacts_enum::ImpactsEnum + 'static,
  N: crate::core::index::numeric_doc_values::NumericDocValues + 'static,
  D: crate::core::index::numeric_doc_values::NumericDocValues + 'static,
  B: crate::core::index::binary_doc_values::BinaryDocValues + 'static,
  S: crate::core::index::sorted_doc_values::SortedDocValues + 'static,
  SN: crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues + 'static,
  SS: crate::core::index::sorted_set_doc_values::SortedSetDocValues + 'static,
  K: crate::core::index::doc_values_skipper::DocValuesSkipper + 'static,
  BI: crate::core::index::knn_vector_values::DocIndexIterator + 'static,
  FI: crate::core::index::knn_vector_values::DocIndexIterator + 'static,
  BS: crate::core::search::vector_scorer::VectorScorer + 'static,
  FS: crate::core::search::vector_scorer::VectorScorer + 'static,
  PV: crate::core::index::point_values::PointValues + 'static,
{
  type Similarity = SIM;
  type Posting = P;
  type Impacts = I;
  type Norm = N;
  type Numeric = D;
  type Binary = B;
  type Sorted = S;
  type SortedNumeric = SN;
  type SortedSet = SS;
  type Skipper = K;
  type ByteIterator = BI;
  type FloatIterator = FI;
  type ByteScorer = BS;
  type FloatScorer = FS;
  type PointValues = PV;
}
pub type LeafScorerTypes<LR, SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity> = OwnedScorerTypes<
  crate::core::index::leaf_reader::LRPosting<LR>,
  crate::core::index::leaf_reader::LRImpactsEnum<LR>,
  <LR as LeafReader>::NormNumericDocValues,
  <LR as LeafReader>::NumericDocValues,
  <LR as LeafReader>::BinaryDocValues,
  <LR as LeafReader>::SortedDocValues,
  <LR as LeafReader>::SortedNumericDocValues,
  <LR as LeafReader>::SortedSetDocValues,
  <LR as LeafReader>::DocValuesSkipper,
  crate::core::index::leaf_reader::IRCByteVectorIter<LR>,
  crate::core::index::leaf_reader::IRCFloatVectorIter<LR>,
  <<LR as LeafReader>::ByteVectorValues as crate::core::index::byte_vector_values::ByteVectorValues>::VectorScorer,
  <<LR as LeafReader>::FloatVectorValues as crate::core::index::float_vector_values::FloatVectorValues>::VectorScorer,
  <LR as LeafReader>::PointValues,
 SIM>;
