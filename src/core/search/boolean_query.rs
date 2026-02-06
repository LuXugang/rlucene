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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LRTermState;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::boolean_clause::{BooleanClause, Occur};
use crate::core::search::boolean_weight::{BooleanWeight, WeightedBooleanClause};
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::{IndexSearcher, get_max_clause_count};
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};

/// A query that matches documents matching boolean combinations of other queries, e.g.
/// [`TermQuery`](crate::core::search::term_query::TermQuery)s, [`PhraseQuery`](crate::core::search::phrase_query::PhraseQuery)s or other [`BooleanQuery`]s.
#[derive(Debug, Clone)]
pub struct BooleanQuery {
    id: Identity,
    minimum_number_should_match: i32,
    pub(crate) clauses: Vec<BooleanClause>,
    clause_sets: HashMap<Occur, Vec<usize>>,
}

impl BooleanQuery {
    fn new(minimum_number_should_match: i32, clauses: Vec<BooleanClause>) -> BooleanQuery {
        let mut clause_sets: HashMap<Occur, Vec<usize>> = HashMap::new();

        for (idx, clause) in clauses.iter().enumerate() {
            let occur = clause.occur;

            match occur {
                Occur::Should | Occur::Must => {
                    clause_sets.entry(occur).or_default().push(idx);
                },

                Occur::Filter | Occur::MustNot => {
                    let indices = clause_sets.entry(occur).or_default();

                    let exists = indices.iter().any(|&i| clauses[i].query == clause.query);
                    if !exists {
                        indices.push(idx);
                    }
                },
            }
        }

        BooleanQuery {
            id: Identity::new(),
            minimum_number_should_match,
            clauses,
            clause_sets,
        }
    }

    /// Gets the minimum number of the optional [`BooleanClause`]s which must be satisfied.
    pub fn get_minimum_number_should_match(&self) -> i32 {
        self.minimum_number_should_match
    }

    /// Return a slice of the clauses of this [`BooleanQuery`].
    pub fn clauses(&self) -> &[BooleanClause] {
        &self.clauses
    }

    /// Return the collection of queries for the given [`Occur`].
    pub fn get_clauses_idx(&self, occur: Occur) -> &[usize] {
        self.clause_sets
            .get(&occur)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    /// Whether this query is a pure disjunction, ie. it only has SHOULD clauses and it is enough for
    /// a single clause to match for this boolean query to match.
    pub(crate) fn is_pure_disjunction(&self) -> bool {
        self.clauses.len() == self.get_clauses_idx(Occur::Should).len()
            && self.minimum_number_should_match <= 1
    }

    /// Whether this query is a two clause disjunction with two term query clauses.
    pub(crate) fn is_two_clause_pure_disjunction_with_terms(&self) -> bool {
        self.clauses.len() == 2
            && self.is_pure_disjunction()
            && matches!(self.clauses[0].query, Query::Term(_))
            && matches!(self.clauses[1].query, Query::Term(_))
    }
    pub fn rewrite_no_scoring(self) -> Result<Query> {
        let mut actually_rewritten = false;

        let mut new_query = Builder::new();
        new_query.set_minimum_number_should_match(self.get_minimum_number_should_match());

        let keep_should = self.get_minimum_number_should_match() > 0 || {
            let must = self
                .clause_sets
                .get(&Occur::Must)
                .map(|v| v.len())
                .unwrap_or(0);
            let filter = self
                .clause_sets
                .get(&Occur::Filter)
                .map(|v| v.len())
                .unwrap_or(0);
            must + filter == 0
        };

        for clause in &self.clauses {
            let mut rewritten = clause.query.clone();
            // NOTE: rewritingNoScoring() should not call rewrite(), otherwise this
            // method could run in exponential time with the depth of the query as
            // every new level would rewrite 2x more than its parent level.
            match rewritten {
                Query::Boost(b) => {
                    rewritten = b.into_inner();
                },
                Query::ConstantScore(cs) => {
                    rewritten = cs.into_inner();
                },
                Query::Boolean(b) => {
                    rewritten = b.rewrite_no_scoring()?;
                },
                _ => {},
            }

            match clause.occur {
                Occur::Should if !keep_should => {
                    actually_rewritten = true;
                },
                Occur::Must => {
                    new_query.add_query(rewritten, Occur::Filter)?;
                    actually_rewritten = true;
                },
                _ if clause.query.identity() != rewritten.identity() => {
                    new_query.add_query(rewritten, clause.occur)?;
                    actually_rewritten = true;
                },
                _ => {
                    new_query.add_clause(clause.clone())?;
                },
            }
        }

        if !actually_rewritten {
            return Ok(self.into());
        }

        Ok(new_query.build().into())
    }
    fn as_counts_map(&self) -> HashMap<(Occur, &Query), usize> {
        let mut m = HashMap::new();
        for (&occur, indices) in &self.clause_sets {
            for &idx in indices {
                let q = &self.clauses[idx].query;
                *m.entry((occur, q)).or_insert(0) += 1;
            }
        }
        m
    }
    pub(crate) fn raw_weight<IRC, QC>(
        self,
        searcher: &IndexSearcher<IRC, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        _per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<BooleanWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        let similarity = searcher.get_similarity();

        let mut weighted_clauses = Vec::with_capacity(self.clauses().len());
        for c in self.clone().clauses {
            let clause_score_mode = if c.is_scoring() {
                score_mode
            } else {
                &ScoreMode::CompleteNoScores
            };
            let weight = c
                .query
                .clone()
                .create_weight(searcher, clause_score_mode, boost, None)?;

            weighted_clauses.push(WeightedBooleanClause::new(c, weight));
        }
        Ok(BooleanWeight {
            similarity,
            weighted_clauses,
            query: self,
            score_mode: *score_mode,
        })
    }
}
impl Hash for BooleanQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.minimum_number_should_match.hash(state);
        let mut hs = Vec::new();
        for (&occur, indices) in &self.clause_sets {
            for &idx in indices {
                let mut h = DefaultHasher::new();
                occur.hash(&mut h);
                self.clauses[idx].query.hash(&mut h);
                hs.push(h.finish());
            }
        }
        hs.sort_unstable();
        hs.hash(state);
    }
}

impl PartialEq for BooleanQuery {
    fn eq(&self, other: &Self) -> bool {
        self.minimum_number_should_match == other.minimum_number_should_match
            && self.as_counts_map() == other.as_counts_map()
    }
}

impl Eq for BooleanQuery {}

impl HasIdentity for BooleanQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}
impl QueryBase for BooleanQuery {
    fn as_string(&self, field: &str) -> String {
        let mut buffer = String::new();
        let need_parens = self.minimum_number_should_match > 0;

        if need_parens {
            buffer.push('(');
        }

        for (i, clause) in self.clauses.iter().enumerate() {
            buffer.push_str(&clause.occur.to_string());

            buffer.push_str(&clause.query.as_string(field));

            if i != self.clauses.len() - 1 {
                buffer.push(' ');
            }
        }

        if need_parens {
            buffer.push(')');
        }

        if self.minimum_number_should_match > 0 {
            buffer.push('~');
            buffer.push_str(&self.minimum_number_should_match.to_string());
        }

        buffer
    }

