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
use crate::core::search::disjunction_matches_iterator::DisjunctionMatchesIterator;
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_matches_iterator::DummyMatchesIterator;
use crate::core::search::matches::Matches;
use crate::core::util::error::lucene_error::Result;
use once_cell::sync::Lazy;

#[allow(dead_code)] // for quick search
pub struct MatchesUtils;

pub static MATCH_WITH_NO_TERMS: Lazy<MatchWithNoTerms> = Lazy::new(|| MatchWithNoTerms);
pub struct MatchWithNoTerms;
impl IntoIterator for MatchWithNoTerms {
    type Item = String;
    type IntoIter = std::iter::Empty<String>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::empty()
    }
}

impl Matches for MatchWithNoTerms {
    type MatchesIterator = DummyMatchesIterator;

    fn get_matches(&self, _field: &str) -> Result<Option<Self::MatchesIterator>> {
        Ok(None)
    }

    type Matches = DummyMatches;

    fn get_sub_matches(&mut self) -> Vec<Self::Matches> {
        Vec::new()
    }
}

pub struct CombinedMatch<M>
where
    M: Matches,
{
    sub: Vec<M>,
}
impl<M> CombinedMatch<M>
where
    M: Matches,
{
    pub fn new(sub: Vec<M>) -> Self {
        CombinedMatch { sub }
    }
}

impl<M> IntoIterator for CombinedMatch<M>
where
    M: Matches,
{
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

impl<M> Matches for CombinedMatch<M>
where
    M: Matches,
{
    type MatchesIterator = DisjunctionMatchesIterator<M::MatchesIterator>;
    type Matches = M;

    fn get_matches(&self, field: &str) -> Result<Option<Self::MatchesIterator>> {
        let mut sub_iterators = Vec::new();
        for m in &self.sub {
            if let Some(it) = m.get_matches(field)? {
                sub_iterators.push(it);
            }
        }
        if sub_iterators.is_empty() {
            Ok(None)
        } else {
            todo!()
            // Ok(Some(DisjunctionMatchesIterator::from_sub_iterators(sub_iterators)))
        }
    }

    fn get_sub_matches(&mut self) -> Vec<Self::Matches> {
        std::mem::take(&mut self.sub)
    }
}
