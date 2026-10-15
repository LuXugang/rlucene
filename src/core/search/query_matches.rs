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
//! Concrete matches preserve lazy term loading and repeated field iterator creation.
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::{LRPosting, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::terms::Terms;
use crate::core::search::disjunction_matches_iterator::{
  DisjunctionMatchesIterator, TermBytesRefIterator, TermsEnumDisjunctionMatchesIterator,
};
use crate::core::search::multi_term_query::MultiTermQuery;
use crate::core::search::phrase_weight::PhraseMatchesIterator;
use crate::core::search::query::Query;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::{
  self,
  matches::Matches,
  matches_iterator::MatchesIterator,
  matches_utils::{CombinedMatch, FieldMatches, MatchWithNoTerms},
  named_matches::NamedMatches,
};
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;
use std::sync::Arc;
type LeafTermsEnum<LR> =
  crate::core::index::terms_enum::TermsEnumWithUnsupportedPostingsAndAttributesWithEmpty<
    <<LR as LeafReader>::Terms as Terms>::TermsEnum,
  >;
type RewriteIterator<LR, Q> = TermsEnumDisjunctionMatchesIterator<
  LeafTermsEnum<LR>,
  <Q as MultiTermQuery>::TermsEnum<Rc<<LR as LeafReader>::Terms>>,
>;
type DocValuesIterator<LR> = TermsEnumDisjunctionMatchesIterator<
  LeafTermsEnum<LR>,
  search::doc_values_rewrite_method::MultiTermQueryDocValuesTermsEnum<
    crate::core::index::doc_values::SortedSet<LR>,
  >,
>;
pub enum QueryWeightMatches<
  'a,
  IRC: IndexReaderContext + 'static,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: Similarity,
{
  MatchWithNoTerms(MatchWithNoTerms),
  NamedMatches(Box<NamedMatches<'a, IRC, SIM>>),
  Combined(Box<CombinedMatch<'a, IRC, SIM>>),
  Field(Box<FieldMatches<'a, IRC, SIM>>),
  #[cfg(test)]
  Asserting(
    Box<crate::test_framework::core::search::asserting_matches::AssertingMatches<'a, IRC, SIM>>,
  ),
}
impl<'a, IRC: IndexReaderContext + 'static, SIM> Matches for QueryWeightMatches<'a, IRC, SIM>
where
  SIM: Similarity,
{
  type Iterator<'b>
    = QueryWeightMatchesIterator<'b, IRCLeafReader<IRC>, SIM>
  where
    Self: 'b;
  type SubMatches = Self;
  fn get_matches(&self, field: &str) -> Result<Option<Self::Iterator<'_>>> {
    match self {
      Self::MatchWithNoTerms(_) => Ok(None),
      Self::NamedMatches(m) => m.get_matches(field),
      Self::Combined(m) => m.get_matches(field),
      Self::Field(m) => m.get_matches(field),
      #[cfg(test)]
      Self::Asserting(m) => m.get_matches(field),
    }
  }
  fn get_sub_matches(&self) -> &[Self] {
    match self {
      Self::MatchWithNoTerms(_) => &[],
      Self::NamedMatches(m) => m.get_sub_matches(),
      Self::Combined(m) => m.get_sub_matches(),
      Self::Field(m) => m.get_sub_matches(),
      #[cfg(test)]
      Self::Asserting(m) => m.get_sub_matches(),
    }
  }
  fn field(&self) -> &[String] {
    match self {
      Self::MatchWithNoTerms(m) => m.field(),
      Self::NamedMatches(m) => m.field(),
      Self::Combined(m) => m.field(),
      Self::Field(m) => m.field(),
      #[cfg(test)]
      Self::Asserting(m) => m.field(),
    }
  }
}
macro_rules! iterator_nodes {
 ($( $(#[$attr:meta])* $variant:ident: $ty:ty ),+ $(,)?) => {
  pub struct QueryWeightMatchesIterator<'a, LR: LeafReader, SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity> where SIM: Similarity, {
   inner: MatchesIteratorNode<'a, LR, SIM>,
   reader_type: std::marker::PhantomData<(fn() -> LR, &'a ())>,
  }
  enum MatchesIteratorNode<'a, LR: LeafReader, SIM> where SIM: Similarity, {
   $( $(#[$attr])* $variant(Box<$ty>), )+
   Combined(Box<DisjunctionMatchesIterator<QueryWeightMatchesIterator<'a, LR, SIM>>>),
   #[cfg(test)] Asserting(Box<crate::test_framework::core::search::asserting_matches_iterator::AssertingMatchesIterator<'a, LR, SIM>>),
   #[cfg(test)] Fixture(Box<dyn TestMatchesIterator + 'a>),
  }
  $( $(#[$attr])* impl<'a, LR: LeafReader, SIM> From<Box<$ty>> for QueryWeightMatchesIterator<'a, LR, SIM> where SIM: Similarity, {
   fn from(value: Box<$ty>) -> Self { Self { inner: MatchesIteratorNode::$variant(value), reader_type: std::marker::PhantomData } }
  } )+
  impl<'a, LR: LeafReader, SIM> From<Box<DisjunctionMatchesIterator<Self>>> for QueryWeightMatchesIterator<'a, LR, SIM> where SIM: Similarity, {
   fn from(value: Box<DisjunctionMatchesIterator<Self>>) -> Self { Self { inner: MatchesIteratorNode::Combined(value), reader_type: std::marker::PhantomData } }
  }
  #[cfg(test)] impl<'a, LR: LeafReader, SIM> From<Box<crate::test_framework::core::search::asserting_matches_iterator::AssertingMatchesIterator<'a, LR, SIM>>> for QueryWeightMatchesIterator<'a, LR, SIM> where SIM: Similarity, {
   fn from(value: Box<crate::test_framework::core::search::asserting_matches_iterator::AssertingMatchesIterator<'a, LR, SIM>>) -> Self { Self { inner: MatchesIteratorNode::Asserting(value), reader_type: std::marker::PhantomData } }
  }
  #[cfg(test)] impl<'a, LR: LeafReader, SIM> From<Box<dyn TestMatchesIterator + 'a>> for QueryWeightMatchesIterator<'a, LR, SIM> where SIM: Similarity, {
   fn from(value: Box<dyn TestMatchesIterator + 'a>) -> Self { Self { inner: MatchesIteratorNode::Fixture(value), reader_type: std::marker::PhantomData } }
  }
  macro_rules! dispatch_iterator {
   ($value:expr, |$m:ident| $body:expr) => {
    match $value {
     $( $(#[$attr])* MatchesIteratorNode::$variant($m) => $body, )+
     MatchesIteratorNode::Combined($m) => $body,
     #[cfg(test)] MatchesIteratorNode::Asserting($m) => $body,
     #[cfg(test)] MatchesIteratorNode::Fixture($m) => $body,
    }
   };
  }
  impl<'a, LR: LeafReader, SIM> MatchesIterator for QueryWeightMatchesIterator<'a, LR, SIM> where SIM: Similarity, {
   type SubMatches<'b> = QueryWeightMatchesIterator<'b, LR, SIM> where Self: 'b;
   fn next(&mut self) -> Result<bool> { dispatch_iterator!(&mut self.inner, |m| MatchesIterator::next(&mut **m)) }
   fn start_position(&self) -> Result<i32> { dispatch_iterator!(&self.inner, |m| MatchesIterator::start_position(&**m)) }
   fn end_position(&self) -> Result<i32> { dispatch_iterator!(&self.inner, |m| MatchesIterator::end_position(&**m)) }
   fn start_offset(&self) -> Result<i32> { dispatch_iterator!(&self.inner, |m| MatchesIterator::start_offset(&**m)) }
   fn end_offset(&self) -> Result<i32> { dispatch_iterator!(&self.inner, |m| MatchesIterator::end_offset(&**m)) }
   fn get_query(&self) -> Result<Arc<Query>> { dispatch_iterator!(&self.inner, |m| MatchesIterator::get_query(&**m)) }
   fn get_sub_matches(&mut self) -> Result<Option<Self::SubMatches<'_>>> {
    match &mut self.inner {
     $( $(#[$attr])* MatchesIteratorNode::$variant(m) => { let sub = MatchesIterator::get_sub_matches(&mut **m)?; debug_assert!(sub.is_none()); Ok(None) }, )+
     MatchesIteratorNode::Combined(m) => MatchesIterator::get_sub_matches(&mut **m),
     #[cfg(test)] MatchesIteratorNode::Asserting(m) => MatchesIterator::get_sub_matches(&mut **m),
     #[cfg(test)] MatchesIteratorNode::Fixture(m) => m.erased_sub_matches().map(|sub| sub.map(Into::into)),
    }
   }
  }
 };
}
iterator_nodes! {
 Term: search::term_matches_iterator::TermMatchesIterator<LRPosting<LR>>,
 Phrase: PhraseMatchesIterator<crate::core::index::impacts_enum::ImpactsEnumEnum2<crate::core::index::leaf_reader::LRImpactsEnum<LR>, crate::core::index::slow_impacts_enum::SlowImpactsEnum<LRPosting<LR>>>, Arc<search::phrase_weight::SimScorerType<SIM>>>,
 MultiPhrase: PhraseMatchesIterator<crate::core::index::slow_impacts_enum::SlowImpactsEnum<search::multi_phrase_query::MultiPhrasePostingsEnum<LRPosting<LR>>>, Arc<search::phrase_weight::SimScorerType<SIM>>>,
 Terms: TermsEnumDisjunctionMatchesIterator<LeafTermsEnum<LR>, TermBytesRefIterator>,
 DocValues: DocValuesIterator<LR>,
 Automaton: RewriteIterator<LR, search::automaton_query::AutomatonQuery>,
 Fuzzy: RewriteIterator<LR, search::fuzzy_query::FuzzyQuery>,
 TermSet: RewriteIterator<LR, search::term_in_set_query::TermInSetQuery>,
 #[cfg(test)] BoostChecking: RewriteIterator<LR, crate::test_framework::core::search::multi_term::BoostCheckingQuery>,
 #[cfg(test)] DumbPrefix: RewriteIterator<LR, crate::test_framework::core::search::multi_term::DumbPrefixQuery>,
 #[cfg(test)] DumbRegexp: RewriteIterator<LR, crate::test_framework::core::search::multi_term::DumbRegexpQuery>,
}
#[cfg(test)]
pub trait TestMatchesIterator {
  fn next(&mut self) -> Result<bool>;
  fn start_position(&self) -> Result<i32>;
  fn end_position(&self) -> Result<i32>;
  fn start_offset(&self) -> Result<i32>;
  fn end_offset(&self) -> Result<i32>;
  fn get_query(&self) -> Result<Arc<Query>>;
  fn erased_sub_matches(&mut self) -> Result<Option<Box<dyn TestMatchesIterator + '_>>>;
}
#[cfg(test)]
impl<M: MatchesIterator> TestMatchesIterator for M {
  fn next(&mut self) -> Result<bool> {
    MatchesIterator::next(self)
  }
  fn start_position(&self) -> Result<i32> {
    MatchesIterator::start_position(self)
  }
  fn end_position(&self) -> Result<i32> {
    MatchesIterator::end_position(self)
  }
  fn start_offset(&self) -> Result<i32> {
    MatchesIterator::start_offset(self)
  }
  fn end_offset(&self) -> Result<i32> {
    MatchesIterator::end_offset(self)
  }
  fn get_query(&self) -> Result<Arc<Query>> {
    MatchesIterator::get_query(self)
  }
  fn erased_sub_matches(&mut self) -> Result<Option<Box<dyn TestMatchesIterator + '_>>> {
    Ok(
      self
        .get_sub_matches()?
        .map(|m| Box::new(m) as Box<dyn TestMatchesIterator>),
    )
  }
}
#[cfg(test)]
impl MatchesIterator for dyn TestMatchesIterator + '_ {
  type SubMatches<'b>
    = Box<dyn TestMatchesIterator + 'b>
  where
    Self: 'b;
  fn next(&mut self) -> Result<bool> {
    TestMatchesIterator::next(self)
  }
  fn start_position(&self) -> Result<i32> {
    TestMatchesIterator::start_position(self)
  }
  fn end_position(&self) -> Result<i32> {
    TestMatchesIterator::end_position(self)
  }
  fn start_offset(&self) -> Result<i32> {
    TestMatchesIterator::start_offset(self)
  }
  fn end_offset(&self) -> Result<i32> {
    TestMatchesIterator::end_offset(self)
  }
  fn get_query(&self) -> Result<Arc<Query>> {
    TestMatchesIterator::get_query(self)
  }
  fn get_sub_matches(&mut self) -> Result<Option<Self::SubMatches<'_>>> {
    self.erased_sub_matches()
  }
}
pub struct FieldMatchesSupplier<
  'a,
  IRC: IndexReaderContext + 'static,
  SIM = crate::core::search::similarities_impl::bm25_similarity::BM25Similarity,
> where
  SIM: Similarity,
{
  inner: FieldSupplierNode<'a, IRC, SIM>,
}
macro_rules! field_nodes {
 ($( $(#[$attr:meta])* $variant:ident [$($generic:ty),+]: $ty:ty { $( $capture:ident: $capture_ty:ty ),* } ),+ $(,)?) => {
  enum FieldSupplierNode<'a, IRC: IndexReaderContext + 'static, SIM> where SIM: Similarity, {
   $( $(#[$attr])* $variant(&'a $ty, &'a LeafReaderContext<IRCLeafReader<IRC>>, i32 $(, $capture_ty)*), )+
  }
  $( $(#[$attr])* impl<'a, IRC: IndexReaderContext + 'static, SIM> From<(&'a $ty, &'a LeafReaderContext<IRCLeafReader<IRC>>, i32 $(, $capture_ty)*)> for FieldMatchesSupplier<'a, IRC, SIM> where SIM: Similarity, {
   fn from((w,c,d $(, $capture)*): (&'a $ty, &'a LeafReaderContext<IRCLeafReader<IRC>>, i32 $(, $capture_ty)*)) -> Self {
    Self { inner: FieldSupplierNode::$variant(w,c,d $(, $capture)*) }
   }
  } )+
  impl<'a, IRC: IndexReaderContext + 'static, SIM> FieldMatchesSupplier<'a, IRC, SIM> where SIM: Similarity, {
   pub(crate) fn get(&self) -> Result<Option<QueryWeightMatchesIterator<'a, IRCLeafReader<IRC>, SIM>>> {
    match &self.inner { $( $(#[$attr])* FieldSupplierNode::$variant(w,c,d $(, $capture)*) => w.matches_iterator::<$($generic),+>(c,*d $(, $capture.clone())*), )+ }
   }
  }
 };
}
field_nodes! {
 Term [IRC]: search::term_query::TermWeight<SIM> {},
 Phrase [IRC]: search::phrase_weight::PhraseWeight<search::phrase_query::PhraseQueryWeightBase<SIM>, SIM> {},
 MultiPhrase [IRC]: search::phrase_weight::PhraseWeight<search::multi_phrase_query::MultiPhraseQueryWeightBase<SIM>, SIM> {},
 DocValues [IRC, SIM]: search::doc_values_rewrite_method::MultiTermQueryDocValuesWeight {},
 Synonym [IRC]: search::synonym_query::SynonymWeight<SIM> { terms: Arc<[crate::core::index::term::Term]> },
 RewriteAutomaton [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::automaton_query::AutomatonQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 RewriteFuzzy [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::fuzzy_query::FuzzyQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 RewritePrefix [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::prefix_query::PrefixQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 RewriteRegexp [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::regexp_query::RegexpQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 RewriteTermSet [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::term_in_set_query::TermInSetQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 RewriteTermRange [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::term_range_query::TermRangeQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 RewriteWildcard [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<search::wildcard_query::WildcardQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 #[cfg(test)] RewriteBoostChecking [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::BoostCheckingQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 #[cfg(test)] RewriteDumbPrefix [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::DumbPrefixQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
 #[cfg(test)] RewriteDumbRegexp [IRC, SIM]: search::abstract_multi_term_query_constant_score_wrapper::RewritingWeight<crate::test_framework::core::search::multi_term::DumbRegexpQuery> { terms: Rc<<IRCLeafReader<IRC> as LeafReader>::Terms> },
}