    fn create_weight<IRC, QC>(
        self,
        searcher: &IndexSearcher<IRC, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
        Self: Sized,
        <IRC as IndexReaderContext>::LeafReader: 'static,
    {
        let weight = self.raw_weight(searcher, score_mode, boost, per_reader_term_state)?;
        Ok(Box::new(weight))
    }

    fn rewrite<IRC, QC>(self, searcher: &IndexSearcher<IRC, QC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        QC: QueryCache,
        Self: Sized,
    {
        if self.clauses.is_empty() {
            return Ok(MatchNoDocsQuery::with_message("empty BooleanQuery").into());
        }
        let must_not_len = self
            .clause_sets
            .get(&Occur::MustNot)
            .map(|v| v.len())
            .unwrap_or(0);
        if self.clauses.len() == must_not_len {
            return Ok(Query::MatchNoDoc(MatchNoDocsQuery::with_message(
                "pure negative BooleanQuery",
            )));
        }

        // optimize 1-clause queries
        if self.clauses.len() == 1 {
            let clause = &self.clauses[0];

            if self.minimum_number_should_match == 1 && clause.occur == Occur::Should {
                return Ok(clause.query.clone());
            } else if self.minimum_number_should_match == 0 {
                match clause.occur {
                    Occur::Should | Occur::Must => {
                        return Ok(clause.query.clone());
                    },
                    Occur::Filter => {
                        // no scoring clauses, so return a score of 0
                        return Ok(Query::Boost(BoostQuery::new(
                            Query::ConstantScore(ConstantScoreQuery::new(clause.query.clone())),
                            0.0,
                        )?));
                    },
                    Occur::MustNot => return Err(LuceneError::illegal_state("should not be here")),
                }
            }
        }
        // recursively rewrite
        {
            let mut builder = Builder::new();
            builder.set_minimum_number_should_match(self.get_minimum_number_should_match());
            let mut actually_rewritten = false;

            for clause in &self.clauses {
                let query = clause.query.clone();
                let occur = clause.occur;
                let query_id = query.identity().clone();
                let is_match_no_doc = matches!(query, Query::MatchNoDoc(_));
                let rewritten = match occur {
                    Occur::Filter | Occur::MustNot => {
                        // Clauses that are not involved in scoring can get some extra simplifications
                        let rewritten =
                            Query::ConstantScore(ConstantScoreQuery::new(query.clone()))
                                .rewrite(searcher)?;
                        match rewritten {
                            Query::ConstantScore(cs) => cs.into_inner(),
                            q => q,
                        }
                    },
                    _ => query.rewrite(searcher)?,
                };

                if rewritten.identity() != &query_id || is_match_no_doc {
                    // rewrite clause
                    actually_rewritten = true;

                    if matches!(rewritten, Query::MatchNoDoc(_)) {
                        match occur {
                            Occur::Should | Occur::MustNot => {
                                // the clause can be safely ignored
                            },
                            Occur::Must | Occur::Filter => {
                                return Ok(rewritten);
                            },
                        }
                    } else {
                        builder.add_query(rewritten, occur)?;
                    }
                } else {
                    // leave as-is
                    builder.add_clause(clause.clone())?;
                }
            }

            if actually_rewritten {
                return Ok(builder.build().into());
            }
        }
        // remove duplicate FILTER and MUST_NOT clauses
        {
            let mut clause_count = 0;
            for queries in self.clause_sets.values() {
                clause_count += queries.len();
            }

            if clause_count != self.clauses.len() {
                // since clause_sets implicitly deduplicates FILTER and MUST_NOT clauses,
                // this means there were duplicates
                let mut rewritten = Builder::new();
                rewritten.set_minimum_number_should_match(self.minimum_number_should_match);

                for (occur, indices) in &self.clause_sets {
                    for &idx in indices {
                        let clause = &self.clauses[idx];
                        rewritten.add_query(clause.query.clone(), *occur)?;
                    }
                }

                return Ok(rewritten.build().into());
            }
        }

        // Check whether some clauses are both required and excluded
        let must_not = self
            .clause_sets
            .get(&Occur::MustNot)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if !must_not.is_empty() {
            let must = self
                .clause_sets
                .get(&Occur::Must)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let filter = self
                .clause_sets
                .get(&Occur::Filter)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if must_not.iter().any(|&mn_idx| {
                let q = &self.clauses[mn_idx].query;
                must.iter().any(|&m_idx| self.clauses[m_idx].query == *q)
                    || filter.iter().any(|&f_idx| self.clauses[f_idx].query == *q)
            }) {
                return Ok(MatchNoDocsQuery::with_message(
                    "FILTER or MUST clause also in MUST_NOT",
                )
                .into());
            }

            if must_not
                .iter()
                .any(|&idx| matches!(self.clauses[idx].query, Query::MatchAll(_)))
            {
                return Ok(
                    MatchNoDocsQuery::with_message("MUST_NOT clause is MatchAllDocsQuery").into(),
                );
            }
        }

        // remove FILTER clauses that are also MUST clauses or that match all documents
        let filter_idx = self
            .clause_sets
            .get(&Occur::Filter)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if !filter_idx.is_empty() {
            let must_indices = self
                .clause_sets
                .get(&Occur::Must)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let mut new_filter_ixd: Vec<usize> = filter_idx.to_vec();
            let mut modified = false;

            // remove MatchAllDocsQuery if needed
            if new_filter_ixd.len() > 1 || !must_indices.is_empty() {
                let before = new_filter_ixd.len();
                new_filter_ixd
                    .retain(|&idx| !matches!(self.clauses[idx].query, Query::MatchAll(_)));
                modified |= new_filter_ixd.len() != before;
            }

            let before = new_filter_ixd.len();
            new_filter_ixd.retain(|&f_idx| {
                let fq = &self.clauses[f_idx].query;
                !must_indices
                    .iter()
                    .any(|&m_idx| self.clauses[m_idx].query == *fq)
            });

            modified |= new_filter_ixd.len() != before;

            if modified {
                let mut builder = Builder::new();
                builder.set_minimum_number_should_match(self.get_minimum_number_should_match());

                for clause in &self.clauses {
                    if clause.occur != Occur::Filter {
                        builder.add_clause(clause.clone())?;
                    }
                }

                for idx in new_filter_ixd {
                    builder.add_query(self.clauses[idx].query.clone(), Occur::Filter)?;
                }

                return Ok(builder.build().into());
            }
        }

        // convert FILTER clauses that are also SHOULD clauses to MUST clauses
        let should_indices = self
            .clause_sets
            .get(&Occur::Should)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let filter_indices = self
            .clause_sets
            .get(&Occur::Filter)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if !should_indices.is_empty() && !filter_indices.is_empty() {
            // compute intersection by Query equality (not identity)
            let intersection: Vec<usize> = filter_indices
                .iter()
                .cloned()
                .filter(|&f_idx| {
                    let fq = &self.clauses[f_idx].query;
                    should_indices
                        .iter()
                        .any(|&s_idx| self.clauses[s_idx].query == *fq)
                })
                .collect();

            if !intersection.is_empty() {
                let mut builder = Builder::new();
                let mut min_should_match = self.get_minimum_number_should_match();

                for clause in &self.clauses {
                    let in_intersection = intersection
                        .iter()
                        .any(|&idx| self.clauses[idx].query == clause.query);

                    if in_intersection && clause.occur == Occur::Should {
                        builder.add_query(clause.query.clone(), Occur::Must)?;
                        min_should_match -= 1;
                    } else {
                        builder.add_clause(clause.clone())?;
                    }
                }

                builder.set_minimum_number_should_match(min_should_match.max(0));
                return Ok(builder.build().into());
            }
        }

        // Deduplicate SHOULD clauses by summing up their boosts
        let should_indices = self
            .clause_sets
            .get(&Occur::Should)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if !should_indices.is_empty() && self.minimum_number_should_match <= 1 {
            let mut should_clauses = HashMap::new();

            for &idx in should_indices {
                let mut query = self.clauses[idx].query.clone();
                let mut boost = 1.0;

                while let Query::Boost(bq) = query {
                    boost *= bq.get_boost() as f64;
                    query = bq.into_inner();
                }

                *should_clauses.entry(query).or_insert(0.0) += boost;
            }

            if should_clauses.len() != should_indices.len() {
                let mut builder = Builder::new();
                builder.set_minimum_number_should_match(self.minimum_number_should_match);

                for (mut query, boost) in should_clauses {
                    let boost = boost as f32;
                    if boost != 1.0 {
                        query = Query::Boost(BoostQuery::new(query, boost)?);
                    }
                    builder.add_query(query, Occur::Should)?;
                }

                for clause in &self.clauses {
                    if clause.occur != Occur::Should {
                        builder.add_clause(clause.clone())?;
                    }
                }

                return Ok(builder.build().into());
            }
        }

        // Deduplicate MUST clauses by summing up their boosts
        let must_indices = self
            .clause_sets
            .get(&Occur::Must)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if !must_indices.is_empty() {
            let mut must_clauses = HashMap::new();

            for &idx in must_indices {
                let mut query = self.clauses[idx].query.clone();
                let mut boost: f64 = 1.0;

                while let Query::Boost(bq) = query {
                    boost *= bq.get_boost() as f64;
                    query = bq.into_inner();
                }

                *must_clauses.entry(query).or_insert(0.0) += boost;
            }

            if must_clauses.len() != must_indices.len() {
                let mut builder = Builder::new();
                builder.set_minimum_number_should_match(self.minimum_number_should_match);

                for (mut query, boost) in must_clauses {
                    let boost = boost as f32;
                    if boost != 1.0 {
                        query = Query::Boost(BoostQuery::new(query, boost)?);
                    }
                    builder.add_query(query, Occur::Must)?;
                }

                for clause in &self.clauses {
                    if clause.occur != Occur::Must {
                        builder.add_clause(clause.clone())?;
                    }
                }

                return Ok(builder.build().into());
            }
        }

        // Rewrite queries whose single scoring clause is a MUST clause on a
        // MatchAllDocsQuery to a ConstantScoreQuery
        {
            let must_indices = self
                .clause_sets
                .get(&Occur::Must)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let filter_indices = self
                .clause_sets
                .get(&Occur::Filter)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if must_indices.len() == 1 && !filter_indices.is_empty() {
                let mut must = self.clauses[must_indices[0]].query.clone();
                let mut boost = 1.0f32;

                if let Query::Boost(bq) = must {
                    boost = bq.get_boost();
                    must = bq.into_inner();
                }

                if matches!(must, Query::MatchAll(_)) {
                    // our single scoring clause matches everything: rewrite to a CSQ on the filter
                    // ignore SHOULD clause for now
                    let mut builder = Builder::new();
                    for clause in &self.clauses {
                        match clause.occur {
                            Occur::Filter | Occur::MustNot => {
                                builder.add_clause(clause.clone())?;
                            },
                            Occur::Must | Occur::Should => {
                                // ignore
                            },
                        }
                    }

                    let mut rewritten = builder.build().into();
                    rewritten = Query::ConstantScore(ConstantScoreQuery::new(rewritten));

                    if boost != 1.0 {
                        rewritten = Query::Boost(BoostQuery::new(rewritten, boost)?);
                    }

                    // now add back the SHOULD clauses
                    let mut builder = Builder::new();
                    builder.set_minimum_number_should_match(self.get_minimum_number_should_match());
                    builder.add_query(rewritten, Occur::Must)?;

                    let should_indices = self
                        .clause_sets
                        .get(&Occur::Should)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    for &idx in should_indices {
                        builder.add_query(self.clauses[idx].query.clone(), Occur::Should)?;
                    }

                    return Ok(builder.build().into());
                }
            }
        }

        // Flatten nested disjunctions, this is important for block-max WAND to perform well
        if self.minimum_number_should_match <= 1 {
            let mut builder = Builder::new();
            builder.set_minimum_number_should_match(self.minimum_number_should_match);
            let mut actually_rewritten = false;

            for clause in &self.clauses {
                if clause.occur == Occur::Should {
                    if let Query::Boolean(inner) = &clause.query {
                        if inner.is_pure_disjunction() {
                            actually_rewritten = true;
                            for inner_clause in inner.clauses.iter() {
                                builder.add_clause(inner_clause.clone())?;
                            }
                        } else {
                            builder.add_clause(clause.clone())?;
                        }
                    } else {
                        builder.add_clause(clause.clone())?;
                    }
                } else {
                    builder.add_clause(clause.clone())?;
                }
            }

            if actually_rewritten {
                return Ok(builder.build().into());
            }
        }

        // Inline required / prohibited clauses. This helps run filtered conjunctive queries more
        // efficiently by providing all clauses to the block-max AND scorer.
        {
            let mut builder = Builder::new();
            builder.set_minimum_number_should_match(self.minimum_number_should_match);
            let mut actually_rewritten = false;

            for outer_clause in &self.clauses {
                if outer_clause.is_required() {
                    if let Query::Boolean(inner_query) = &outer_clause.query {
                        // Inlining prohibited clauses is not legal if the query is a pure negation, since pure
                        // negations have no matches. It works because the inner BooleanQuery would have first
                        // rewritten to a MatchNoDocsQuery if it only had prohibited clauses.
                        debug_assert!(
                            inner_query
                                .clause_sets
                                .get(&Occur::MustNot)
                                .map(|v| v.len())
                                .unwrap_or(0)
                                != inner_query.clauses.len()
                        );

                        let inner_should_len = inner_query
                            .clause_sets
                            .get(&Occur::Should)
                            .map(|v| v.len())
                            .unwrap_or(0);

                        if inner_query.get_minimum_number_should_match() == 0
                            && inner_should_len == 0
                        {
                            actually_rewritten = true;

                            for inner_clause in &inner_query.clauses {
                                let inner_occur = inner_clause.occur;

                                if inner_occur == Occur::Filter
                                    || inner_occur == Occur::MustNot
                                    || outer_clause.occur == Occur::Must
                                {
                                    builder.add_clause(inner_clause.clone())?;
                                } else {
                                    debug_assert!(outer_clause.occur == Occur::Filter);
                                    debug_assert!(inner_occur == Occur::Must);
                                    // In this case we need to change the occur of the inner query from MUST to FILTER.
                                    builder.add_query(inner_clause.query.clone(), Occur::Filter)?;
                                }
                            }
                        } else {
                            builder.add_clause(outer_clause.clone())?;
                        }
                    } else {
                        builder.add_clause(outer_clause.clone())?;
                    }
                } else {
                    builder.add_clause(outer_clause.clone())?;
                }
            }

            if actually_rewritten {
                return Ok(builder.build().into());
            }
        }
        // SHOULD clause count less than or equal to minimum_number_should_match
        // Important(this can only be processed after nested clauses have been flattened)
        {
            let should_indices = self
                .clause_sets
                .get(&Occur::Should)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let should_len = should_indices.len();

            if should_len < self.minimum_number_should_match as usize {
                return Ok(MatchNoDocsQuery::with_message(
                    "SHOULD clause count less than minimumNumberShouldMatch",
                )
                .into());
            }

            if should_len > 0 && should_len == self.minimum_number_should_match as usize {
                let mut builder = Builder::new();

                for clause in &self.clauses {
                    if clause.occur == Occur::Should {
                        builder.add_query(clause.query.clone(), Occur::Must)?;
                    } else {
                        builder.add_clause(clause.clone())?;
                    }
                }

                return Ok(builder.build().into());
            }
        }
        // Inline SHOULD clauses from the only MUST clause
        {
            let should_indices = self
                .clause_sets
                .get(&Occur::Should)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let must_indices = self
                .clause_sets
                .get(&Occur::Must)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if should_indices.is_empty() && must_indices.len() == 1 {
                let must_clause = &self.clauses[must_indices[0]];
                if let Query::Boolean(inner) = &must_clause.query {
                    let inner_should_len = inner
                        .clause_sets
                        .get(&Occur::Should)
                        .map(|v| v.len())
                        .unwrap_or(0);

                    if inner.clauses.len() == inner_should_len {
                        let mut rewritten = Builder::new();

                        for clause in &self.clauses {
                            if clause.occur != Occur::Must {
                                rewritten.add_clause(clause.clone())?;
                            }
                        }

                        for inner_clause in &inner.clauses {
                            rewritten.add_clause(inner_clause.clone())?;
                        }

                        let msm = inner.get_minimum_number_should_match().max(1);
                        rewritten.set_minimum_number_should_match(msm);

                        return Ok(rewritten.build().into());
                    }
                }
            }
        }
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

/// A builder for boolean queries
pub struct Builder {
    minimum_number_should_match: i32,
    clauses: Vec<BooleanClause>,
}
impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            minimum_number_should_match: 0,
            clauses: Vec::new(),
        }
    }
    /// Specifies a minimum number of the optional [`BooleanClause`]s which must be satisfied.
    ///
    /// By default, no optional clauses are necessary for a match (unless there are no required
    /// clauses). If this method is used, then the specified number of clauses is required.
    ///
    /// Use of this method is totally independent of specifying that any specific clauses are
    /// required (or prohibited). This number will only be compared against the number of matching
    /// optional clauses.
    /// # Parameters
    ///
    /// * `min` – the number of optional clauses that must match
    pub fn set_minimum_number_should_match(&mut self, min: i32) -> &mut Self {
        self.minimum_number_should_match = min;
        self
    }

    /// Add a new clause to this [`Builder`]. Note that the order in which clauses are added does
    /// not have any impact on matching documents or query performance.
    ///
    /// # Errors
    ///
    /// Returns [`IndexSearcherError::TooManyClauses`] if the new number of clauses exceeds
    /// the maximum clause count.
    pub fn add_clause(&mut self, clause: BooleanClause) -> Result<&mut Self> {
        // We do the final deep check for max clauses count limit during
        // `IndexSearcher::rewrite` but do this check to short circuit in case
        // a single query holds more than numClauses.
        //
        // NOTE: this is not just an early check for optimization -- it's
        // necessary to prevent run-away rewriting of bad queries from
        // creating BooleanQuery objects that might eat up all the heap.
        if self.clauses.len() >= get_max_clause_count() {
            return Err(LuceneError::too_many_clauses(""));
        }
        self.clauses.push(clause);
        Ok(self)
    }
    /// Add a collection of [`BooleanClause`]s to this [`Builder`]. Note that the order in which
    /// clauses are added does not have any impact on matching documents or query performance.
    ///
    /// # Errors
    ///
    /// Returns [`LuceneError::TooManyClauses`] if the new number of clauses exceeds
    /// the maximum clause count.
    pub fn add_all(&mut self, collection: Vec<BooleanClause>) -> Result<&mut Self> {
        let len = collection.len();

        if self.clauses.len() + len > get_max_clause_count() {
            return Err(LuceneError::too_many_clauses(""));
        }
        self.clauses.extend(collection);
        Ok(self)
    }
    /// Add a new clause to this [`Builder`]. Note that the order in which clauses are added does
    /// not have any impact on matching documents or query performance.
    ///
    /// # Errors
    ///
    /// Returns [`LuceneError::TooManyClauses`] if the new number of clauses exceeds
    /// the maximum clause count.
    pub fn add_query<Q>(&mut self, query: Q, occur: Occur) -> Result<&mut Self>
    where
        Q: Into<Query>,
    {
        self.add_clause(BooleanClause::new(query.into(), occur))
    }

    /// Create a new [`BooleanQuery`] based on the parameters that have been set on this builder.
    pub fn build(self) -> BooleanQuery {
        BooleanQuery::new(self.minimum_number_should_match, self.clauses)
    }
}
#[cfg(test)]
mod tests {
    use crate::core::document::document::Document;
    use crate::core::document::field::FieldBase;
    use crate::core::document::field::Store::No;
    use crate::core::document::field_type::FieldType;
    use crate::core::document::long_point::LongPoint;
    use crate::core::document::string_field::StringField;
    use crate::core::index::composite_reader::get_context;
    use crate::core::index::directory_reader::directory_reader_util;
    use rand::Rng;
    use rand::prelude::SliceRandom;
    use std::collections::HashMap;

    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::index_writer_config::IndexWriterConfig;
    use crate::core::index::term::Term;
    use crate::core::search::boolean_clause::{BooleanClause, Occur};
    use crate::core::search::boolean_query::Builder;
    use crate::core::search::boost_query::BoostQuery;
    use crate::core::search::index_searcher::{IndexSearcher, get_max_clause_count};
    use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
    use crate::core::search::query::Query;
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::term_query::TermQuery;
    use crate::core::util::CoreHelper;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::test::search::query_utils::QueryUtils;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_directory_shared, new_index_writer_config, new_searcher_with_reader, new_text_field,
        random,
    };
    use crate::test::util::test_util::TestUtil;
    #[allow(dead_code)]
    struct TestBooleanQuery;

