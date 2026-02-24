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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryWeightSsScorer};
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;

pub struct CheckHits;
impl CheckHits {
    pub fn check_equal<S>(query: &Query, hits1: &[S], hits2: &[S]) -> Result<()>
    where
        S: ScoreDocLike,
    {
        const SCORE_TOLERANCE: f32 = 1.0e-6;

        if hits1.len() != hits2.len() {
            return Err(LuceneError::illegal_argument(format!(
                "Unequal lengths: hits1={}, hits2={}",
                hits1.len(),
                hits2.len()
            )));
        }

        for (i, (h1, h2)) in hits1.iter().zip(hits2.iter()).enumerate() {
            if h1.doc() != h2.doc() {
                return Err(LuceneError::illegal_argument(format!(
                    "Hit {i} docnumbers don't match\nhits1={:?}\nhits2={:?}\nfor query: {:?}",
                    hits1, hits2, query
                )));
            }

            if (h1.doc() != h2.doc()) || (h1.score() - h2.score()).abs() > SCORE_TOLERANCE {
                return Err(LuceneError::illegal_argument(format!(
                    "Hit {i}, doc nrs {} and {}\nunequal: {}\nand: {}\nfor query: {:?}",
                    h1.doc(),
                    h2.doc(),
                    h1.score(),
                    h2.score(),
                    query
                )));
            }
        }

        Ok(())
    }
    pub fn verify_explanation(
        q: &str,
        doc: i32,
        score: f32,
        deep: bool,
        expl: &Explanation,
    ) -> Result<()> {
        let value = expl.get_value().to_f32().ok_or_else(|| {
            LuceneError::illegal_argument(format!("cannot convert to f32: {}", expl.get_value()))
        })?;
        if value != score {
            unreachable!(
                "{}: score(doc={})={} != explanationScore={} Explanation: {}",
                q, doc, score, value, expl
            );
        }

        if !deep {
            return Ok(());
        }

        let details = expl.get_details();
        let descr = expl.get_description().to_lowercase();

        if descr.ends_with("computed from:") {
            return Ok(());
        }

        if descr.starts_with("score based on ") && descr.contains("child docs in range") {
            assert!(!details.is_empty(), "Child doc explanations are missing");
        }

        if !details.is_empty() && expl.is_match() {
            if details.len() == 1 && !COMPUTED_FROM_PATTERN.is_match(&descr) {
                let allow_compute_freq = !expl.get_description().ends_with("with freq of:")
                    && (score >= 0.0 || !expl.get_description().ends_with("times others of:"));

                if allow_compute_freq {
                    Self::verify_explanation(q, doc, score, deep, &details[0])?;
                }
                return Ok(());
            }

            let product_of = descr.ends_with("product of:");
            let sum_of = descr.ends_with("sum of:");
            let max_of = descr.ends_with("max of:");
            let computed_of =
                descr.contains("computed as") && COMPUTED_FROM_PATTERN.is_match(&descr);

            let mut max_times_others = false;
            let mut x: f32 = 0.0;

            if !(product_of || sum_of || max_of || computed_of) {
                let pat = "max plus ";
                if let Some(k1) = descr.find(pat) {
                    let k1 = k1 + pat.len();
                    if let Some(k2) = descr[k1..].find(' ') {
                        let k2 = k1 + k2;
                        let slice = descr[k1..k2].trim();
                        if let Ok(val) = slice.parse::<f32>() {
                            x = val;
                            let remain = descr[k2..].trim();
                            if remain == "times others of:" {
                                max_times_others = true;
                            }
                        }
                    }
                }
            }

            if !(product_of || sum_of || max_of || computed_of || max_times_others) {
                unreachable!(
                    "{}: multi valued explanation description=\"{}\" must be 'max plus x times others', \
                 'computed as x from:' or end with 'product of', 'sum of:', 'max of:' - {}",
                    q, descr, expl
                );
            }

            // sum/product/max computing
            let mut sum = 0f64;
            let mut product = 1f32;
            let mut max = f32::NEG_INFINITY;
            let mut max_error = 0f64;

            for d in details.iter() {
                let dval = d.get_value().to_f32().ok_or_else(|| {
                    LuceneError::illegal_argument(format!(
                        "cannot convert to f32: {}",
                        d.get_value()
                    ))
                })?;
                Self::verify_explanation(q, doc, dval, deep, d)?;

                product *= dval;
                sum += dval as f64;
                if dval > max {
                    max = dval;
                }

                if sum_of {
                    // Java leniency
                    max_error += (dval as f64).to_bits() as f64 * f64::EPSILON * 2.0;
                }
            }

            let combined: f32 = if product_of {
                product
            } else if sum_of {
                sum as f32
            } else if max_of {
                max
            } else if max_times_others {
                let s = sum as f32;
                max + x * (s - max)
            } else {
                // computedOf
                value
            };

            // assertEquals(combined, value, maxError)
            let diff = (combined as f64 - value as f64).abs();
            if diff > max_error {
                unreachable!(
                    "{}: actual subDetails combined=={} != value={} Explanation: {}",
                    q, combined, value, expl
                );
            }
        }

        Ok(())
    }
    pub(crate) fn check_top_scores<IRC, R>(
        random: &mut R,
        query: &Query,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<()>
    where
        IRC: IndexReaderContext,
        R: Rng + ?Sized,
        IRCLeafReader<IRC>: 'static,
    {
        // Check it computed the top hits correctly
        Self::do_check_top_scores(query, searcher, 1)?;
        Self::do_check_top_scores(query, searcher, 10)?;

        // Now check that the exposed max scores and block boundaries are valid
        Self::do_check_max_scores(random, query.clone(), searcher)?;

        Ok(())
    }

    fn do_check_top_scores<IRC>(
        query: &Query,
        searcher: &IndexSearcher<IRC>,
        num_hits: usize,
    ) -> Result<()>
    where
        IRC: IndexReaderContext,
        IRCLeafReader<IRC>: 'static,
    {
        let complete = TopScoreDocCollectorManager::with_after(num_hits, None, i32::MAX as usize)?;
        let top_scores = TopScoreDocCollectorManager::with_after(num_hits, None, 1)?;

        let complete_top_docs = searcher.search_with_collector_manager(query.clone(), &complete)?;
        let top_scores_top_docs =
            searcher.search_with_collector_manager(query.clone(), &top_scores)?;
        Self::check_equal(
            query,
            &complete_top_docs.score_docs,
            &top_scores_top_docs.score_docs,
        )?;

        Ok(())
    }

    fn do_check_max_scores<IRC, R>(
        random: &mut R,
        mut query: Query,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<()>
    where
        IRC: IndexReaderContext,
        R: Rng + ?Sized,
        IRCLeafReader<IRC>: 'static,
    {
        query = searcher.rewrite(query)?;

        let w1 = searcher.create_weight(query.clone(), ScoreMode::Complete, 1.0, None)?;
        let w2 = searcher.create_weight(query, ScoreMode::TopScores, 1.0, None)?;

        // Check boundaries and max scores when iterating all matches
        for ctx in searcher.get_leaf_contexts()? {
            let mut s1 = w1.scorer(ctx)?;
            let mut ss2 = w2.scorer_supplier(ctx)?;
            let mut s2 = if let Some(mut ss2) = ss2.take() {
                ss2.set_top_level_scoring_clause()?;
                Some(ss2.get(i64::MAX, ctx)?)
            } else {
                None
            };

            if s1.is_none() {
                if let Some(s2) = s2.as_mut() {
                    assert_eq!(NO_MORE_DOCS, s2.iterator_mut().next_doc()?);
                }
                continue;
            }
            if s2.is_none() {
                let s1 = s1.as_mut().unwrap();
                assert_eq!(NO_MORE_DOCS, s1.iterator_mut().next_doc()?);
                continue;
            }

            let mut s1 = s1.unwrap();
            let mut s2 = s2.unwrap();

            let mut upto: i32 = -1;
            let mut max_score: f32 = 0.0;
            let mut min_score: f32 = 0.0;

            let mut doc2 = Self::next_doc(&mut s2)?;
            loop {
                let mut doc1 = Self::next_doc(&mut s1)?;
                while doc1 < doc2 {
                    let matches1 = Self::matches(&mut s1)?;
                    if matches1 {
                        assert!(s1.score()? < min_score);
                    }
                    doc1 = Self::next_doc(&mut s1)?;
                }

                assert_eq!(doc1, doc2);
                if doc2 == NO_MORE_DOCS {
                    break;
                }

                if doc2 > upto {
                    upto = s2.advance_shallow(doc2)?;
                    assert!(upto >= doc2);
                    max_score = s2.get_max_score(upto)?;
                }

                let matches2 = Self::matches(&mut s2)?;
                if matches2 {
                    let matches1 = Self::matches(&mut s1)?;
                    assert!(matches1);

                    let score = s2.score()?;
                    assert_eq!(s1.score()?, score);
                    assert!(score <= max_score);

                    if score >= min_score && random.random_range(0..10) == 0 {
                        min_score = score;
                        s2.set_min_competitive_score(min_score)?;
                    }
                }

                doc2 = Self::next_doc(&mut s2)?;
            }
        }

        // Now check advancing
        for ctx in searcher.get_leaf_contexts()? {
            let mut s1 = w1.scorer(ctx)?;
            let mut ss2 = w2.scorer_supplier(ctx)?;
            let mut s2 = if let Some(mut ss2) = ss2.take() {
                ss2.set_top_level_scoring_clause()?;
                Some(ss2.get(i64::MAX, ctx)?)
            } else {
                None
            };

            if s1.is_none() {
                if let Some(s2) = s2.as_mut() {
                    assert_eq!(NO_MORE_DOCS, s2.iterator_mut().next_doc()?);
                }
                continue;
            }
            if s2.is_none() {
                let s1 = s1.as_mut().unwrap();
                assert_eq!(NO_MORE_DOCS, s1.iterator_mut().next_doc()?);
                continue;
            }

            let mut s1 = s1.unwrap();
            let mut s2 = s2.unwrap();

            let mut upto: i32 = -1;
            let mut min_score: f32 = 0.0;
            let mut max_score: f32 = 0.0;

            loop {
                let doc_id = s2.doc_id()?;
                let (advance, target) = if random.random_bool(0.5) {
                    (false, doc_id + 1)
                } else {
                    let delta =
                        std::cmp::min(1 + random.random_range(0..512), NO_MORE_DOCS - doc_id);
                    (true, s2.doc_id()? + delta)
                };

                if target > upto && random.random_bool(0.5) {
                    let delta = std::cmp::min(random.random_range(0..512), NO_MORE_DOCS - target);
                    upto = target + delta;
                    let m = s2.advance_shallow(target)?;
                    assert!(m >= target);
                    max_score = s2.get_max_score(upto)?;
                }

                let doc2 = if advance {
                    Self::advance(&mut s2, target)?
                } else {
                    Self::next_doc(&mut s2)?
                };

                let mut doc1 = Self::advance(&mut s1, target)?;
                while doc1 < doc2 {
                    let matches1 = Self::matches(&mut s1)?;
                    if matches1 {
                        assert!(s1.score()? < min_score);
                    }
                    doc1 = Self::next_doc(&mut s1)?;
                }
                assert_eq!(doc1, doc2);

                if doc2 == NO_MORE_DOCS {
                    break;
                }

                let matches2 = Self::matches(&mut s2)?;
                if matches2 {
                    let matches1 = Self::matches(&mut s1)?;
                    assert!(matches1);

                    let score = s2.score()?;
                    assert_eq!(s1.score()?, score);

                    if doc2 > upto {
                        upto = s2.advance_shallow(doc2)?;
                        assert!(upto >= doc2);
                        max_score = s2.get_max_score(upto)?;
                    }

                    assert!(score <= max_score);

                    if score >= min_score && random.random_range(0..10) == 0 {
                        min_score = score;
                        s2.set_min_competitive_score(min_score)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn advance(s: &mut QueryWeightSsScorer, target: i32) -> Result<i32> {
        if let Some(tp) = s.two_phase_iterator_mut().as_mut() {
            let mut v = tp.approximation_mut();
            v.advance(target)
        } else {
            let mut v = s.iterator_mut();
            v.advance(target)
        }
    }

    fn next_doc(s: &mut QueryWeightSsScorer) -> Result<i32> {
        if let Some(tp) = s.two_phase_iterator_mut().as_mut() {
            let mut v = tp.approximation_mut();
            v.next_doc()
        } else {
            let mut v = s.iterator_mut();
            v.next_doc()
        }
    }
    fn matches(s: &mut QueryWeightSsScorer) -> Result<bool> {
        if let Some(tp) = s.two_phase_iterator_mut().as_mut() {
            tp.matches()
        } else {
            Ok(true)
        }
    }
}
pub static COMPUTED_FROM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^.*, computed as .* from:$").unwrap());
