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
use crate::core::search::matches_iterator::{
    Either2MatchesIterator, Either3MatchesIterator, Either4MatchesIterator, Either5MatchesIterator,
    MatchesIterator,
};
use crate::core::util::error::lucene_error::Result;

/// Reports the positions and optionally offsets of all matching terms
/// in a query for a single document.
///
/// To obtain a [`MatchesIterator`] for a particular field, call
/// [`Matches::get_matches`]. Note that you can call this method multiple
/// times to retrieve new iterators, but it is not thread-safe.
///
/// @lucene.experimental
pub trait Matches {
    type MatchesIterator: MatchesIterator;
    /// Returns a [`MatchesIterator`] over the matches for a single field,
    /// or `None` if there are no matches in that field.
    fn get_matches(&self, field: &str) -> Result<Option<Self::MatchesIterator>>;

    type Matches: Matches;
    /// Returns a collection of [`Matches`] that make up this instance;
    /// if it is not a composite, then this returns an empty list.
    fn get_sub_matches(&mut self) -> Vec<Self::Matches>;

    fn field(&self) -> &[String];
}
macro_rules! either_matches {
    (
        $vis:vis $name:ident
        => { mi: $mi:ident }
        { $( $Variant:ident : $T:ident ),+ $(,)? }
    ) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> Matches for $name<$( $T ),+>
        where
            $( $T: Matches ),+
        {
            type MatchesIterator = $mi<$( < $T as Matches >::MatchesIterator ),+>;

            fn get_matches(
                &self,
                field: &str,
            ) -> Result<Option<Self::MatchesIterator>> {
                match self {
                    $(
                        Self::$Variant(inner) => {
                            let opt = inner.get_matches(field)?;
                            Ok(opt.map($mi::$Variant))
                        }
                    ),+
                }
            }

            type Matches = $name<$( < $T as Matches >::Matches ),+>;

            fn get_sub_matches(&mut self) -> Vec<Self::Matches> {
                match self {
                    $(
                        Self::$Variant(inner) => inner
                            .get_sub_matches()
                            .into_iter()
                            .map(Self::Matches::$Variant)
                            .collect(),
                    )+
                }
            }

            fn field(&self) -> &[String] {
                match self {
                    $( Self::$Variant(inner) => inner.field(), )+
                }
            }
        }
    };
}
either_matches!(
    pub Either2Matches
    => { mi: Either2MatchesIterator }
    { A: A, B: B }
);
either_matches!(
    pub Either3Matches
    => { mi: Either3MatchesIterator }
    { A: A, B: B,C:C }
);
either_matches!(
    pub Either4Matches
    => { mi: Either4MatchesIterator }
    { A: A, B: B,C:C,D:D }
);
either_matches!(
    pub Either5Matches
    => { mi: Either5MatchesIterator }
    { A: A, B: B, C: C, D: D, E: E }
);