    #[test]
    fn test_equality() -> Result<()> {
        let mut bq1 = Builder::new();
        bq1.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "value1"))),
            Occur::Should,
        )?;
        bq1.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "value2"))),
            Occur::Should,
        )?;
        let mut nested1 = Builder::new();
        nested1.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "nestedvalue1"))),
            Occur::Should,
        )?;
        nested1.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "nestedvalue2"))),
            Occur::Should,
        )?;
        bq1.add_query(Query::Boolean(nested1.build()), Occur::Should)?;

        let mut bq2 = Builder::new();
        bq2.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "value1"))),
            Occur::Should,
        )?;
        bq2.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "value2"))),
            Occur::Should,
        )?;
        let mut nested2 = Builder::new();
        nested2.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "nestedvalue1"))),
            Occur::Should,
        )?;
        nested2.add_query(
            Query::Term(TermQuery::new(Term::from_text("field", "nestedvalue2"))),
            Occur::Should,
        )?;
        bq2.add_query(Query::Boolean(nested2.build()), Occur::Should)?;

        assert_eq!(bq1.build(), bq2.build());
        Ok(())
    }
    #[test]
    fn test_equality_does_not_depend_on_order() -> Result<()> {
        let mut random = random();

        let queries = [
            TermQuery::new(Term::from_text("foo", "bar")),
            TermQuery::new(Term::from_text("foo", "baz")),
        ];

        for _ in 0..10 {
            let num_clauses = random.random_range(0..20) as usize;

            let mut clauses: Vec<BooleanClause> = Vec::with_capacity(num_clauses);
            for _ in 0..num_clauses {
                let mut query = if random.random_bool(0.5) {
                    Query::Term(queries[0].clone())
                } else {
                    Query::Term(queries[1].clone())
                };

                if random.random_bool(0.5) {
                    let boost = random.random();
                    query = Query::Boost(BoostQuery::new(query, boost)?);
                }

                let occur = match random.random_range(0..4) {
                    0 => Occur::Must,
                    1 => Occur::Filter,
                    2 => Occur::Should,
                    _ => Occur::MustNot,
                };

                clauses.push(BooleanClause { query, occur });
            }

            let min_should_match = random.random_range(0..5);

            let mut bq1_builder = Builder::new();
            bq1_builder.set_minimum_number_should_match(min_should_match);
            for clause in &clauses {
                bq1_builder.add_clause(clause.clone())?;
            }
            let bq1 = bq1_builder.build();

            clauses.shuffle(&mut random);

            let mut bq2_builder = Builder::new();
            bq2_builder.set_minimum_number_should_match(min_should_match);
            for clause in &clauses {
                bq2_builder.add_clause(clause.clone())?;
            }
            let bq2 = bq2_builder.build();

            QueryUtils::check_equal(&bq1, &bq2)
        }

        Ok(())
    }
    #[test]
    fn test_equality_on_duplicate_should_clauses() -> Result<()> {
        let mut random = random();

        let min_should_match = random.random_range(0..2);

        let mut bq1_builder = Builder::new();
        bq1_builder.set_minimum_number_should_match(min_should_match);
        bq1_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
        let bq1 = bq1_builder.build();

        let mut bq2_builder = Builder::new();
        bq2_builder.set_minimum_number_should_match(bq1.get_minimum_number_should_match());
        bq2_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
        bq2_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
        let bq2 = bq2_builder.build();

        QueryUtils::check_unequal(&bq1, &bq2);
        Ok(())
    }
    #[test]
    fn test_equality_on_duplicate_filter_clauses() -> Result<()> {
        let mut random = random();

        let min_should_match = random.random_range(0..2);

        let mut bq1_builder = Builder::new();
        bq1_builder.set_minimum_number_should_match(min_should_match);
        bq1_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq1 = bq1_builder.build();

        let mut bq2_builder = Builder::new();
        bq2_builder.set_minimum_number_should_match(bq1.get_minimum_number_should_match());
        bq2_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        bq2_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq2 = bq2_builder.build();

        QueryUtils::check_equal(&bq1, &bq2);
        Ok(())
    }

    #[test]
    fn test_equality_on_duplicate_must_not_clauses() -> Result<()> {
        let mut random = random();

        let min_should_match = random.random_range(0..2);

        let mut bq1_builder = Builder::new();
        bq1_builder.set_minimum_number_should_match(min_should_match);
        bq1_builder.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?;
        bq1_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq1 = bq1_builder.build();

        let mut bq2_builder = Builder::new();
        bq2_builder.set_minimum_number_should_match(bq1.get_minimum_number_should_match());
        bq2_builder.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?;
        bq2_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        bq2_builder.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq2 = bq2_builder.build();

        QueryUtils::check_equal(&bq1, &bq2);
        Ok(())
    }

    #[test]
    fn test_hash_code_is_stable() -> Result<()> {
        let mut random = random();

        let t1 = Term::from_text("foo", TestUtil::random_simple_string(&mut random));
        let t2 = Term::from_text("foo", TestUtil::random_simple_string(&mut random));

        let mut bq_builder = Builder::new();
        bq_builder.add_query(TermQuery::new(t1), Occur::Should)?;
        bq_builder.add_query(TermQuery::new(t2), Occur::Should)?;
        let bq = bq_builder.build();

        let hash1 = CoreHelper::calculate_hash(&bq);
        assert_eq!(hash1, CoreHelper::calculate_hash(&bq));

        Ok(())
    }
    #[test]
    fn test_too_many_clauses() -> Result<()> {
        let mut bq = Builder::new();

        let max = get_max_clause_count();

        for i in 0..max {
            bq.add_query(
                TermQuery::new(Term::from_text("foo", format!("bar-{}", i))),
                Occur::Should,
            )?;
        }

        let res = bq.add_query(
            TermQuery::new(Term::from_text("foo", "bar-MAX")),
            Occur::Should,
        );

        assert!(matches!(res, Err(LuceneError::TooManyClauses(_))));
        Ok(())
    }

    #[test]
    fn test_null_or_sub_scorer() -> Result<()> {
        // TODO PhraseQuery未实现
        Ok(())
    }
    #[test]
    fn test_de_morgan() -> Result<()> {
        // TODO DeMorgan 相关逻辑尚未实现
        Ok(())
    }
    #[test]
    fn test_bs2disjunction_next_vs_advance() -> Result<()> {
        // TODO IMPORTANT 等BooleanQuery稳定后再来实现
        Ok(())
    }
    #[test]
    fn test_min_should_match_leniency() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        // TODO: 未实现 MockAnalyzer
        let iwc = new_index_writer_config(&mut random);
        let writer = IndexWriter::new(dir.clone(), iwc)?;
        let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
        let mut doc = Document::new();
        doc.add(new_text_field("field", "a b c d", No, &mut field_to_type)?);
        writer.add_document(doc)?;

        let reader = directory_reader_util::open_with_writer(&writer)?;
        let searcher = new_searcher_with_reader(reader)?;

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?;
        bq.add_query(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?;

        // No doc can match: only 2 SHOULD clauses, but min_should_match = 4
        bq.set_minimum_number_should_match(4);
        let query = bq.build();

        let top_docs = searcher.search(query, 1)?;
        assert_eq!(0, top_docs.total_hits.value());

        Ok(())
    }
    #[test]
    fn test_filter_clause_behaves_like_must() -> Result<()> {
        // TODO IMPORTANT FixedBitSetCollector未实现
        Ok(())
    }
    #[test]
    fn test_filter_clause_does_not_impact_score() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }

    #[test]
    fn test_conjunction_propagates_approximations() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }

    #[test]
    fn test_disjunction_propagates_approximations() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }

    #[test]
    fn test_boosted_scorer_propagates_approximations() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }

    #[test]
    fn test_exclusion_propagates_approximations() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }

    #[test]
    fn test_req_opt_propagates_approximations() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }
    #[test]
    fn test_query_matches_count() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }
    #[test]
    fn test_conjunction_matches_count() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let writer = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;

        let mut doc = Document::new();
        let mut long_point = LongPoint::new("long", [3i64])?;
        doc.add(long_point.clone());
        let mut string_field = StringField::with_string("string", "abc", No)?;
        doc.add(string_field.clone());
        writer.add_document(doc.clone())?;

        long_point.set_long_value(10)?;
        string_field.set_string_value("xyz")?;
        doc = Document::new();
        doc.add(string_field);
        doc.add(long_point);
        writer.add_document(doc)?;

        let reader = directory_reader_util::open_with_writer(&writer)?;
        let reader = get_context(reader)?;
        let searcher = IndexSearcher::new(reader)?;

        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Must,
            )?
            .add_query(LongPoint::new_exact_query("long", 3)?, Occur::Filter)?;
        let query = builder.build();

        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // Both queries match a single doc, BooleanWeight can't figure out the count of the conjunction
        assert_eq!(-1, weight.count(&searcher.get_leaf_contexts()?[0])?);

        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "missing")),
                Occur::Must,
            )?
            .add_query(LongPoint::new_exact_query("long", 3)?, Occur::Filter)?;
        let query = builder.build();

        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // One query has a count of 0, the conjunction has a count of 0 too
        assert_eq!(0, weight.count(&searcher.get_leaf_contexts()?[0])?);

        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Must,
            )?
            .add_query(LongPoint::new_exact_query("long", 5)?, Occur::Filter)?;
        let query = builder.build();

        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // One query has a count of 0, the conjunction has a count of 0 too
        assert_eq!(0, weight.count(&searcher.get_leaf_contexts()?[0])?);

        // FILTER matches all docs → conjunction count equals MUST count
        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Must,
            )?
            .add_query(LongPoint::new_range_query("long", 0, 10)?, Occur::Filter)?;
        let query = builder.build();

        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // One query matches all docs, the count of the conjunction is the count of the other query
        assert_eq!(1, weight.count(&searcher.get_leaf_contexts()?[0])?);

        let mut builder = Builder::new();
        builder
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(LongPoint::new_range_query("long", 1, 5)?, Occur::Filter)?;
        let query = builder.build();

        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // One query matches all docs, the count of the conjunction is the count of the other query
        assert_eq!(1, weight.count(&searcher.get_leaf_contexts()?[0])?);

        Ok(())
    }
    #[test]
    fn test_disjunction_matches_count() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let writer = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;

        let mut doc = Document::new();
        let mut long_point = LongPoint::new("long", [3i64])?;
        let mut long_point_3dim = LongPoint::new("long3dim", [3i64, 4i64, 5i64])?;
        doc.add(long_point.clone());
        doc.add(long_point_3dim.clone());

        let mut string_field = StringField::with_string("string", "abc", No)?;
        doc.add(string_field.clone());

        writer.add_document(doc.clone())?;

        long_point.set_long_value(10)?;
        long_point_3dim.set_long_values([10i64, 11i64, 12i64])?;
        string_field.set_string_value("xyz")?;

        doc = Document::new();
        doc.add(string_field);
        doc.add(long_point);
        doc.add(long_point_3dim);
        writer.add_document(doc)?;

        let reader = directory_reader_util::open_with_writer(&writer)?;
        let reader = get_context(reader)?;
        let searcher = IndexSearcher::new(reader)?;

        let leaf = &searcher.get_leaf_contexts()?[0];

        // Both queries match a single doc, BooleanWeight can't figure out the count of the disjunction
        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Should,
            )?
            .add_query(LongPoint::new_exact_query("long", 3)?, Occur::Should)?;
        let query = builder.build();
        // Both queries match a single doc, BooleanWeight can't figure out the count of the disjunction
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        assert_eq!(-1, weight.count(leaf)?);

        // One query has a count of 0, the disjunction count is the other count
        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "missing")),
                Occur::Should,
            )?
            .add_query(LongPoint::new_exact_query("long", 3)?, Occur::Should)?;
        let query = builder.build();
        // One query has a count of 0, the disjunction count is the other count
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        assert_eq!(1, weight.count(leaf)?);

        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Should,
            )?
            .add_query(LongPoint::new_exact_query("long", 5)?, Occur::Should)?;
        let query = builder.build();
        // One query has a count of 0, the disjunction count is the other count
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        assert_eq!(1, weight.count(leaf)?);

        // One query matches all docs, the count of the disjunction is the number of docs
        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Should,
            )?
            .add_query(LongPoint::new_range_query("long", 0, 10)?, Occur::Should)?;
        let query = builder.build();
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // One query matches all docs, the count of the disjunction is the number of docs

        assert_eq!(2, weight.count(leaf)?);

        let mut builder = Builder::new();
        builder
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Should)?
            .add_query(LongPoint::new_range_query("long", 1, 5)?, Occur::Should)?;
        let query = builder.build();
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // One query matches all docs, the count of the disjunction is the number of docs
        assert_eq!(2, weight.count(leaf)?);

        // Unknown count query on 3D long point range
        let lower = [4i64, 5i64, 6i64];
        let upper = [9i64, 10i64, 11i64];
        let unknown_count_query = LongPoint::new_range_query_n("long3dim", &lower, &upper)?;

        debug_assert_eq!(1, searcher.get_leaf_contexts()?.len());
        let w =
            searcher.create_weight(unknown_count_query.clone(), ScoreMode::Complete, 1.0, None)?;
        assert_eq!(-1, w.count(leaf)?);

        // count of the first MUST_NOT clause is unknown, but the second MUST_NOT clause matches all docs
        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "xyz")),
                Occur::Must,
            )?
            .add_query(unknown_count_query.clone(), Occur::MustNot)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::MustNot)?;
        let query = builder.build();
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // count of the first MUST_NOT clause is unknown, but the second MUST_NOT clause matches all
        // docs
        assert_eq!(0, weight.count(leaf)?);

        let mut builder = Builder::new();
        builder
            .add_query(
                TermQuery::new(Term::from_text("string", "xyz")),
                Occur::Must,
            )?
            .add_query(unknown_count_query.clone(), Occur::MustNot)?
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::MustNot,
            )?;
        let query = builder.build();
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // count of the first MUST_NOT clause is unknown, though the second MUST_NOT clause matche one
        // doc, we can't figure out the number of
        // docs
        assert_eq!(-1, weight.count(leaf)?);

        // test pure disjunction
        let mut builder = Builder::new();
        builder
            .add_query(unknown_count_query.clone(), Occur::Should)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Should)?;
        let query = builder.build();
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // count of the first SHOULD clause is unknown, but the second SHOULD clause matches all docs
        assert_eq!(2, weight.count(leaf)?);

        // count of the first SHOULD clause is unknown, though the second SHOULD clause matches one doc
        let mut builder = Builder::new();
        builder
            .add_query(unknown_count_query, Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("string", "abc")),
                Occur::Should,
            )?;
        let query = builder.build();
        let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0, None)?;
        // count of the first SHOULD clause is unknown, though the second SHOULD clause matche one doc,
        // we can't figure out the number of
        // docs
        assert_eq!(-1, weight.count(leaf)?);

        Ok(())
    }
    #[test]
    fn test_two_clause_term_disjunction_count_optimization() -> Result<()> {
        // TODO IndexSearch
        Ok(())
    }

    // TODO IMPORTANT 还有好几个未完成的测试
}

