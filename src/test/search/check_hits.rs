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
use crate::core::search::explanation::Explanation;
use crate::core::search::query::Query;
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::error::Error;

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
        let value = expl.get_value().to_f32().unwrap();

        // assertEquals(score, value, 0d)
        if value != score {
            assert!(
                false,
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
                assert!(
                    false,
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

            for i in 0..details.len() {
                let d = &details[i];
                let dval = d.get_value().to_f32().unwrap();
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
                assert!(
                    false,
                    "{}: actual subDetails combined=={} != value={} Explanation: {}",
                    q, combined, value, expl
                );
            }
        }

        Ok(())
    }
}
pub static COMPUTED_FROM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^.*, computed as .* from:$").unwrap());
