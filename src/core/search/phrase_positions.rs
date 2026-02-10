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
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::term::Term;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::Result;

/// Position of a term in a document that takes into account the term offset
/// within the phrase.
pub struct PhrasePositions {
    /// Position in the document.
    pub(crate) position: usize,
    /// Remaining positions in this document.
    pub(crate) count: i32,
    /// Position in the phrase.
    pub(crate) offset: usize,
    /// Unique ordinal across all `PhrasePositions` instances.
    pub(crate) ord: usize,
    /// Stream of documents and positions.
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
    pub fn new(postings: usize, offset: usize, ord: usize, terms: Vec<Term>) -> Self {
        Self {
            postings_idx: postings,
            offset,
            ord,
            terms,
            position: 0,
            count: 0,
            rpt_group: -1,
            rpt_ind: 0,
        }
    }

    pub fn first_position<PE>(&mut self, postings: &mut [PE]) -> Result<()>
    where
        PE: PostingsEnum,
    {
        // read first position
        self.count = postings[self.postings_idx].freq()?;
        self.next_position(postings)?;
        Ok(())
    }

    /// Go to next location of this term in the current document, and set
    /// `position` as `location - offset`, so that a matching exact phrase is
    /// easily identified when all `PhrasePositions` have exactly the same
    /// `position`.
    pub fn next_position<PE>(&mut self, postings: &mut [PE]) -> Result<bool>
    where
        PE: PostingsEnum,
    {
        if self.count > 0 {
            self.count -= 1;
            let pos = postings[self.postings_idx].next_position()?.try_convert()?;
            self.position = pos.saturating_sub(self.offset);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
