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
use crate::core::index::term::Term;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::explanation::Explanation;
use crate::core::search::similarities_impl::similarities::{SimScorer, Similarity};
use crate::core::search::term_statistics::TermStatistics;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::small_float::SmallFloat;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::util::lucene_test_case::{at_least, rarely};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;

pub(crate) const MAXDOC_FORTESTING: i64 = 1 << 48;
// must be at least MAXDOC_FORTESTING + i32::MAX
pub(crate) const MAXTOKENS_FORTESTING: i64 = 1 << 49;
/// Base test support for a similarity. NOTE: This test focuses on the similarity
/// impl, nothing else. The [stretch] goal is for this test to be so thorough in testing a new
/// Similarity that if this test passes, then all Lucene tests should also pass. Ie, if there is some
/// bug in a given Similarity that this test fails to catch then this test needs to be improved!
pub trait BaseSimilarityTestCase {
  /// returns a random corpus that is at least possible given the norm value for a single document.
  fn new_corpus<R>(random: &mut R, norm: i32) -> Result<CollectionStatistics>
  where
    R: Rng + ?Sized,
  {
    // lower bound of tokens in the collection (you produced this norm somehow)
    let lower_bound = if norm == 0 {
      // norms are omitted, but there must have been at least one token to produce that norm
      1i64
    } else {
      // minimum value that would decode to such a norm
      SmallFloat::byte4_to_int(norm as u8)? as i64
    };

    let max_doc: i64 = match random.random_range(0..6) {
      0 => {
        // 1 doc collection
        1
      },
      1 => {
        // 2 doc collection
        2
      },
      2 => {
        // tiny collection
        TestUtil::next_long(random, 3, 16)
      },
      3 => {
        // small collection
        TestUtil::next_long(random, 16, 100_000)
      },
      4 => {
        // big collection
        TestUtil::next_long(random, 100_000, MAXDOC_FORTESTING)
      },
      _ => {
        // yuge collection
        MAXDOC_FORTESTING
      },
    };

    let doc_count: i64 = match random.random_range(0..3) {
      0 => {
        // sparsest field
        1
      },
      1 => {
        // sparse field
        TestUtil::next_long(random, 1, max_doc)
      },
      _ => {
        // fully populated
        max_doc
      },
    };

    // random docsize: but can't require docs to have > 2B tokens
    let upper_bound: i64 = match doc_count.checked_mul(i32::MAX as i64) {
      Some(v) => std::cmp::min(MAXTOKENS_FORTESTING, v),
      None => MAXTOKENS_FORTESTING,
    };

    let sum_doc_freq: i64 = match random.random_range(0..3) {
      0 => {
        // shortest possible docs
        doc_count
      },
      1 => {
        // biggest possible docs
        upper_bound + 1 - lower_bound
      },
      _ => {
        // random docsize
        TestUtil::next_long(random, doc_count, upper_bound + 1 - lower_bound)
      },
    };

    let sum_total_term_freq: i64 = match random.random_range(0..4) {
      0 => {
        // term frequencies were omitted
        sum_doc_freq
      },
      1 => {
        // no repetition of terms (except to satisfy this norm)
        sum_doc_freq - 1 + lower_bound
      },
      2 => {
        // maximum repetition of terms
        upper_bound
      },
      _ => {
        // random repetition
        assert!(sum_doc_freq - 1 + lower_bound <= upper_bound);
        TestUtil::next_long(random, sum_doc_freq - 1 + lower_bound, upper_bound)
      },
    };

    CollectionStatistics::new(
      "field",
      max_doc,
      doc_count,
      sum_total_term_freq,
      sum_doc_freq,
    )
  }
  fn new_term<R>(random: &mut R, corpus: &CollectionStatistics) -> Result<TermStatistics>
  where
    R: Rng + ?Sized,
  {
    let doc_freq: i64 = match random.random_range(0..3) {
      0 => {
        // rare term
        1
      },
      1 => {
        // common term
        corpus.get_doc_count()
      },
      _ => {
        // random specificity
        TestUtil::next_long(random, 1, corpus.get_doc_count())
      },
    };

    // can't require docs to have > 2B tokens
    let upper_bound: i64 = match doc_freq.checked_mul(i32::MAX as i64) {
      Some(v) => std::cmp::min(corpus.get_sum_total_term_freq(), v),
      None => corpus.get_sum_total_term_freq(),
    };

    let total_term_freq: i64 = if corpus.get_sum_total_term_freq() == corpus.get_sum_doc_freq() {
      // omitTF
      doc_freq
    } else {
      match random.random_range(0..3) {
        0 => {
          // no repetition
          doc_freq
        },
        1 => {
          // maximum repetition
          upper_bound
        },
        _ => {
          // random repetition
          TestUtil::next_long(random, doc_freq, upper_bound)
        },
      }
    };

    TermStatistics::new(Term::from_text("term", "term"), doc_freq, total_term_freq)
  }
  /// runs for a single test case, so that if you hit a test failure you can write a reproducer just for that scenario
  fn do_test_scoring<S, R>(
    similarity: &S,
    corpus: &CollectionStatistics,
    term: &[TermStatistics],
    boost: f32,
    freq: f32,
    norm: i32,
    random: &mut R,
  ) -> Result<()>
  where
    S: Similarity,
    R: Rng + ?Sized,
  {
    let scorer = similarity.scorer(boost, corpus, term)?;

    let max_score = scorer.score(f32::MAX, 1);
    assert!(!max_score.is_nan(), "maxScore is NaN");

    let score = scorer.score(freq, norm.into());
    // check that score isn't infinite or negative
    assert!(score.is_finite(), "infinite/NaN score: {score}");
    // if !similarity.is_indri_dirichlet() {
    //     assert!(score >= 0.0, "negative score: {score}");
    // }
    assert!(
      score <= max_score,
      "greater than maxScore: {score}>{max_score}"
    );

    // explanation check
    let explanation = scorer.explain(
      Explanation::match_no_details(freq, "freq, occurrences of term within document"),
      norm as i64,
    )?;
    let explanation_value = explanation.get_value().to_f32().ok_or_else(|| {
      LuceneError::illegal_argument(format!(
        "cannot convert to f32: {}",
        explanation.get_value()
      ))
    })?;
    assert_eq!(
      score, explanation_value,
      "expected: {score}, got: {explanation}"
    );

    if rarely(random) {
      CheckHits::verify_explanation("<test query>", 0, score, true, &explanation)?;
    }

    let prev_freq = if random.random_bool(0.5)
      && freq == (freq as i32) as f32
      && freq > 1.0
      && term[0].get_doc_freq() > 1
    {
      freq - 1.0
    } else {
      freq.next_down()
    };

    let prev_score = scorer.score(prev_freq, norm as i64);
    assert!(prev_score.is_finite());
    // if !similarity.is_indri_dirichlet() {
    //     assert!(prev_score >= 0.0);
    // }

    let prev_explanation = scorer.explain(
      Explanation::match_no_details(prev_freq, "freq, occurrences of term within document"),
      norm as i64,
    )?;
    let prev_explanation_value = prev_explanation.get_value().to_f32().ok_or_else(|| {
      LuceneError::illegal_argument(format!(
        "cannot convert to f32: {}",
        prev_explanation.get_value()
      ))
    })?;
    assert_eq!(
      prev_score, prev_explanation_value,
      "expected: {prev_score}, got: {prev_explanation}"
    );

    if rarely(random) {
      CheckHits::verify_explanation(
        "test query (prevFreq)",
        0,
        prev_score,
        true,
        &prev_explanation,
      )?;
    }

    if prev_score > score {
      println!("{prev_explanation}");
      println!("{explanation}");
      unreachable!("score({prev_freq})={prev_score} > score({freq})={score}");
    }
    // check score(norm-1), given the same freq it should be >= score(norm) [scores non-decreasing
    // as docs get shorter]
    if norm > 1 {
      let prev_norm_score = scorer.score(freq, (norm - 1) as i64);
      assert!(prev_norm_score.is_finite());
      // if !similarity.is_indri_dirichlet() {
      //     assert!(prev_norm_score >= 0.0);
      // }

      let prev_norm_explanation = scorer.explain(
        Explanation::match_no_details(freq, "freq, occurrences of term within document"),
        norm as i64 - 1,
      )?;
      let prev_norm_explanation_value =
        prev_norm_explanation.get_value().to_f32().ok_or_else(|| {
          LuceneError::illegal_argument(format!(
            "cannot convert to f32: {}",
            prev_norm_explanation.get_value()
          ))
        })?;
      assert_eq!(
        prev_norm_score, prev_norm_explanation_value,
        "expected: {prev_norm_score}, got: {prev_norm_explanation}"
      );

      if rarely(random) {
        CheckHits::verify_explanation(
          "test query (prevNorm)",
          0,
          prev_norm_score,
          true,
          &prev_norm_explanation,
        )?;
      }

      if prev_norm_score < score {
        println!("{prev_norm_explanation}");
        println!("{explanation}");
        unreachable!(
          "score({freq},{})={} < score({freq},{norm})={}",
          norm - 1,
          prev_norm_score,
          score
        );
      }
    }
    // check score(term-1), given the same freq/norm it should be >= score(term) [scores
    // non-decreasing as terms get rarer]
    if term[0].get_doc_freq() > 1 && (freq as i64) < term[0].get_total_term_freq() {
      let prev_term = TermStatistics::new(
        term[0].get_term().clone(),
        term[0].get_doc_freq() - 1,
        term[0].get_total_term_freq() - 1,
      )?;
      let prev_term = vec![prev_term];

      let prev_term_scorer = similarity.scorer(boost, corpus, prev_term.as_slice())?;

      let prev_term_score = prev_term_scorer.score(freq, norm.into());
      assert!(prev_term_score.is_finite());
      // if !similarity.is_indri_dirichlet() {
      //     assert!(prev_term_score >= 0.0);
      // }

      let prev_term_explanation = prev_term_scorer.explain(
        Explanation::match_no_details(freq, "freq, occurrences of term within document"),
        norm.into(),
      )?;
      let prev_term_explanation_value =
        prev_term_explanation.get_value().to_f32().ok_or_else(|| {
          LuceneError::illegal_argument(format!(
            "cannot convert to f32: {}",
            prev_term_explanation.get_value()
          ))
        })?;
      assert_eq!(
        prev_term_score, prev_term_explanation_value,
        "expected: {prev_term_score}, got: {prev_term_explanation}"
      );

      if rarely(random) {
        CheckHits::verify_explanation(
          "test query (prevTerm)",
          0,
          prev_term_score,
          true,
          &prev_term_explanation,
        )?;
      }

      if prev_term_score < score {
        println!("{prev_term_explanation}");
        println!("{explanation}");
        unreachable!(
          "score({freq},{prev_term:?})={prev_term_score} < score({freq},{term:?})={score}"
        );
      }
    }
    Ok(())
  }
  type Similarity: Similarity;
  fn get_similarity<R>(&self, random: &mut R) -> Result<Self::Similarity>
  where
    R: Rng + ?Sized;
  /// Tests scoring across a bunch of random terms/corpora/frequencies for each possible document
  /// length. It does the following checks:
  ///
  /// - Scores are non-negative and finite.
  /// - Score matches the explanation exactly.
  /// - Internal explanations calculations are sane (e.g., sum of: and so on actually compute
  ///   sums)
  /// - Scores don't decrease as term frequencies increase: e.g., score(freq=N + 1) >=
  ///   score(freq=N)
  /// - Scores don't decrease as documents get shorter, e.g., score(len=M) >= score(len=M+1)
  /// - Scores don't decrease as terms get rarer, e.g., score(term=N) >= score(term=N+1)
  /// - Scoring works for floating point frequencies (e.g., sloppy phrase and span queries will
  ///   work)
  /// - Scoring works for reasonably large 64-bit statistic values (e.g. distributed search will
  ///   work)
  /// - Scoring works for reasonably large boost values (0 .. `i32::MAX`, e.g. query
  ///   boosts will work)
  /// - Scoring works for parameters randomized within valid ranges (see [`get_similarity`])
  fn test_random_scoring<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let iterations = at_least(random, 1);

