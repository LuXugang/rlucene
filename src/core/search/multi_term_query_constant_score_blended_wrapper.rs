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
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::postings_enum::{NONE, PostingsEnum};
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::abstract_multi_term_query_constant_score_wrapper::{
    RewritingWeight, RewritingWeightBase, TermAndState, WeightOrDocIdSetIterator,
};
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::disi_priority_queue::DisiPriorityQueue;
use crate::core::search::disi_wrapper::DisiWrapper;
use crate::core::search::disjunction_disi_approximation::DisjunctionDISIApproximation;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, DocIdSetIteratorEnum2};
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::MultiTermQueryEnum;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::CompleteNoScores;
use crate::core::search::term_query::TermQuery;
use crate::core::util::HasIdentity;
use crate::core::util::doc_id_set_builder::{DocIdSetBuilder, DocIdSetBuilderIterator};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

/// This struct implements the logic behind `MultiTermQuery::ConstantScoreBlendedRewrite`.
///
/// It behaves similarly to a boolean-query-style rewrite for a limited number of the
/// highest-cost terms, while rewriting the remaining lower-cost terms into a filter bitset.
#[derive(Clone)]
pub struct MultiTermQueryConstantScoreBlendedWrapper {
    q: MultiTermQueryEnum,
    id: Identity,
}
impl MultiTermQueryConstantScoreBlendedWrapper {
    pub fn new(q: MultiTermQueryEnum) -> Self {
        Self {
            q,
            id: Identity::new(),
        }
    }
}

impl Debug for MultiTermQueryConstantScoreBlendedWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.as_string("") {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl HasIdentity for MultiTermQueryConstantScoreBlendedWrapper {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for MultiTermQueryConstantScoreBlendedWrapper {
    fn as_string(&self, field: &str) -> Result<String> {
        self.q.as_string(field)
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
        let sub = BlendedRewritingWeight;
        match self.q {
            MultiTermQueryEnum::Prefix(q) => Ok(Box::new(RewritingWeight::new(
                boost,
                *score_mode,
                q,
                sub.into(),
            ))),
            MultiTermQueryEnum::TermRange(q) => Ok(Box::new(RewritingWeight::new(
                boost,
                *score_mode,
                q,
                sub.into(),
            ))),
            MultiTermQueryEnum::Automaton(q) => Ok(Box::new(RewritingWeight::new(
                boost,
                *score_mode,
                q,
                sub.into(),
            ))),
            MultiTermQueryEnum::Wildcard(q) => Ok(Box::new(RewritingWeight::new(
                boost,
                *score_mode,
                q,
                sub.into(),
            ))),
            MultiTermQueryEnum::Regexp(q) => Ok(Box::new(RewritingWeight::new(
                boost,
                *score_mode,
                q,
                sub.into(),
            ))),
        }
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}
impl Hash for MultiTermQueryConstantScoreBlendedWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.q.hash(state);
    }
}
impl PartialEq for MultiTermQueryConstantScoreBlendedWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.q == other.q
    }
}
impl Eq for MultiTermQueryConstantScoreBlendedWrapper {}

