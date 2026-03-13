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
use crate::core::index::index_reader::Identity;
/// A [`Query`] wrapper that allows giving a boost to the wrapped query.
///
/// Boost values that are less than one will give less importance to this query
/// compared to other ones, while values that are greater than one will give
/// more importance to the scores returned by this query.
///
///
/// More complex boosts can be applied by using `FunctionScoreQuery` in the
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{IntoBoxQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct BoostQuery {
    id: Identity,
    query: Box<Query>,
    boost: f32,
}
impl BoostQuery {
    pub fn new<T>(query: T, boost: f32) -> Result<Self>
    where
        T: IntoBoxQuery,
    {
        let query = query.into_box_query();
        if !boost.is_finite() || boost < 0.0 || (boost == 0.0 && boost.is_sign_negative()) {
            return Err(LuceneError::illegal_argument(format!(
                "boost must be a positive float, got {:.1}",
                boost
            )));
        }
        Ok(Self {
            id: Identity::new(),
            query,
            boost,
        })
    }
    pub fn get_query(&self) -> &Query {
        &self.query
    }
    pub fn get_boost(&self) -> f32 {
        self.boost
    }
    pub(crate) fn into_inner(self) -> Query {
        *self.query
    }
}

impl PartialEq for BoostQuery {
    fn eq(&self, other: &Self) -> bool {
        self.boost.to_bits() == other.boost.to_bits() && self.query == other.query
    }
}
impl Eq for BoostQuery {}
impl QueryBase for BoostQuery {
    fn as_string(&self, field: &str) -> Result<String> {
        let inner = self.query.as_string(field)?;
        let mut s = String::new();
        s.push('(');
        s.push_str(&inner);
        s.push(')');
        s.push('^');
        s.push_str(&format!("{:.1}", self.boost));
        Ok(s)
    }

    fn create_weight<IRC>(
        self,
        searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        self.query
            .create_weight(searcher, score_mode, self.boost * boost)
    }

    fn rewrite<IRC>(mut self, searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        let query_id = self.query.identity().clone();

        let rewritten = self.query.rewrite(searcher)?;

        if self.boost == 1.0 {
            return Ok(rewritten);
        }

        let rewritten = match rewritten {
            Query::Boost(in_boost) => {
                return Ok(BoostQuery::new(in_boost.query, self.boost * in_boost.boost)?.into());
            },
            Query::MatchNoDocs(_) => {
                return Ok(rewritten);
            },
            other => other,
        };

        if self.boost == 0.0 && !matches!(rewritten, Query::ConstantScore(_)) {
            return Ok(BoostQuery::new(ConstantScoreQuery::new(rewritten), 0.0)?.into());
        }

        if &query_id != rewritten.identity() {
            return Ok(BoostQuery::new(rewritten, self.boost)?.into());
        }
        self.query = Box::new(rewritten);
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

impl Hash for BoostQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query.hash(state);
        self.boost.to_bits().hash(state);
    }
}

impl HasIdentity for BoostQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use crate::core::index::multi_reader::MultiReader;
    use crate::core::index::term::Term;
    use crate::core::search::boolean_clause::Occur;
    use crate::core::search::boolean_query::Builder;
    use crate::core::search::boost_query::BoostQuery;
    use crate::core::search::constant_score_query::ConstantScoreQuery;
    use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
    use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
    use crate::core::search::phrase_query::PhraseQuery;
    use crate::core::search::query::{Query, QueryBase};
    use crate::core::search::term_query::TermQuery;
    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
        new_searcher_with_reader, random,
    };
    use rand::RngExt;

    #[allow(dead_code)] // for quick search
    struct TestBoostQuery;

    #[test]
    fn test_validation() -> Result<()> {
        let err = BoostQuery::new(MatchAllDocsQuery::new(), -3.0).unwrap_err();
        match err {
            LuceneError::IllegalArgument(msg) => {
                assert_eq!("boost must be a positive float, got -3.0", msg.to_string())
            },
            _ => unreachable!("expected IllegalArgumentException"),
        }

        let err = BoostQuery::new(MatchAllDocsQuery::new(), -0.0).unwrap_err();
        match err {
            LuceneError::IllegalArgument(msg) => {
                assert_eq!("boost must be a positive float, got -0.0", msg.to_string())
            },
            _ => unreachable!("expected IllegalArgumentException"),
        }

        let err = BoostQuery::new(MatchAllDocsQuery::new(), f32::NAN).unwrap_err();
        match err {
            LuceneError::IllegalArgument(msg) => {
                assert_eq!("boost must be a positive float, got NaN", msg.to_string())
            },
            _ => unreachable!("expected IllegalArgumentException"),
        }

        Ok(())
    }

    #[test]
    fn test_equals() -> Result<()> {
        let mut random = random();

        let boost = random.random::<f32>() * 3.0;
        let q1 = BoostQuery::new(MatchAllDocsQuery::new(), boost)?;
        let q2 = BoostQuery::new(MatchAllDocsQuery::new(), boost)?;
        assert_eq!(q1, q2);
        assert_eq!(q1.get_boost(), q2.get_boost());

        let mut boost2 = boost;
        while boost == boost2 {
            boost2 = random.random::<f32>() * 3.0;
        }

        let q3 = BoostQuery::new(MatchAllDocsQuery::new(), boost2)?;
        assert_ne!(q1, q3);

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h1 = DefaultHasher::new();
        q1.hash(&mut h1);

        let mut h3 = DefaultHasher::new();
        q3.hash(&mut h3);

        assert_ne!(h1.finish(), h3.finish());

        Ok(())
    }

    #[test]
    fn test_to_string() -> Result<()> {
        assert_eq!(
            "(foo:bar)^2.0",
            BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?.as_string("")?
        );

        let mut builder = Builder::new();
        builder.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
        builder.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
        let bq = builder.build();

        assert_eq!(
            "(foo:bar foo:baz)^2.0",
            BoostQuery::new(bq, 2.0)?.as_string("")?
        );

        Ok(())
    }

    #[test]
    fn test_rewrite() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let q = BoostQuery::new(PhraseQuery::from_terms_no_slop("foo", &["bar"])?, 2.0)?;
        let v: Query = BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?.into();
        assert_eq!(v, searcher.rewrite(q)?);

        let q = BoostQuery::new(BoostQuery::new(MatchAllDocsQuery::new(), 3.0)?, 2.0)?;
        let v: Query = BoostQuery::new(MatchAllDocsQuery::new(), 6.0)?.into();
        assert_eq!(v, searcher.rewrite(q)?);

        let q = BoostQuery::new(MatchAllDocsQuery::new(), 0.0)?;
        let v: Query =
            BoostQuery::new(ConstantScoreQuery::new(MatchAllDocsQuery::new()), 0.0)?.into();
        assert_eq!(v, searcher.rewrite(q)?);

        Ok(())
    }

    #[test]
    fn test_rewrite_bubbles_up_match_no_docs_query() -> Result<()> {
        let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

        let query = BoostQuery::new(MatchNoDocsQuery::new(), 2.0)?;
        let v: Query = MatchNoDocsQuery::new().into();
        assert_eq!(v, searcher.rewrite(query)?);

        let query = BoostQuery::new(MatchNoDocsQuery::new(), 0.0)?;
        let v: Query = MatchNoDocsQuery::new().into();
        assert_eq!(v, searcher.rewrite(query)?);

        Ok(())
    }
}
