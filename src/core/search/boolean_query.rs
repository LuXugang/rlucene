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
use std::hash::{Hash, Hasher};

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
        let mut clause_sets = HashMap::new();
        for (idx, clause) in clauses.iter().enumerate() {
            clause_sets
                .entry(clause.occur)
                .or_insert_with(Vec::new)
                .push(idx);
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
}
impl Hash for BooleanQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.minimum_number_should_match.hash(state);
        for clause in &self.clause_sets {
            for x in clause.1 {
                self.clauses[*x].hash(state);
            }
            clause.hash(state);
        }
    }
}

impl PartialEq for BooleanQuery {
    fn eq(&self, other: &Self) -> bool {
        self.minimum_number_should_match == other.minimum_number_should_match
            && self.clauses == other.clauses
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
        _per_reader_term_state: Option<TermStates<LRTermState<IRCLeafReader<IRC>>>>,
    ) -> Result<QueryWeight<IRCLeafReader<IRC>>>
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
        let v = BooleanWeight {
            similarity,
            weighted_clauses,
            query: self,
            score_mode: *score_mode,
        };
        Ok(Box::new(v))
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
        if let Some(_filter_indices) = self.clause_sets.get(&Occur::Filter) {
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
        } else {
            return Err(LuceneError::illegal_state(
                "clause_sets should contains all occurs, even if empty",
            ));
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