#[derive(Default, Clone)]
pub struct BlendedRewritingWeight;
impl RewritingWeightBase for BlendedRewritingWeight {
    type Iter<T>
        = DocIdSetIteratorEnum2<
        DummyDISI,
        DisjunctionDISIApproximation<
            ConstantScoreScorer<
                DocIdSetIteratorEnum2<DocIdSetBuilderIterator, TermsPosting<T>>,
                DummyTwoPhaseIterator,
            >,
        >,
    >
    where
        T: Terms,
        TermsPosting<T>: 'static;

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
        IRC: IndexReaderContext,
        TermsPosting<T>: 'static,
    {
        let max_doc = context.reader().max_doc()?;
        let mut other_terms = DocIdSetBuilder::from_terms(max_doc, terms)?;
        let cmp = PostingsEnumCmp::new(vec![]);
        let mut high_frequency_terms = PriorityQueue::new(collected_terms.len(), cmp)?;
        // Handle the already-collected terms:
        let mut reuse = None;
        if !collected_terms.is_empty() {
            let mut terms_enum2 = terms.iterator()?;
            for t in collected_terms.iter() {
                terms_enum2.seek_exact_with_state(&t.term, &t.state)?;
                let mut pe = terms_enum2.postings_with_flags(reuse, NONE as i32)?;
                if t.doc_freq <= POSTINGS_PRE_PROCESS_THRESHOLD {
                    other_terms.add_disi(&mut pe)?;
                    reuse = Some(pe);
                } else {
                    high_frequency_terms.compare.postings_enum.push(Some(pe));
                    let idx = high_frequency_terms.compare.postings_enum.len();
                    high_frequency_terms.add(idx)?;
                    reuse = None;
                }
            }
        }
        // Then collect remaining terms:
        loop {
            let mut pe = terms_enum.postings_with_flags(reuse, NONE as i32)?;
            let doc_freq = terms_enum.doc_freq()?;

            if field_doc_count == doc_freq {
                let mut term_states = TermStates::new(searcher.get_top_reader_context())?;
                term_states.register_with_stats(
                    terms_enum.term_state()?,
                    context.ord,
                    doc_freq,
                    terms_enum.total_term_freq()?,
                );

                let term = Term::new(field, terms_enum.term()?.into_owned());
                let tq = TermQuery::with_term_state(term, Some(term_states));
                let q = ConstantScoreQuery::new(Box::new(tq.into()));

                let rewritten = searcher.rewrite(q)?;
                let weight = rewritten.create_weight(searcher, score_mode, score)?;
                let v = WeightOrDocIdSetIterator::from_weight(weight);
                return Ok(v);
            }

            if doc_freq <= POSTINGS_PRE_PROCESS_THRESHOLD {
                other_terms.add_disi(&mut pe)?;
                reuse = Some(pe);
            } else {
                high_frequency_terms.compare.postings_enum.push(Some(pe));
                let idx = high_frequency_terms.compare.postings_enum.len();
                let dropped = high_frequency_terms.insert_with_overflow(idx)?;

                if let Some(dropped_idx) = dropped {
                    let mut dropped_pe = high_frequency_terms.compare.postings_enum[dropped_idx]
                        .take()
                        .ok_or_else(|| LuceneError::illegal_state("posting enum is None"))?;
                    other_terms.add_disi(&mut dropped_pe)?;
                    // Reuse the postings that drop out of the PQ. Note that `dropped` will be null here
                    // if nothing is evicted, meaning we will _not_ reuse any postings (which is intentional
                    // since we can't reuse postings that are in the PQ).
                    reuse = Some(dropped_pe);
                } else {
                    reuse = None;
                }
            }

            if terms_enum.next()?.is_none() {
                break;
            }
        }
        let size = high_frequency_terms.size() + 1;
        let mut subs = DisiPriorityQueue::new(size);

        let mut all_scorers = Vec::with_capacity(size);
        for (idx, pe) in high_frequency_terms
            .compare
            .postings_enum
            .into_iter()
            .flatten()
            .enumerate()
        {
            let scorer = wrap_with_dummy_scorer(DocIdSetIteratorEnum2::B(pe));
            all_scorers.push(DisiWrapper::new(scorer)?);
            subs.add(idx, all_scorers.as_slice());
        }
        let scorer =
            wrap_with_dummy_scorer(DocIdSetIteratorEnum2::A(other_terms.build()?.iterator()?));
        all_scorers.push(DisiWrapper::new(scorer)?);
        let len = all_scorers.len() - 1;
        subs.add(len, all_scorers.as_slice());
        let v = WeightOrDocIdSetIterator::from_iterator(DocIdSetIteratorEnum2::B(
            DisjunctionDISIApproximation::new(subs, all_scorers),
        ));
        Ok(v)
    }
}
/// Wrap a DISI with a "dummy" scorer so we can directly reuse `DisiWrapper` and
/// `DisjunctionDISIApproximation` without modification.
///
/// This is merely a convenient vehicle to place the DISI into the priority queue
/// consumed by `DisjunctionDISIApproximation`.
///
/// The actual `Scorer` ultimately returned by the weight provides the real constant
/// boost and reflects the effective score mode.
fn wrap_with_dummy_scorer<D>(disi: D) -> ConstantScoreScorer<D, DummyTwoPhaseIterator>
where
    D: DocIdSetIterator,
{
    ConstantScoreScorer::from_disi(1.0f32, CompleteNoScores, disi)
}
const POSTINGS_PRE_PROCESS_THRESHOLD: i32 = 16;
struct PostingsEnumCmp<PE>
where
    PE: PostingsEnum,
{
    // for easy taken
    postings_enum: Vec<Option<PE>>,
}
impl<PE> PostingsEnumCmp<PE>
where
    PE: PostingsEnum,
{
    fn new(postings_enum: Vec<Option<PE>>) -> Self {
        Self { postings_enum }
    }
}
impl<PE> Compare<usize> for PostingsEnumCmp<PE>
where
    PE: PostingsEnum,
{
    fn less_than(&self, a: &usize, b: &usize) -> Result<bool> {
        let l = self.postings_enum[*a]
            .as_ref()
            .ok_or_else(|| LuceneError::illegal_state("posting enum is None"))?;
        let r = self.postings_enum[*b]
            .as_ref()
            .ok_or_else(|| LuceneError::illegal_state("posting enum is None"))?;
        Ok(l.cost()? < r.cost()?)
    }
}
