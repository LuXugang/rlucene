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
use crate::core::index::BytesRef;
use crate::core::index::automaton_terms_enum::AutomatonTermsEnum;
use crate::core::index::doc_values::DocValues;
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::core::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::core::index::filtered_terms_enum::FilteredTermsEnum;
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::index::sorted_set_doc_values_terms_enum::SortedSetDocValuesTermsEnum;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::disjunction_matches_iterator::from_terms_enum;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, EmptyDISI};
use crate::core::search::doc_values_range_iterator::DocValuesRangeIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::fuzzy_query::FuzzyQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::for_field;
use crate::core::search::multi_term_query::{
  MultiTermQuery, MultiTermQuerySet, RewriteMethod, dispatch_multi_term_query,
};
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::term_in_set_query::TermInSetQuery;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::two_phase_iterator::{TwoPhaseIterator, TwoPhaseIteratorEnum2};
use crate::core::search::weight::Weight;
use crate::core::search::wildcard_query::WildcardQuery;
use crate::core::util::automation::compiled_automaton::CompiledAutomaton;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::dummy::dummy_attribute_source::DummyAttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::long_bit_set::LongBitSet;
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

/// Rewrites `MultiTermQueries` into a filter, using DocValues for term enumeration.
///
/// This can be used to perform these queries against an unindexed docvalues field.
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DocValuesRewriteMethod;
impl RewriteMethod for DocValuesRewriteMethod {
  fn rewrite<IRC, Q>(self, _index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQuerySet>,
    IRC: IndexReaderContext,
  {
    Ok(ConstantScoreQuery::new(MultiTermQueryDocValuesWrapper::new(query)).into())
  }
}

#[derive(Clone)]
pub struct MultiTermQueryDocValuesWrapper {
  query: MultiTermQuerySet,
  id: Identity,
}

impl MultiTermQueryDocValuesWrapper {
  pub fn new<T>(query: T) -> Self
  where
    T: Into<MultiTermQuerySet>,
  {
    Self {
      query: query.into(),
      id: Identity::new(),
    }
  }

  pub fn get_field(&self) -> &str {
    dispatch_multi_term_query!(&self.query, |q| q.get_field())
  }
}

impl Debug for MultiTermQueryDocValuesWrapper {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.to_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for MultiTermQueryDocValuesWrapper {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for MultiTermQueryDocValuesWrapper {
  fn to_string(&self, field: &str) -> Result<String> {
    self.query.to_string(field)
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(MultiTermQueryDocValuesWeight::new(
      self,
      boost,
      *score_mode,
    )))
  }

  fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(self.into())
  }

  fn visit<QV>(&self, visitor: &mut QV) -> Result<()>
  where
    QV: QueryVisitor,
  {
    if visitor.accept_field(self.get_field()) {
      dispatch_multi_term_query!(&self.query, |query| {
        let _ = visitor.get_sub_visitor(Occur::Filter, query.into());
      });
    }
    Ok(())
  }
}

impl Hash for MultiTermQueryDocValuesWrapper {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    std::any::type_name::<Self>().hash(state);
    self.query.hash(state);
  }
}

impl PartialEq for MultiTermQueryDocValuesWrapper {
  fn eq(&self, other: &Self) -> bool {
    self.query == other.query
  }
}

impl Eq for MultiTermQueryDocValuesWrapper {}

pub struct MultiTermQueryDocValuesWeight {
  parent_query: Arc<Query>,
  query: MultiTermQuerySet,
  base: ConstantScoreWeight,
  score_mode: ScoreMode,
}

impl MultiTermQueryDocValuesWeight {
  fn new(query: MultiTermQueryDocValuesWrapper, boost: f32, score_mode: ScoreMode) -> Self {
    let query_enum = query.query.clone();
    Self {
      parent_query: Arc::new(query.into()),
      query: query_enum,
      base: ConstantScoreWeight::new(boost),
      score_mode,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for MultiTermQueryDocValuesWeight
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    let field = dispatch_multi_term_query!(&self.query, |q| q.get_field().to_string());
    DocValues::is_cacheable(ctx, &[field])
  }
}

impl<IRC> Weight<IRC> for MultiTermQueryDocValuesWeight
where
  IRC: IndexReaderContext,
{
  fn matches<'a>(
    &'a self,
    context: &'a LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    _searcher: &'a IndexSearcher<IRC>,
  ) -> Result<Option<crate::core::search::query::QueryWeightMatches<'a>>> {
    let field = dispatch_multi_term_query!(&self.query, |query| query.get_field().to_string());
    for_field(field.clone(), move || {
      let values = DocValues::get_sorted_set(context.reader(), &field)?;
      let terms_enum = get_terms_enum(&self.query, values)?;
      from_terms_enum(
        context,
        doc,
        Arc::new(self.query.clone().into()),
        &field,
        terms_enum,
      )
    })
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let scorer = self.scorer(context, searcher)?;
    self
      .base
      .explain(scorer, doc, self.parent_query.to_string("")?)
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    let field = dispatch_multi_term_query!(&self.query, |q| q.get_field().to_string());
    let values = DocValues::get_sorted_set(context.reader(), &field)?;
    if values.get_value_count()? == 0 {
      return Ok(None);
    }
    Ok(Some(Box::new(DocValuesScorerSupplier::new(
      self.query.clone(),
      values.cost()?,
      self.base.score(),
      self.score_mode,
    ))))
  }
}

