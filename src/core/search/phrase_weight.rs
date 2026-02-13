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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::phrase_matcher::PhraseMatcher;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::similarities_impl::similarities::{SimScorer, SimilarityEnum};
use crate::core::util::error::lucene_error::Result;

pub struct PhraseWeight<SS>
where
    SS: SimScorer,
{
    score_mode: ScoreMode,
    stats: SS,
    similarity: SimilarityEnum,
    field: String,
}

pub trait PhraseWeightBase<IRC>
where
    IRC: IndexReaderContext,
{
    type SimScorer: SimScorer;
    fn get_stats(&mut self, searcher: &IndexSearcher<IRC>) -> Result<Option<Self::SimScorer>>;

    type PhraseMatcher: PhraseMatcher;
    fn get_phrase_matcher(
        &mut self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        scorer: Self::SimScorer,
        expose_offsets: bool,
    ) -> Result<Option<Self::PhraseMatcher>>;
}
