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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::postings_enum::{NONE, PostingsEnum};
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, TermsPostingEnum};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::abstract_multi_term_query_constant_score_wrapper::{
    RewritingWeightBase, TermAndState, WeightOrDocIdSetIterator,
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
use crate::core::search::query::QueryBase;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::score_mode::ScoreMode::CompleteNoScores;
use crate::core::search::term_query::{TermQuery, TermStatesMeta};
use crate::core::util::doc_id_set_builder::{DocIdSetBuilder, DocIdSetBuilderIterator};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};
/// This struct implements the logic behind `MultiTermQuery::CONSTANT_SCORE_BLENDED_REWRITE`.
///
/// It behaves similarly to a boolean-query-style rewrite for a limited number of the
/// highest-cost terms, while rewriting the remaining lower-cost terms into a filter bitset.
pub(crate) struct MultiTermQueryConstantScoreBlendedWrapper;

#[derive(Default, Clone)]
pub struct BlendedRewritingWeight;
impl RewritingWeightBase for BlendedRewritingWeight {
    type Iter<T>
        = DocIdSetIteratorEnum2<
        DummyDISI,
        DisjunctionDISIApproximation<
            ConstantScoreScorer<
                DocIdSetIteratorEnum2<DocIdSetBuilderIterator, TermsPostingEnum<T>>,
                DummyTwoPhaseIterator,
            >,
        >,
    >
    where
        T: Terms,
        TermsPostingEnum<T>: 'static;

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
        TermsPostingEnum<T>: 'static,
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
                let meta = TermStatesMeta::new(
                    context.ord,
                    doc_freq,
                    terms_enum.total_term_freq()?,
                    terms_enum.term_state()?,
                    searcher.get_top_reader_context().base().identity.clone(),
                );

                let term = Term::new(field, terms_enum.term()?.into_owned());
                let tq = TermQuery::with_term_state(term, Some(meta));
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
