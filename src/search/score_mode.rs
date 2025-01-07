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

pub enum ScoreMode {
    Complete {
        is_exhaustive: bool,
        needs_scores: bool,
    },

    CompleteNoScores {
        is_exhaustive: bool,
        needs_scores: bool,
    },

    TopScores {
        is_exhaustive: bool,
        needs_scores: bool,
    },

    TopDocs {
        is_exhaustive: bool,
        needs_scores: bool,
    },

    TopDocsWithScores {
        is_exhaustive: bool,
        needs_scores: bool,
    },
}

impl ScoreMode {
    pub fn needs_scores(&self) -> bool {
        match self {
            ScoreMode::Complete { needs_scores, .. }
            | ScoreMode::CompleteNoScores { needs_scores, .. }
            | ScoreMode::TopScores { needs_scores, .. }
            | ScoreMode::TopDocs { needs_scores, .. }
            | ScoreMode::TopDocsWithScores { needs_scores, .. } => *needs_scores,
        }
    }

    pub fn is_exhaustive(&self) -> bool {
        match self {
            ScoreMode::Complete { is_exhaustive, .. }
            | ScoreMode::CompleteNoScores { is_exhaustive, .. }
            | ScoreMode::TopScores { is_exhaustive, .. }
            | ScoreMode::TopDocs { is_exhaustive, .. }
            | ScoreMode::TopDocsWithScores { is_exhaustive, .. } => *is_exhaustive,
        }
    }
}