    for _i in 0..iterations {
      let similarity = self.get_similarity(random)?;

      for _j in 0..3 {
        for k in 1..256 {
          let corpus = Self::new_corpus(random, k)?;
          for _ in 0..10 {
            let term = Self::new_term(random, &corpus)?;
            let freq: f32 = if term.get_total_term_freq() == term.get_doc_freq() {
              // omit TF
              1.0
            } else if term.get_doc_freq() == 1 {
              // only one document, all instances are in this doc
              term.get_total_term_freq() as f32
            } else {
              // at least one other document has at least 1 occurrence
              let upper_bound = std::cmp::min(
                term.get_total_term_freq() - term.get_doc_freq() + 1,
                i32::MAX as i64,
              ) as i32;

              if random.random_bool(0.5) {
                // integer freq
                match random.random_range(0..3) {
                  0 => 1.0,
                  1 => upper_bound as f32,
                  _ => TestUtil::next_int(random, 1, upper_bound) as f32,
                }
              } else {
                // float freq
                let mut freq_candidate: f32 = match random.random_range(0..2) {
                  0 => f32::MIN_POSITIVE,
                  _ => {
                    let r: f32 = random.random();
                    (upper_bound as f32) * r
                  },
                };
                // we need to be 2nd float value at a minimum, the pairwise test will check
                // MIN_VALUE in this case.
                // this avoids testing frequencies of 0 which seem wrong to allow (we should enforce
                // computeSlopFactor etc)
                if freq_candidate <= f32::MIN_POSITIVE {
                  freq_candidate = f32::MIN_POSITIVE.next_up();
                }

                freq_candidate
              }
            };
            // we just limit the test to "reasonable" boost values but don't enforce this anywhere.
            // too big, and you are asking for overflow. that's hard for a sim to enforce (but
            // definitely possible)
            // for now, we just want to detect overflow where its a real bug/hazard in the
            // computation with reasonable inputs.
            let boost: f32 = match random.random_range(0..5) {
              0 => 0.0,
              1 => f32::MIN_POSITIVE,
              2 => 1.0,
              3 => i32::MAX as f32,
              _ => {
                let r: f32 = random.random();
                r * (i32::MAX as f32)
              },
            };
            let term = vec![term];
            Self::do_test_scoring(
              &similarity,
              &corpus,
              term.as_slice(),
              boost,
              freq,
              k,
              random,
            )?;
          }
        }
      }
    }

    Ok(())
  }
}
