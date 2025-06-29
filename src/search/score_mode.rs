/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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