pub struct DocValuesScorerSupplier {
  query: MultiTermQuerySet,
  cost: i64,
  score: f32,
  score_mode: ScoreMode,
}

impl DocValuesScorerSupplier {
  fn new(query: MultiTermQuerySet, cost: i64, score: f32, score_mode: ScoreMode) -> Self {
    Self {
      query,
      cost,
      score,
      score_mode,
    }
  }
}

impl<IRC> ScorerSupplier<IRC> for DocValuesScorerSupplier
where
  IRC: IndexReaderContext,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    _lead_cost: i64,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    let field = dispatch_multi_term_query!(&self.query, |q| q.get_field().to_string());
    let values = DocValues::get_sorted_set(context.reader(), &field)?;
    let mut terms_enum = get_terms_enum(&self.query, values)?;

    if terms_enum.next()?.is_none() {
      let v = ConstantScoreScorer::from_disi(self.score, self.score_mode, EmptyDISI::default());
      return Ok(Box::new(v));
    }

    let mut skipper_opt = context.reader().get_doc_values_skipper(&field)?;
    let mut values = DocValues::get_sorted_set(context.reader(), &field)?;
    // Create a bit set for the "term set" ordinals (these are the terms provided by the
    // query that are actually present in the doc values field). Cannot use FixedBitSet
    // because we require long index (ord):
    let mut term_set = LongBitSet::new(values.get_value_count()? as usize)?;
    let min_ord = terms_enum.ord()?;
    debug_assert!(min_ord >= 0);
    let mut max_ord = -1;

    loop {
      let ord = terms_enum.ord()?;
      debug_assert!(ord >= 0 && ord > max_ord);
      max_ord = ord;
      term_set.set(ord as usize);
      if terms_enum.next()?.is_none() {
        break;
      }
    }

    if let Some(ref skipper) = skipper_opt
      && (min_ord > skipper.max_value() || max_ord < skipper.min_value())
    {
      let v = ConstantScoreScorer::from_disi(self.score, self.score_mode, EmptyDISI::default());
      return Ok(Box::new(v));
    }

    let iterator = if values.is_single_valued() {
      let singleton = DocValues::unwrap_singleton_sorted(&mut values)?;
      TwoPhaseIteratorEnum2::A(SingletonTermSetTwoPhaseIterator::new(singleton, term_set))
    } else {
      TwoPhaseIteratorEnum2::B(SortedSetTermSetTwoPhaseIterator::new(
        values, term_set, max_ord,
      ))
    };

    match skipper_opt.take() {
      Some(skipper) => {
        let v = DocValuesRangeIterator::new(iterator, skipper, min_ord, max_ord, true);
        Ok(Box::new(ConstantScoreScorer::from_tpi(
          self.score,
          self.score_mode,
          v,
        )))
      },
      None => Ok(Box::new(ConstantScoreScorer::from_tpi(
        self.score,
        self.score_mode,
        iterator,
      ))),
    }
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    Ok(self.cost)
  }
}
/// Create a [`TermsEnum`] that provides the intersection of the query terms with the terms
/// present in the doc values.
fn get_terms_enum<S>(
  query: &MultiTermQuerySet,
  values: S,
) -> Result<MultiTermQueryDocValuesTermsEnum<S>>
where
  S: SortedSetDocValues,
{
  let terms = DocValuesTerms::new(values);
  match query {
    MultiTermQuerySet::Automaton(q) => Ok(MultiTermQueryDocValuesTermsEnum::Automaton(
      q.get_terms_enum(terms)?,
    )),
    MultiTermQuerySet::Fuzzy(q) => Ok(MultiTermQueryDocValuesTermsEnum::Fuzzy(Box::new(
      q.get_terms_enum(terms)?,
    ))),
    MultiTermQuerySet::Prefix(q) => Ok(MultiTermQueryDocValuesTermsEnum::Prefix(
      q.get_terms_enum(terms)?,
    )),
    MultiTermQuerySet::Regexp(q) => Ok(MultiTermQueryDocValuesTermsEnum::Regexp(
      q.get_terms_enum(terms)?,
    )),
    MultiTermQuerySet::TermInSet(q) => Ok(MultiTermQueryDocValuesTermsEnum::TermInSet(
      q.get_terms_enum(terms)?,
    )),
    MultiTermQuerySet::TermRange(q) => Ok(MultiTermQueryDocValuesTermsEnum::TermRange(
      q.get_terms_enum(terms)?,
    )),
    MultiTermQuerySet::Wildcard(q) => Ok(MultiTermQueryDocValuesTermsEnum::Wildcard(
      q.get_terms_enum(terms)?,
    )),
    #[cfg(test)]
    MultiTermQuerySet::BoostChecking(q) => Ok(MultiTermQueryDocValuesTermsEnum::BoostChecking(
      q.get_terms_enum(terms)?,
    )),
    #[cfg(test)]
    MultiTermQuerySet::DumbPrefix(q) => Ok(MultiTermQueryDocValuesTermsEnum::DumbPrefix(
      q.get_terms_enum(terms)?,
    )),
    #[cfg(test)]
    MultiTermQuerySet::DumbRegexp(q) => Ok(MultiTermQueryDocValuesTermsEnum::DumbRegexp(
      q.get_terms_enum(terms)?,
    )),
  }
}

