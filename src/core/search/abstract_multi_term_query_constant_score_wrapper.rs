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
use crate::core::index::index_reader_context::{IRCLeafReader, IRCTerm, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStateEnum;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::{IndexSearcher, get_max_clause_count};
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::multi_term_query::MultiTermQuery;
use crate::core::search::query::{
    Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::term_query::{TermQuery, TermStatesMeta};
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::Result;
use std::marker::PhantomData;
use std::sync::Arc;

pub(crate) const BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD: usize = 16;
pub struct AbstractMultiTermQueryConstantScoreWrapper {}

pub struct RewritingWeight<IRC, Q>
where
    IRC: IndexReaderContext,
    Q: MultiTermQuery,
{
    score_mode: ScoreMode,
    q: Q,
    base: ConstantScoreWeight,
    _irc: PhantomData<IRC>,
}
impl<IRC, Q> RewritingWeight<IRC, Q>
where
    IRC: IndexReaderContext,
    Q: MultiTermQuery,
{
    fn collect_terms<TE>(
        field_doc_count: i32,
        terms_enum: &mut TE,
        terms: &mut Vec<TermAndState>,
    ) -> Result<bool>
    where
        TE: TermsEnum,
    {
        let threshold = std::cmp::min(BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD, get_max_clause_count());

        for _ in 0..threshold {
            let term = match terms_enum.next()? {
                Some(t) => t.into_owned(),
                None => return Ok(true),
            };

            let state = terms_enum.term_state()?;
            let doc_freq = terms_enum.doc_freq()?;
            let total_term_freq = terms_enum.total_term_freq()?;

            let term_and_state = TermAndState::new(term, state, doc_freq, total_term_freq);

            if field_doc_count == doc_freq {
                // If the term contains every document with a value for the field, we can ignore all
                // other terms:
                terms.clear();
                terms.push(term_and_state);
                return Ok(true);
            }

            terms.push(term_and_state);
        }

        Ok(terms_enum.next()?.is_none())
    }
}

impl<IRC, Q> SegmentCacheable for RewritingWeight<IRC, Q>
where
    IRC: IndexReaderContext,
    Q: MultiTermQuery,
{
    type IRC = IRC;

    fn is_cacheable(&self, _ctx: &LeafReaderContext<IRCLeafReader<Self::IRC>>) -> Result<bool> {
        Ok(true)
    }
}

impl<IRC, Q> Weight for RewritingWeight<IRC, Q>
where
    IRC: IndexReaderContext,
    Q: MultiTermQuery,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<Self::IRC>>,
        _doc: i32,
        _searcher: &IndexSearcher<Self::IRC>,
    ) -> Result<Option<Self::Matches>> {
        todo!()
    }

    fn explain(
        &self,
        _context: &LeafReaderContext<IRCLeafReader<Self::IRC>>,
        _doc: i32,
        _searcher: &IndexSearcher<Self::IRC>,
    ) -> Result<Explanation> {
        todo!()
    }

    fn get_query(&self) -> Arc<Query> {
        todo!()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRCLeafReader<Self::IRC>>,
        _searcher: &IndexSearcher<Self::IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        let terms = match context.reader().terms(self.q.get_field())? {
            Some(t) => t,
            None => return Ok(None),
        };

        let field_doc_count = terms.get_doc_count()?;
        let mut terms_enum = self.q.get_terms_enum(&terms)?;
        let mut collected_terms = Vec::new();

        let collect_result =
            Self::collect_terms(field_doc_count, &mut terms_enum, &mut collected_terms)?;

        let _cost = if collect_result {
            if collected_terms.is_empty() {
                return Ok(None);
            }

            let mut sum_term_cost: i64 = 0;
            for collected_term in &collected_terms {
                sum_term_cost += collected_term.doc_freq as i64;
            }
            sum_term_cost
        } else {
            estimate_cost(&terms, self.q.get_terms_count())?
        };
        todo!()
    }
}
fn rewrite_as_boolean_query<IRC>(
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    collected_terms: &[TermAndState],
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    score: f32,
    field: &str,
) -> Result<WeightOrDocIdSetIterator<IRC, DummyDISI>>
where
    IRC: IndexReaderContext,
{
    let mut builder = Builder::new();

    for t in collected_terms.iter() {
        let term = Term::new(field, t.term.clone());
        let meta = TermStatesMeta::new(
            context.ord,
            t.doc_freq,
            t.total_term_freq,
            t.state.clone(),
            searcher.get_top_reader_context().base().identity.clone(),
        );
        let tq = TermQuery::with_term_state(term, Some(meta));

        builder.add(tq, Occur::Should)?;
    }

    let bq = builder.build();
    let query = ConstantScoreQuery::new(Box::new(bq.into()));

    let rewritten = searcher.rewrite(query)?;
    let weight = rewritten.create_weight(searcher, score_mode, score)?;

    Ok(WeightOrDocIdSetIterator::new_weight(weight))
}
/// Estimate the cost. If the MTQ can provide its term count, we can do a better job
/// estimating.
/// Cost estimation reasoning is:
/// 1. If we don't know how many query terms there are, we assume that every term could be
///    in the MTQ and estimate the work as the total docs across all terms.
/// 2. If we know how many query terms there are...
///    2a. Assume every query term matches at least one document (queryTermsCount).
///    2b. Determine the total number of docs beyond the first one for each term.
///        That count provides a ceiling on the number of extra docs that could match beyond
///        that first one. (We omit the first since it's already been counted in 2a).
/// See: LUCENE-10207
pub(crate) fn estimate_cost<T>(terms: &T, query_terms_count: i64) -> Result<i64>
where
    T: Terms,
{
    let cost: i64;
    if query_terms_count == -1 {
        cost = terms.get_sum_doc_freq()?;
    } else {
        let mut potential_extra_cost = terms.get_sum_doc_freq()?;
        let indexed_term_count = terms.size()?;
        if indexed_term_count != -1 {
            potential_extra_cost -= indexed_term_count;
        }
        cost = query_terms_count + potential_extra_cost;
    }
    Ok(cost)
}
pub trait RewritingWeightBase {
    type Iter<T>: DocIdSetIterator
    where
        T: Terms,
        <<T as Terms>::TermsEnum as TermsEnum>::PostingsEnum: 'static;
    fn rewrite_inner<T, TE, IRC>(
        &self,
        field_doc_count: i32,
        terms: &mut T,
        terms_enum: &mut TE,
        collected_terms: &[TermAndState],
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        searcher: &IndexSearcher<IRC>,
        field: &str,
        score_mode: &ScoreMode,
        score: f32,
    ) -> Result<WeightOrDocIdSetIterator<IRC, Self::Iter<T>>>
    where
        T: Terms,
        TE: TermsEnum<PostingsEnum = <T::TermsEnum as TermsEnum>::PostingsEnum>,
        IRC: IndexReaderContext;
}
pub struct ScorerSupplierImpl<IRC, TE>
where
    IRC: IndexReaderContext,
    TE: TermsEnum,
{
    cost: i64,
    score_mode: ScoreMode,
    terms: IRCTerm<IRC>,
    collected_terms: Vec<TermAndState>,
    terms_enum: TE,
    score: f32,
    collect_result: bool,
    field: String,
}
impl<IRC, TE> ScorerSupplier for ScorerSupplierImpl<IRC, TE>
where
    IRC: IndexReaderContext,
    TE: TermsEnum,
{
    type Scorer = QueryWeightSsScorer;
    type BulkScorer = QueryWeightSsBulkScorer;
    type IRC = IRC;

    fn get(
        &mut self,
        _lead_cost: i64,
        context: &LeafReaderContext<IRCLeafReader<Self::IRC>>,
        searcher: &IndexSearcher<Self::IRC>,
    ) -> Result<Self::Scorer> {
        match self.collect_result {
            true => {
                let _v = rewrite_as_boolean_query(
                    context,
                    self.collected_terms.as_slice(),
                    searcher,
                    &self.score_mode,
                    self.score,
                    &self.field,
                )?;
                // let scorer = match v.weight {
                //     Some(weight) => match weight.scorer(context, searcher)? {
                //         Some(scorer) => ScorerEnum3::A(scorer),
                //         None => {
                //             let s = ConstantScoreScorer::from_disi(
                //                 self.score,
                //                 self.score_mode,
                //                 EmptyDISI::default(),
                //             );
                //             ScorerEnum3::C(s)
                //         },
                //     },
                //     None => return Err(LuceneError::illegal_state("weight is None")),
                // };
                todo!()
            },
            false => {
                todo!()
            },
        }
        todo!()
    }

    fn bulk_scorer(
        &mut self,
        _context: &LeafReaderContext<IRCLeafReader<Self::IRC>>,
        _searcher: &IndexSearcher<Self::IRC>,
    ) -> Result<Option<Self::BulkScorer>> {
        todo!()
    }

    fn cost(
        &mut self,
        _context: &LeafReaderContext<IRCLeafReader<Self::IRC>>,
        _searcher: &IndexSearcher<Self::IRC>,
    ) -> Result<i64> {
        Ok(self.cost)
    }
}
pub(crate) struct TermAndState {
    pub(crate) term: BytesRef<Vec<u8>>,
    pub(crate) state: TermStateEnum,
    pub(crate) doc_freq: i32,
    pub(crate) total_term_freq: i64,
}

impl TermAndState {
    pub(crate) fn new(
        term: BytesRef<Vec<u8>>,
        state: TermStateEnum,
        doc_freq: i32,
        total_term_freq: i64,
    ) -> Self {
        Self {
            term,
            state,
            doc_freq,
            total_term_freq,
        }
    }
}
pub(crate) struct WeightOrDocIdSetIterator<IRC, D>
where
    IRC: IndexReaderContext,
    D: DocIdSetIterator,
{
    pub(crate) weight: Option<QueryWeight<IRC>>,
    pub(crate) iterator: Option<D>,
}

impl<IRC, D> WeightOrDocIdSetIterator<IRC, D>
where
    IRC: IndexReaderContext,
    D: DocIdSetIterator,
{
    pub(crate) fn new_weight(weight: QueryWeight<IRC>) -> Self {
        Self {
            weight: Some(weight),
            iterator: None,
        }
    }

    pub(crate) fn new_iterator(iterator: D) -> Self {
        Self {
            weight: None,
            iterator: Some(iterator),
        }
    }
}
