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
use crate::core::index::postings_enum::NONE;
use crate::core::index::term::Term;
use crate::core::index::terms::{Terms, TermsPosting};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::abstract_multi_term_query_constant_score_wrapper::{
    RewritingWeight, RewritingWeightBase, TermAndState, WeightOrDocIdSetIterator,
};
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIteratorEnum2;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::MultiTermQueryEnum;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_query::{TermQuery, TermStatesMeta};
use crate::core::util::HasIdentity;
use crate::core::util::doc_id_set_builder::{DocIdSetBuilder, DocIdSetBuilderIterator};
use crate::core::util::error::lucene_error::Result;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

/// This struct implements the logic behind `MultiTermQuery::ConstantScoreRewrite`.
///
/// It attempts to rewrite per-segment into a boolean query that produces a
/// constant score. If that is not possible, it falls back to accumulating
/// matches into a bit set and building a `Scorer` on top of that bit set.
#[derive(Clone)]
pub struct MultiTermQueryConstantScoreWrapper {
    q: MultiTermQueryEnum,
    id: Identity,
}
impl MultiTermQueryConstantScoreWrapper {
    pub fn new(q: MultiTermQueryEnum) -> Self {
        Self {
            q,
            id: Identity::new(),
        }
    }
}

impl Debug for MultiTermQueryConstantScoreWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.as_string("") {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl HasIdentity for MultiTermQueryConstantScoreWrapper {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

impl QueryBase for MultiTermQueryConstantScoreWrapper {
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
        IRCLeafReader<IRC>: 'static,
    {
        let sub = StandardRewritingWeight;
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
impl Hash for MultiTermQueryConstantScoreWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.q.hash(state);
    }
}
impl PartialEq for MultiTermQueryConstantScoreWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.q == other.q
    }
}
impl Eq for MultiTermQueryConstantScoreWrapper {}
#[derive(Default, Clone)]
pub struct StandardRewritingWeight;
impl RewritingWeightBase for StandardRewritingWeight {
    type Iter<T>
        = DocIdSetIteratorEnum2<DummyDISI, DocIdSetBuilderIterator>
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
        let mut builder = DocIdSetBuilder::from_terms(max_doc, terms)?;

        let mut docs = None;

        // Handle the already-collected terms:
        if !collected_terms.is_empty() {
            let mut terms_enum2 = terms.iterator()?;
            for t in collected_terms.iter() {
                terms_enum2.seek_exact_with_state(&t.term, &t.state)?;
                let mut pe = terms_enum2.postings_with_flags(docs, NONE as i32)?;
                builder.add_disi(&mut pe)?;
                docs = Some(pe);
            }
        }

        // Then keep filling the bit set with remaining terms:
        loop {
            let mut pe = terms_enum.postings_with_flags(docs, NONE as i32)?;
            // If a term contains all docs with a value for the specified field, we can discard the
            // other terms and just use the dense term's postings:
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
                return Ok(WeightOrDocIdSetIterator::from_weight(weight));
            }

            builder.add_disi(&mut pe)?;
            docs = Some(pe);

            if terms_enum.next()?.is_none() {
                break;
            }
        }

        let iterator = builder.build()?.iterator()?;

        Ok(WeightOrDocIdSetIterator::from_iterator(
            DocIdSetIteratorEnum2::B(iterator),
        ))
    }
}