pub struct SingletonTermSetTwoPhaseIterator<S>
where
  S: SortedDocValues,
{
  singleton: S,
  term_set: LongBitSet,
}

impl<S> SingletonTermSetTwoPhaseIterator<S>
where
  S: SortedDocValues,
{
  fn new(singleton: S, term_set: LongBitSet) -> Self {
    Self {
      singleton,
      term_set,
    }
  }
}

impl<S> TwoPhaseIterator for SingletonTermSetTwoPhaseIterator<S>
where
  S: SortedDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.singleton)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.singleton)
  }

  fn matches(&mut self) -> Result<bool> {
    Ok(self.term_set.get(self.singleton.ord_value()? as usize))
  }

  fn match_cost(&self) -> f32 {
    3.0
  }
}

pub struct SortedSetTermSetTwoPhaseIterator<S>
where
  S: SortedSetDocValues,
{
  values: S,
  term_set: LongBitSet,
  max_ord: i64,
}

impl<S> SortedSetTermSetTwoPhaseIterator<S>
where
  S: SortedSetDocValues,
{
  fn new(values: S, term_set: LongBitSet, max_ord: i64) -> Self {
    Self {
      values,
      term_set,
      max_ord,
    }
  }
}

impl<S> TwoPhaseIterator for SortedSetTermSetTwoPhaseIterator<S>
where
  S: SortedSetDocValues,
{
  fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&mut self.values)
  }

  fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
    Box::new(&self.values)
  }

  fn matches(&mut self) -> Result<bool> {
    for _ in 0..self.values.doc_value_count()? {
      let value = self.values.next_ord()?;
      if value > self.max_ord {
        return Ok(false);
      } else if self.term_set.get(value as usize) {
        return Ok(true);
      }
    }
    Ok(false)
  }

  fn match_cost(&self) -> f32 {
    3.0
  }
}

pub struct DocValuesTerms<S>
where
  S: SortedSetDocValues,
{
  values: Rc<RefCell<S>>,
}

impl<S> DocValuesTerms<S>
where
  S: SortedSetDocValues,
{
  fn new(values: S) -> Self {
    Self {
      values: Rc::new(RefCell::new(values)),
    }
  }
}

impl<S> Clone for DocValuesTerms<S>
where
  S: SortedSetDocValues,
{
  fn clone(&self) -> Self {
    Self {
      values: self.values.clone(),
    }
  }
}