#[cfg(test)]
mod test {
    use crate::core::document::document::Document;
    use crate::core::document::field::Store::No;
    use crate::core::document::field_type::FieldType;
    use crate::core::index::composite_reader::get_context;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::index_writer_config::IndexWriterConfig;
    use crate::core::index::multi_reader::MultiReader;
    use crate::core::index::term::Term;
    use crate::core::search::boolean_clause::Occur;
    use crate::core::search::boolean_query::Builder;
    use crate::core::search::boost_query::BoostQuery;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::index_searcher::IndexSearcher;
    use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
    use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
    use crate::core::search::query::{Query, QueryBase};
    use crate::core::search::score_mode::ScoreMode;
    use crate::core::search::term_query::TermQuery;
    use crate::core::util::HasIdentity;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::test::index::random_index_writer::RandomIndexWriter;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        at_least, new_directory_shared, new_searcher_with_reader, new_text_field, random,
    };
    use rand::Rng;
    use std::collections::HashMap;

    #[allow(dead_code)]
    struct TestBooleanRewrites;
    #[test]
    fn test_one_clause_rewrite_optimization() -> Result<()> {
        let mut random = random();
        let field = "content";
        let value = "foo";

        let dir = new_directory_shared(&mut random)?;
        let writer = RandomIndexWriter::new(&mut random, dir.clone());
        writer.close()?;

        let reader = directory_reader_util::open(dir)?;
        let searcher = new_searcher_with_reader(reader)?;

        let expected: Query = TermQuery::new(Term::from_text(field, value)).into();

        let num_layers = at_least(&mut random, 3);
        let mut actual: Query = TermQuery::new(Term::from_text(field, value)).into();

        for _ in 0..num_layers {
            let mut bq = Builder::new();
            let occur = if random.random_bool(0.5) {
                Occur::Should
            } else {
                Occur::Must
            };
            bq.add_query(actual, occur)?;
            actual = bq.build().into();
        }

        let rewritten = searcher.rewrite(actual)?;
        assert_eq!(expected, rewritten);

        Ok(())
    }
    #[test]
    fn test_single_filter_clause() -> Result<()> {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;
        let writer = IndexWriter::new(dir.clone(), IndexWriterConfig::new())?;

        let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
        let mut doc = Document::new();
        doc.add(new_text_field("field", "a", No, &mut field_to_type)?);
        writer.add_document(doc)?;
        writer.commit()?;

        let reader = directory_reader_util::open_with_writer(&writer)?;
        let reader = get_context(reader)?;
        let searcher = IndexSearcher::new(reader)?;

        let mut query1 = Builder::new();
        query1.add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Filter)?;
        let rewritten1 = query1.build().rewrite(&searcher)?;
        match rewritten1 {
            Query::Boost(bq) => {
                assert_eq!(0.0, bq.get_boost());
            },
            _ => return Err(LuceneError::illegal_state("expected BoostQuery")),
        }
        // When there are two clauses, we cannot rewrite, but if one of them creates
        // a null scorer we will end up with a single filter scorer and will need to
        // make sure to set score=0
        let mut query2 = Builder::new();
        query2.add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Filter)?;
        query2.add_query(
            TermQuery::new(Term::from_text("missing_field", "b")),
            Occur::Should,
        )?;

        let rewritten2 = searcher.rewrite(query2.build())?;
        let weight = searcher.create_weight(rewritten2, ScoreMode::Complete, 1.0, None)?;
        let leaf = &searcher.get_leaf_contexts()?[0];
        let mut scorer = weight
            .scorer(leaf)?
            .ok_or_else(|| LuceneError::illegal_state("null scorer"))?;

        assert_eq!(0, scorer.iterator_mut().next_doc()?);
        assert_eq!(0.0, scorer.score()?);

        Ok(())
    }
    #[test]
    fn test_single_must_match_all() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq = bq.build();

        assert_eq!(
            Query::ConstantScore(ConstantScoreQuery::new(Box::new(
                TermQuery::new(Term::from_text("foo", "bar")).into()
            ))),
            searcher.rewrite(bq)?
        );

        let mut bq = Builder::new();
        bq.add_query(
            BoostQuery::new(Query::MatchAll(MatchAllDocsQuery::new()), 42.0)?,
            Occur::Must,
        )?
        .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq = bq.build();

        let v: Query = BoostQuery::new(
            Query::ConstantScore(ConstantScoreQuery::new(Box::new(
                TermQuery::new(Term::from_text("foo", "bar")).into(),
            ))),
            42.0,
        )?
        .into();
        assert_eq!(v, searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();

        assert_eq!(
            Query::MatchAll(MatchAllDocsQuery::new()),
            searcher.rewrite(bq)?
        );

        let mut bq = Builder::new();
        bq.add_query(
            BoostQuery::new(Query::MatchAll(MatchAllDocsQuery::new()), 42.0)?,
            Occur::Must,
        )?
        .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();
        let v: Query = BoostQuery::new(Query::MatchAll(MatchAllDocsQuery::new()), 42.0)?.into();
        assert_eq!(v, searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "bar")),
                Occur::MustNot,
            )?;
        let bq = bq.build();
        let v: Query = bq.clone().into();
        assert_eq!(v, searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();

        assert_eq!(
            Query::MatchAll(MatchAllDocsQuery::new()),
            searcher.rewrite(bq)?
        );

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let bq = bq.build();

        let mut expected = Builder::new();
        expected
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let expected = expected.build();
        let v: Query = ConstantScoreQuery::new(Box::new(expected.into())).into();
        assert_eq!(v, searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "baz")),
                Occur::MustNot,
            )?;
        let bq = bq.build();

        let mut expected = Builder::new();
        expected
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "baz")),
                Occur::MustNot,
            )?;
        let expected = expected.build();
        let v: Query = ConstantScoreQuery::new(Box::new(expected.into())).into();
        assert_eq!(v, searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
        let bq: Query = bq.build().into();

        assert_eq!(bq.clone(), searcher.rewrite(bq)?);

        Ok(())
    }
    #[test]
    fn test_single_must_match_all_with_should_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let bq = bq.build();

        let mut expected = Builder::new();
        expected
            .add_query(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "bar")).into(),
                )),
                Occur::Must,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let expected = expected.build();

        assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

        Ok(())
    }
    #[test]
    fn test_deduplicate_must_and_filter() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq = bq.build();

        assert_eq!(
            Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
            searcher.rewrite(bq)?
        );

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let bq = bq.build();

        let mut expected = Builder::new();
        expected
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let expected = expected.build();

        assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

        Ok(())
    }
    #[test]
    fn test_convert_should_and_filter_to_must() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
        let bq = bq.build();

        assert_eq!(
            Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
            searcher.rewrite(bq)?
        );

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "quz")), Occur::Should)?
            .set_minimum_number_should_match(2);
        let bq = bq.build();

        let mut expected = Builder::new();
        expected
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "quz")), Occur::Should)?
            .set_minimum_number_should_match(1);
        let expected = expected.build();

        assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

        Ok(())
    }

    #[test]
    fn test_duplicate_must_or_filter_with_must_not() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "bar")),
                Occur::MustNot,
            )?;
        let bq = bq.build();

        assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq)?);

        let mut bq2 = Builder::new();
        bq2.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "bar")),
                Occur::MustNot,
            )?;
        let bq2 = bq2.build();

        assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq2)?);

        Ok(())
    }

    #[test]
    fn test_match_all_must_not() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::MustNot)?;
        let bq = bq.build();

        assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq)?);

        let mut bq2 = Builder::new();
        bq2.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "bor")),
                Occur::MustNot,
            )?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::MustNot)?;
        let bq2 = bq2.build();

        assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq2)?);

        Ok(())
    }

    #[test]
    fn test_deeply_nested_boolean_rewrite_should_clauses() -> Result<()> {
        // TODO TestRewriteQuery未实现
        Ok(())
    }
    #[test]
    fn test_deeply_nested_boolean_rewrite() -> Result<()> {
        // TODO TestRewriteQuery未实现
        Ok(())
    }
    #[test]
    fn test_remove_match_all_filter() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();

        assert_eq!(
            Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
            searcher.rewrite(bq)?
        );

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();

        let mut expected = Builder::new();
        expected
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let expected = expected.build();

        assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();

        let expected: Query = BoostQuery::new(
            Query::ConstantScore(ConstantScoreQuery::new(Box::new(
                TermQuery::new(Term::from_text("foo", "bar")).into(),
            ))),
            0.0,
        )?
        .into();

        assert_eq!(expected, searcher.rewrite(bq)?);

        let mut bq = Builder::new();
        bq.add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?
            .add_query(Query::MatchAll(MatchAllDocsQuery::new()), Occur::Filter)?;
        let bq = bq.build();

        let expected: Query = BoostQuery::new(
            Query::ConstantScore(ConstantScoreQuery::new(Box::new(Query::MatchAll(
                MatchAllDocsQuery::new(),
            )))),
            0.0,
        )?
        .into();

        assert_eq!(expected, searcher.rewrite(bq)?);

        Ok(())
    }
    #[test]
    fn test_random() -> Result<()> {
        // TODO IMPORTANT
        Ok(())
    }
    #[test]
    fn test_deduplicate_should_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
        let query: Query = query.build().into();

        let expected: Query = BoostQuery::new(
            Box::new(TermQuery::new(Term::from_text("foo", "bar")).into()),
            2.0,
        )?
        .into();
        assert_eq!(expected, searcher.rewrite(query.clone())?);

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                BoostQuery::new(
                    Box::new(TermQuery::new(Term::from_text("foo", "bar")).into()),
                    2.0,
                )?,
                Occur::Should,
            )?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let query: Query = query.build().into();

        let mut expected = Builder::new();
        expected
            .add_query(
                BoostQuery::new(
                    Box::new(TermQuery::new(Term::from_text("foo", "bar")).into()),
                    3.0,
                )?,
                Occur::Should,
            )?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let expected: Query = expected.build().into();

        assert_eq!(expected, searcher.rewrite(query.clone())?);

        let mut query = Builder::new();
        query
            .set_minimum_number_should_match(2)
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let query: Query = query.build().into();

        let expected = query.clone();
        assert_eq!(expected, searcher.rewrite(query)?);

        Ok(())
    }

    #[test]
    fn test_deduplicate_must_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?;
        let query: Query = query.build().into();

        let expected: Query = BoostQuery::new(
            Box::new(TermQuery::new(Term::from_text("foo", "bar")).into()),
            2.0,
        )?
        .into();
        assert_eq!(expected, searcher.rewrite(query.clone())?);

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                BoostQuery::new(
                    Box::new(TermQuery::new(Term::from_text("foo", "bar")).into()),
                    2.0,
                )?,
                Occur::Must,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?;
        let query: Query = query.build().into();

        let mut expected = Builder::new();
        expected
            .add_query(
                BoostQuery::new(
                    Box::new(TermQuery::new(Term::from_text("foo", "bar")).into()),
                    3.0,
                )?,
                Occur::Must,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?;
        let expected: Query = expected.build().into();

        assert_eq!(expected, searcher.rewrite(query)?);

        Ok(())
    }
    #[test]
    fn test_flatten_inner_disjunctions() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner.clone(), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut query = Builder::new();
        query
            .set_minimum_number_should_match(0)
            .add_query(inner.clone(), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .set_minimum_number_should_match(0)
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut query = Builder::new();
        query
            .set_minimum_number_should_match(1)
            .add_query(inner.clone(), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .set_minimum_number_should_match(1)
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut query = Builder::new();
        query
            .set_minimum_number_should_match(2)
            .add_query(inner.clone(), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let query: Query = query.build().into();

        assert_eq!(
            Query::from(MatchNoDocsQuery::new()),
            searcher.rewrite(query)?
        );

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
            .set_minimum_number_should_match(2);
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner, Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let query: Query = query.build().into();
        let query_id = query.identity().clone();
        let v = searcher.rewrite(query)?;
        assert_eq!(query_id, v.identity().clone());

        Ok(())
    }
    #[test]
    fn test_flatten_inner_conjunctions() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?;
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner.clone(), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut query = Builder::new();
        query
            .set_minimum_number_should_match(0)
            .add_query(inner.clone(), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .set_minimum_number_should_match(0)
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut query = Builder::new();
        query.add_query(inner.clone(), Occur::Must)?.add_query(
            TermQuery::new(Term::from_text("foo", "baz")),
            Occur::MustNot,
        )?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "baz")),
                Occur::MustNot,
            )?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Filter,
            )?;
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner.clone(), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Filter,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Filter,
            )?;
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner.clone(), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Filter,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::MustNot,
            )?;
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner, Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::MustNot,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        Ok(())
    }
    #[test]
    fn test_flatten_disjunction_in_must_clause() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?;
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner, Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
            .set_minimum_number_should_match(1);
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "foo")), Occur::Should)?
            .set_minimum_number_should_match(2);
        let inner: Query = inner.build().into();

        let mut query = Builder::new();
        query
            .add_query(inner, Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let query: Query = query.build().into();

        let mut expected_rewritten = Builder::new();
        expected_rewritten
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "quux")),
                Occur::Should,
            )?
            .add_query(TermQuery::new(Term::from_text("foo", "foo")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
            .set_minimum_number_should_match(2);
        let expected_rewritten: Query = expected_rewritten.build().into();

        assert_eq!(expected_rewritten, searcher.rewrite(query)?);

        Ok(())
    }

    #[test]
    fn test_discard_should_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?;
        let inner = inner.build();

        let query1: Query = ConstantScoreQuery::new(Box::new(inner.into())).into();
        let rewritten1: Query = ConstantScoreQuery::new(Box::new(
            TermQuery::new(Term::from_text("field", "a")).into(),
        ))
        .into();
        assert_eq!(rewritten1, searcher.rewrite(query1)?);

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("field", "c")), Occur::Filter)?;
        let inner = inner.build();

        let query2: Query = ConstantScoreQuery::new(Box::new(inner.into())).into();

        let mut rewritten2_inner = Builder::new();
        rewritten2_inner
            .add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Filter)?
            .add_query(TermQuery::new(Term::from_text("field", "c")), Occur::Filter)?;
        let rewritten2_inner = rewritten2_inner.build();
        let rewritten2: Query = ConstantScoreQuery::new(Box::new(rewritten2_inner.into())).into();

        assert_eq!(rewritten2, searcher.rewrite(query2)?);

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?;
        let inner = inner.build();

        let query3: Query = ConstantScoreQuery::new(Box::new(inner.into())).into();
        let query3_id = query3.identity().clone();
        let v = searcher.rewrite(query3)?;
        assert_eq!(query3_id, v.identity().clone());

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?
            .add_query(
                TermQuery::new(Term::from_text("field", "b")),
                Occur::MustNot,
            )?;
        let inner = inner.build();

        let query4: Query = ConstantScoreQuery::new(Box::new(inner.into())).into();
        let query4_id = query4.identity().clone();
        let v = searcher.rewrite(query4)?;
        assert_eq!(query4_id, v.identity().clone());

        let mut inner = Builder::new();
        inner
            .set_minimum_number_should_match(1)
            .add_query(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("field", "c")), Occur::Filter)?;
        let inner = inner.build();

        let query5: Query = ConstantScoreQuery::new(Box::new(inner.into())).into();
        let query5_id = query5.identity().clone();
        let v = searcher.rewrite(query5)?;
        assert_eq!(query5_id, v.identity().clone());

        Ok(())
    }

    #[test]
    fn test_should_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(Query::MatchNoDoc(MatchNoDocsQuery::new()), Occur::Should)?;
        let query = query.build();

        assert_eq!(
            Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
            searcher.rewrite(query)?
        );

        Ok(())
    }
    #[test]
    fn test_must_not_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(Query::MatchNoDoc(MatchNoDocsQuery::new()), Occur::MustNot)?;
        let query = query.build();

        assert_eq!(
            Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
            searcher.rewrite(query)?
        );

        Ok(())
    }

    #[test]
    fn test_must_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(Query::MatchNoDoc(MatchNoDocsQuery::new()), Occur::Must)?;
        let query = query.build();

        assert_eq!(
            Query::from(MatchNoDocsQuery::new()),
            searcher.rewrite(query)?
        );

        Ok(())
    }

    #[test]
    fn test_filter_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(Query::MatchNoDoc(MatchNoDocsQuery::new()), Occur::Filter)?;
        let query = query.build();

        assert_eq!(
            Query::from(MatchNoDocsQuery::new()),
            searcher.rewrite(query)?
        );

        Ok(())
    }

    #[test]
    fn test_empty_boolean() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let query: Query = Builder::new().build().into();

        assert_eq!(
            Query::from(MatchNoDocsQuery::new()),
            searcher.rewrite(query)?
        );

        Ok(())
    }

    #[test]
    fn test_simplify_filter_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query1 = Builder::new();
        query1
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "baz")).into(),
                )),
                Occur::Filter,
            )?;
        let query1 = query1.build();

        let mut expected1 = Builder::new();
        expected1
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
        let expected1: Query = expected1.build().into();

        assert_eq!(expected1, searcher.rewrite(query1)?);

        let mut query2 = Builder::new();
        query2
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
            .add_query(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "bar")).into(),
                )),
                Occur::Filter,
            )?;
        let query2 = query2.build();

        let expected2: Query = BoostQuery::new(
            Box::new(Query::ConstantScore(ConstantScoreQuery::new(Box::new(
                TermQuery::new(Term::from_text("foo", "bar")).into(),
            )))),
            0.0,
        )?
        .into();

        assert_eq!(expected2, searcher.rewrite(query2)?);

        Ok(())
    }
    #[test]
    fn test_simplify_must_not_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut query = Builder::new();
        query
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "baz")).into(),
                )),
                Occur::MustNot,
            )?;
        let query = query.build();

        let mut expected = Builder::new();
        expected
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
            .add_query(
                TermQuery::new(Term::from_text("foo", "baz")),
                Occur::MustNot,
            )?;
        let expected: Query = expected.build().into();

        assert_eq!(expected, searcher.rewrite(query)?);

        Ok(())
    }

    #[test]
    fn test_simplify_non_scoring_should_clauses() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let mut inner = Builder::new();
        inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(
                ConstantScoreQuery::new(Box::new(
                    TermQuery::new(Term::from_text("foo", "baz")).into(),
                )),
                Occur::Should,
            )?;
        let inner = inner.build();

        let query: Query = ConstantScoreQuery::new(Box::new(inner.into())).into();

        let mut expected_inner = Builder::new();
        expected_inner
            .add_query(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
            .add_query(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let expected_inner = expected_inner.build();

        let expected: Query = ConstantScoreQuery::new(Box::new(expected_inner.into())).into();

        assert_eq!(expected, searcher.rewrite(query)?);

        Ok(())
    }

    #[test]
    fn test_should_clauses_less_than_or_equal_to_minimum_number_should_match() -> Result<()> {
        // TODO PhraseQuery 未实现
        Ok(())
    }
}
