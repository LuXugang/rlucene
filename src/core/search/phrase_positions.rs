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
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::term::Term;
use crate::core::search::similarities_impl::similarities::SimScorer;
use crate::core::search::sloppy_phrase_matcher::SloppyPhraseMatcher;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;

/// Position of a term in a document that takes into account the term offset
/// within the phrase.
pub struct PhrasePositions {
    /// Position in the document.
    pub(crate) position: i32,
    /// Remaining positions in this document.
    pub(crate) count: i32,
    /// Position in the phrase.
    pub(crate) offset: i32,
    /// Unique ordinal across all `PhrasePositions` instances.
    pub(crate) ord: usize,
    pub(crate) postings_idx: usize,
    /// Repetition group identifier.
    /// A value >= 0 indicates that this is a repeating `PhrasePositions`.
    pub(crate) rpt_group: i32,
    /// Index within the repetition group.
    pub(crate) rpt_ind: usize,
    /// Terms associated with this position, used for repetition initialization.
    pub(crate) terms: Vec<Term>,
}
impl PhrasePositions {
    pub(crate) fn new(
        postings: usize,
        offset: usize,
        ord: usize,
        terms: Vec<Term>,
    ) -> Result<Self> {
        Ok(Self {
            postings_idx: postings,
            offset: offset.try_convert()?,
            ord,
            terms,
            position: 0,
            count: 0,
            rpt_group: -1,
            rpt_ind: 0,
        })
    }

    pub(crate) fn first_position<IE, SS>(
        phrase_matcher: &mut SloppyPhraseMatcher<IE, SS>,
        pp_idx: usize,
    ) -> Result<()>
    where
        IE: ImpactsEnum,
        SS: SimScorer,
    {
        // read first position
        let freq = phrase_matcher.posting_mut(pp_idx).freq()?;
        let pp = &mut phrase_matcher.pq.compare.phrase_positions[pp_idx];
        pp.count = freq;
        Self::next_position(phrase_matcher, pp_idx)?;
        Ok(())
    }

    /// Go to next location of this term in the current document, and set
    /// `position` as `location - offset`, so that a matching exact phrase is
    /// easily identified when all `PhrasePositions` have exactly the same
    /// `position`.
    pub(crate) fn next_position<IE, SS>(
        phrase_matcher: &mut SloppyPhraseMatcher<IE, SS>,
        pp_idx: usize,
    ) -> Result<bool>
    where
        IE: ImpactsEnum,
        SS: SimScorer,
    {
        let count = phrase_matcher.pq.compare.phrase_positions[pp_idx].count;
        if count > 0 {
            let pos = phrase_matcher.posting_mut(pp_idx).next_position()?;
            let pp = &mut phrase_matcher.pq.compare.phrase_positions[pp_idx];
            pp.count -= 1;
            pp.position = pos - pp.offset;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