impl<S> Terms for DocValuesTerms<S>
where
  S: SortedSetDocValues,
{
  type TermsEnum = SortedSetDocValuesTermsEnum<Rc<RefCell<S>>>;

  fn iterator(&self) -> Result<Self::TermsEnum> {
    Ok(SortedSetDocValuesTermsEnum::new(self.values.clone()))
  }

  type IntersectIter = FilteredTermsEnum<Self::TermsEnum, AutomatonTermsEnum>;

  fn intersect(
    &self,
    compiled: &CompiledAutomaton,
    start_term: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<Self::IntersectIter> {
    self.default_intersect(compiled, start_term)
  }

  fn size(&self) -> Result<i64> {
    Ok(-1)
  }

  fn get_sum_total_term_freq(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_sum_doc_freq(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_doc_count(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn has_freqs(&self) -> bool {
    false
  }

  fn has_offsets(&self) -> bool {
    false
  }

  fn has_positions(&self) -> bool {
    false
  }

  fn has_payloads(&self) -> bool {
    false
  }
}

pub enum MultiTermQueryDocValuesTermsEnum<S>
where
  S: SortedSetDocValues,
{
  Automaton(<AutomatonQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>),
  Fuzzy(Box<<FuzzyQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>>),
  Prefix(<crate::core::search::prefix_query::PrefixQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>),
  Regexp(<RegexpQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>),
  TermInSet(<TermInSetQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>),
  TermRange(<TermRangeQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>),
  Wildcard(<WildcardQuery as MultiTermQuery>::TermsEnum<DocValuesTerms<S>>),
  #[cfg(test)]
  BoostChecking(
    <crate::test_framework::core::search::multi_term::BoostCheckingQuery as MultiTermQuery>::TermsEnum<
      DocValuesTerms<S>,
    >,
  ),
  #[cfg(test)]
  DumbPrefix(
    <crate::test_framework::core::search::multi_term::DumbPrefixQuery as MultiTermQuery>::TermsEnum<
      DocValuesTerms<S>,
    >,
  ),
  #[cfg(test)]
  DumbRegexp(
    <crate::test_framework::core::search::multi_term::DumbRegexpQuery as MultiTermQuery>::TermsEnum<
      DocValuesTerms<S>,
    >,
  ),
}

macro_rules! dispatch_doc_values_terms_enum {
  ($self:expr, |$inner:ident| $body:expr) => {{
    match $self {
      MultiTermQueryDocValuesTermsEnum::Automaton($inner) => $body,
      MultiTermQueryDocValuesTermsEnum::Fuzzy($inner) => $body,
      MultiTermQueryDocValuesTermsEnum::Prefix($inner) => $body,
      MultiTermQueryDocValuesTermsEnum::Regexp($inner) => $body,
      MultiTermQueryDocValuesTermsEnum::TermInSet($inner) => $body,
      MultiTermQueryDocValuesTermsEnum::TermRange($inner) => $body,
      MultiTermQueryDocValuesTermsEnum::Wildcard($inner) => $body,
      #[cfg(test)]
      MultiTermQueryDocValuesTermsEnum::BoostChecking($inner) => $body,
      #[cfg(test)]
      MultiTermQueryDocValuesTermsEnum::DumbPrefix($inner) => $body,
      #[cfg(test)]
      MultiTermQueryDocValuesTermsEnum::DumbRegexp($inner) => $body,
    }
  }};
}

impl<S> BytesRefIterator for MultiTermQueryDocValuesTermsEnum<S>
where
  S: SortedSetDocValues,
{
  fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.next())
  }
}

impl<S> TermsEnum for MultiTermQueryDocValuesTermsEnum<S>
where
  S: SortedSetDocValues,
{
  type AttributeSource<'a>
    = &'a DummyAttributeSource
  where
    Self: 'a;
  type AttributeSourceMut<'a>
    = &'a mut DummyAttributeSource
  where
    Self: 'a;

  fn attributes(&self) -> Result<Self::AttributeSource<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn attributes_mut(&mut self) -> Result<Self::AttributeSourceMut<'_>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<bool> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.seek_exact(text))
  }

  fn prepare_seek_exact(&mut self, text: &BytesRef<Vec<u8>>) -> Result<Option<()>> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.prepare_seek_exact(text))
  }

  fn get_prepare_seek_exact_status(&mut self, target: &BytesRef<Vec<u8>>) -> Result<bool> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum
      .get_prepare_seek_exact_status(target))
  }

  fn seek_ceil(&mut self, text: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.seek_ceil(text))
  }

  fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.seek_exact_with_ord(ord))
  }

  fn seek_exact_with_state(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    state: &crate::core::codecs::block_term_state::TermStateEnum,
  ) -> Result<()> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum
      .seek_exact_with_state(term, state))
  }

  fn term(&self) -> Result<Cow<'_, BytesRef<Vec<u8>>>> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.term())
  }

  fn ord(&self) -> Result<i64> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.ord())
  }

  fn doc_freq(&mut self) -> Result<i32> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.doc_freq())
  }

  fn total_term_freq(&mut self) -> Result<i64> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.total_term_freq())
  }

  type PostingsEnum = DummyPostingsEnum;

  fn postings(&mut self, reuse: Option<Self::PostingsEnum>) -> Result<Self::PostingsEnum> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.postings(reuse))
  }

  fn postings_with_flags(
    &mut self,
    reuse: Option<Self::PostingsEnum>,
    flags: i32,
  ) -> Result<Self::PostingsEnum> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum
      .postings_with_flags(reuse, flags))
  }

  type ImpactsEnum = DummyImpactsEnum;

  fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn term_state(&mut self) -> Result<crate::core::codecs::block_term_state::TermStateEnum> {
    dispatch_doc_values_terms_enum!(self, |terms_enum| terms_enum.term_state())
  }
}

impl crate::core::util::accountable::Accountable for MultiTermQueryDocValuesWrapper {
  fn ram_bytes_used(&self) -> crate::core::util::error::lucene_error::Result<i64> {
    Ok(crate::core::util::ram_usage_estimator::QUERY_DEFAULT_RAM_BYTES_USED)
  }
}
