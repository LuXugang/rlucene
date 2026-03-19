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
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Allows access to the score of a query.
pub trait Scorable: FixedScore {
  /// Returns the score of the current document matching the query.
  ///
  /// # Errors
  /// Returns an error if an I/O error occurs.
  fn score(&mut self) -> Result<f32>;

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
  fn smoothing_score(&mut self, _doc_id: i32) -> Result<f32> {
    Ok(0.0)
  }

  /// Optional method: Tells the scorer that its iterator may safely ignore
  /// all documents whose score is lower than the given `min_score`. This
  /// is a no-op by default.
  ///
  /// # Note
  /// This method may only be called from collectors that use
  /// [`ScoreMode::TOP_SCORES`](crate::core::search::score_mode::ScoreMode::TopScores),
  /// and successive calls may only set increasing values of `min_score`.
  fn set_min_competitive_score(&mut self, _min_score: f32) -> Result<()> {
    Ok(())
  }

  /// Returns child sub-scorers positioned on the current document.
  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    Ok(vec![])
  }

  /// In Java Lucene, some methods define their parameters as [`Scorable`],
  /// But internally they use `instanceOf` to cast them to Scorer,
  /// And then call Scorer’s defaultCost() method.
  /// Therefore, when implementing in Rust, if a struct implements Scorer,
  /// it must also implement the Scorable trait’s cost method,
  /// and the implementation should delegate to Scorer’s default_cost() method for consistency.
  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}

pub trait FixedScore {
  fn set_score(&mut self, _score: f32) -> Result<()> {
    Err(LuceneError::need_implemented(""))
  }
}

impl<T> FixedScore for &mut T
where
  T: FixedScore + ?Sized,
{
  fn set_score(&mut self, score: f32) -> Result<()> {
    (**self).set_score(score)
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

impl<T> Scorable for &mut T
where
  T: Scorable + ?Sized,
{
  fn score(&mut self) -> Result<f32> {
    (**self).score()
  }

  fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
    (**self).smoothing_score(doc_id)
  }

  fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
    (**self).set_min_competitive_score(min_score)
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    (**self).get_children()
  }

  fn cost(&self) -> Result<i64> {
    (**self).cost()
  }
}
macro_rules! either_scorable {
    (
        $vis:vis $name:ident {
            $( $Variant:ident : $T:ident ),+ $(,)?
        }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Scorable for $name<$( $T ),+>
        where
            $( $T: Scorable ),+
        {
            fn score(&mut self) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.score(), )+ }
            }

            fn smoothing_score(&mut self, doc_id: i32) -> Result<f32> {
                match self { $( Self::$Variant(inner) => inner.smoothing_score(doc_id), )+ }
            }

            fn set_min_competitive_score(&mut self, min_score: f32) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.set_min_competitive_score(min_score), )+ }
            }

            fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
                match self {
                    $( Self::$Variant(inner) => inner.get_children(), )+
                }
            }

            fn cost(&self) -> Result<i64> {
                match self { $( Self::$Variant(inner) => inner.cost(), )+ }
            }
        }

        impl<$( $T ),+> FixedScore for $name<$( $T ),+>
        where
            $( $T: Scorable ),+
        {
            fn set_score(&mut self, score: f32) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.set_score(score), )+ }
            }
        }
    };
}
either_scorable!(
    pub ScorableEnum2 { A: A, B: B }
);
