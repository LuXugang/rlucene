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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::scorer::{Scorer, TwoPhaseState};
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::error::lucene_error::Result;
/// A Weight that has a constant score equal to the boost of the wrapped query.
/// This is typically useful when building queries which do not produce
/// meaningful scores and are mostly useful for filtering.
#[derive(Clone, Default)]
pub struct ConstantScoreWeight {
    score: f32,
}
impl ConstantScoreWeight {
    pub fn new(score: f32) -> Self {
        Self { score }
    }
    /// Return the score produced by this Weight.
    pub fn score(&self) -> f32 {
        self.score
    }
    pub fn explain<S, T>(&self, scorer: Option<S>, doc: i32, query_str: T) -> Result<Explanation>
    where
        S: Scorer,
        T: Into<String>,
    {
        let exists = match scorer {
            None => false,
            Some(mut s) => {
                let has_two_phase = s.has_two_phase_iterator();
                if has_two_phase == TwoPhaseState::Yes {
                    let mut two_phase = s.two_phase_iterator_mut().unwrap();
                    two_phase.approximation()?.advance(doc)? == doc && two_phase.matches()?
                } else {
                    s.iterator_mut().advance(doc)? == doc
                }
            },
        };

        if exists {
            if (self.score - 1.0).abs() < f32::EPSILON {
                Ok(Explanation::match_(self.score, query_str.into(), vec![]))
            } else {
                Ok(Explanation::match_(
                    self.score,
                    format!("{}^{}", query_str.into(), self.score),
                    vec![],
                ))
            }
        } else {
            Ok(Explanation::no_match(
                format!("{} doesn't match id {}", query_str.into(), doc),
                vec![],
            ))
        }
    }
}
