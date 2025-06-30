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
use crate::util::error::lucene_error::Result;

/// Allows access to the score of a query.
pub trait Scorable {
    /// Returns the score of the current document matching the query.
    ///
    /// # Errors
    /// Returns an error if an I/O error occurs.
    fn score(&self) -> Result<f32>;

    /// Returns the smoothing score of the current document matching the query.
    ///
    /// This score is used when the query/term does not appear in the document,
    /// and behaves like an IDF (inverse document frequency). The smoothing
    /// score is particularly important when the scorer returns a product of
    /// probabilities, so that the document score does not go to zero when
    /// one probability is zero. This can return `0` or a smoothing score.
    ///
    /// # Note
    /// Smoothing scores are described in many papers, including:
    /// - Metzler, D. and Croft, W. B., "Combining the Language Model and
    ///   Inference Network Approaches to Retrieval," *Information Processing
    ///   and Management Special Issue on Bayesian Networks and Information
    ///   Retrieval*, 40(5), pp. 735-750.
    fn smoothing_score(&self, _doc_id: i32) -> Result<f32> {
        Ok(0.0)
    }

    /// Optional method: Tells the scorer that its iterator may safely ignore
    /// all documents whose score is lower than the given `min_score`. This
    /// is a no-op by default.
    ///
    /// # Note
    /// This method may only be called from collectors that use
    /// [`ScoreMode::TOP_SCORES`](crate::search::score_mode::ScoreMode::TopScores),
    /// and successive calls may only set increasing values of `min_score`.
    fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
        Ok(())
    }

    /// Returns child sub-scorers positioned on the current document.
    fn get_children<T: Scorable>(&self) -> Result<Vec<ChildScorable<T>>> {
        Ok(vec![])
    }
}

/// A child Scorer and its relationship to its parent.
///
/// The relationship can be any string that makes sense to the parent scorer.
///
/// # Fields
/// - `child`: The child `Scorable`. (This is typically a direct child and may
///   itself also have children.)
/// - `relationship`: An arbitrary string relating this scorer to the parent.
#[derive(Debug, Clone)]
pub struct ChildScorable<T>
where
    T: Scorable,
{
    pub child: T,
    pub relationship: String,
}

impl<T> ChildScorable<T>
where
    T: Scorable,
{
    pub fn new(child: T, relationship: String) -> Self {
        Self {
            child,
            relationship,
        }
    }
}
