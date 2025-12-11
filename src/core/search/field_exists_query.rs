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
use crate::core::index::doc_values::DocValues;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader_context::{IRCTermState, IndexReaderContext};
use crate::core::index::leaf_reader::{LRDisis, LRNormNumericDocValues, LeafReader};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::PointValues;
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::term_states::TermStates;
use crate::core::index::terms::Terms;
use crate::core::search::QueryCache;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::Either3DocIdSetIterator;
use crate::core::search::dummy::dummy_disi::DummyDISI;
use crate::core::search::dummy::dummy_query::DummyQuery;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::weight::{DefaultScorerSupplier, Weight};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

/// A `Query` that matches documents that contain either a `KnnFloatVectorField`,
/// `org.apache.lucene.document.KnnByteVectorField`, or a field that indexes norms
/// or doc values.
#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct FieldExistsQuery {
    field: String,
}
impl FieldExistsQuery {
    /// Create a query that will match that have a value for the given [`field`].
    pub fn new<T>(field: T) -> Self
    where
        T: Into<String>,
    {
        let field = field.into();
        Self { field }
    }
    pub fn get_field(&self) -> &str {
        &self.field
    }
    fn build_error_msg(&self, field_info: &FieldInfo) -> String {
        format!(
            "FieldExistsQuery requires that the field indexes doc values, norms or vectors, but field '{}' exists and indexes neither of these data structures",
            field_info.name
        )
    }
    fn get_vector_values_size<LR>(&self, _fi: &FieldInfo, _reader: &LR) -> i32
    where
        LR: LeafReader,
    {
        todo!()
    }
}

impl QueryBase for FieldExistsQuery {
    fn as_string(&self, _field: &str) -> String {
        format!("FieldExistsQuery [field={}]", self.field)
    }

    type Weight<S, IRC, QCP, QC>
        = FieldExistsWeight<IRC::LeafReader>
    where
        S: Similarity,
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache<IRC::LeafReader>;

    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
        score_mode: &ScoreMode,
        boost: f32,
        _per_reader_term_state: Option<TermStates<IRCTermState<IRC>>>,
    ) -> Result<Self::Weight<S, IRC, QCP, QC>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache<IRC::LeafReader>,
        Self: Sized,
    {
        Ok(FieldExistsWeight::new(boost, self, *score_mode))
    }

    type RewriteQuery = DummyQuery;

    fn rewrite<IRC, S, QT, QCP, QC>(
        &self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
    ) -> Result<Option<Self::RewriteQuery>>
    where
        IRC: IndexReaderContext,
        S: Similarity,
        QT: QueryTimeout,
        QCP: QueryCachingPolicy,
        QC: QueryCache<IRC::LeafReader>,
    {
        todo!()
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}
pub struct FieldExistsWeight<LR>
where
    LR: LeafReader,
{
    query: FieldExistsQuery,
    base: ConstantScoreWeight,
    parent_query: Arc<Query>,
    score_mode: ScoreMode,
    _leaf_reader: PhantomData<LR>,
    score: f32,
}
impl<LR> FieldExistsWeight<LR>
where
    LR: LeafReader,
{
    fn new(score: f32, query: FieldExistsQuery, score_mode: ScoreMode) -> Self {
        let query_clone = query.clone();
        let parent_query = Arc::new(query_clone.into());
        Self {
            base: ConstantScoreWeight::new(score),
            query,
            parent_query,
            score_mode,
            _leaf_reader: PhantomData,
            score,
        }
    }
}

impl<LR> SegmentCacheable<LR> for FieldExistsWeight<LR>
where
    LR: LeafReader,
{
    fn is_cacheable(&self, ctx: &LeafReaderContext<LR>) -> Result<bool> {
        let field_infos = ctx.reader().get_field_infos()?;
        let field_info = field_infos.field_info_by_name(&self.query.field);

        if let Some(fi) = field_info
            && *fi.get_doc_values_type() != DocValuesType::None
        {
            let field = vec![self.query.field.clone()];
            return DocValues::is_cacheable(ctx, field.as_ref());
        }
        Ok(true)
    }
}

pub type Disi<LR> = Either3DocIdSetIterator<LRNormNumericDocValues<LR>, DummyDISI, LRDisis<LR>>;
impl<LR> Weight<LR> for FieldExistsWeight<LR>
where
    LR: LeafReader,
{
    type Matches = MatchWithNoTerms;

    fn matches(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Option<Self::Matches>> {
        self.default_matches(context, doc)
    }

    fn explain(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        let scorer = self.scorer(context)?;
        self.base
            .explain(scorer, doc, self.parent_query.as_string(""))
    }

    fn get_query(&self) -> Arc<Query> {
        self.parent_query.clone()
    }

    type ScorerSupplier =
        DefaultScorerSupplier<ConstantScoreScorer<Disi<LR>, DummyTwoPhaseIterator>>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        let reader = context.reader();
        let field = self.query.get_field();
        let field_infos = reader.get_field_infos()?;
        let field_info = field_infos.field_info_by_name(field);

        let Some(fi) = field_info else {
            return Ok(None);
        };
        let disi_opt = if fi.has_norms() {
            // the field indexes norms
            reader.get_norm_values(field)?.map(Disi::<LR>::A)
        } else if fi.get_vector_dimension() != 0 {
            // TODO IMPORTANT vector未实现
            unimplemented!();
        } else if *fi.get_doc_values_type() != DocValuesType::None {
            match *fi.get_doc_values_type() {
                DocValuesType::Numeric => reader
                    .get_numeric_doc_values(field)?
                    .map(|numeric| Disi::<LR>::C(LRDisis::<LR>::A(numeric))),

                DocValuesType::Binary => reader
                    .get_binary_doc_values(field)?
                    .map(|binary| Disi::<LR>::C(LRDisis::<LR>::B(binary))),

                DocValuesType::Sorted => reader
                    .get_sorted_doc_values(field)?
                    .map(|sorted| Disi::<LR>::C(LRDisis::<LR>::C(sorted))),

                DocValuesType::SortedNumeric => reader
                    .get_sorted_numeric_doc_values(field)?
                    .map(|sorted_numeric| Disi::<LR>::C(LRDisis::<LR>::D(sorted_numeric))),

                DocValuesType::SortedSet => reader
                    .get_sorted_set_doc_values(field)?
                    .map(|sorted_set| Disi::<LR>::C(LRDisis::<LR>::E(sorted_set))),
                DocValuesType::None => None,
            }
        } else {
            return Err(LuceneError::illegal_argument(
                self.query.build_error_msg(fi.as_ref()),
            ));
        };
        match disi_opt {
            Some(disi) => Ok(Some(DefaultScorerSupplier::new(
                ConstantScoreScorer::with_disi(self.score, self.score_mode, disi),
            ))),
            None => Ok(None),
        }
    }

    fn count(&self, ctx: &LeafReaderContext<LR>) -> Result<i32> {
        let reader = ctx.reader();

        let field_infos = reader.get_field_infos()?;
        let field_info = field_infos.field_info_by_name(self.query.get_field());

        let Some(fi) = field_info else {
            return Ok(0);
        };

        if fi.has_norms() {
            // the field indexes norms
            // If every field has a value then we can shortcut
            let doc_count = reader.get_doc_count(self.query.get_field())?;
            if doc_count == reader.max_doc()? {
                return reader.num_docs();
            }
            return self.default_count(ctx);
        }

        if fi.has_vector_values() {
            // the field indexes vectors
            if !reader.has_deletions()? {
                return Ok(self.query.get_vector_values_size(fi.as_ref(), reader));
            }
            return self.default_count(ctx);
        }

        if *fi.get_doc_values_type() != DocValuesType::None {
            // the field indexes doc values
            if !reader.has_deletions()? {
                if fi.get_point_dimension_count() > 0 {
                    if let Some(point_values) = reader.get_point_values(self.query.get_field())? {
                        return point_values.get_doc_count();
                    } else {
                        return Ok(0);
                    }
                }

                if *fi.get_index_options() != IndexOptions::None {
                    if let Some(terms) = reader.terms(self.query.get_field())? {
                        return terms.get_doc_count();
                    } else {
                        return Ok(0);
                    }
                }
            }

            return self.default_count(ctx);
        }

        Err(LuceneError::illegal_argument(
            self.query.build_error_msg(fi.as_ref()),
        ))
    }
}

/// Returns a DocIdSetIterator from the given field or None if the field doesn't
/// exist in the reader or if the reader has no doc values for the field.
pub fn get_doc_values_doc_id_set_iterator<LR>(
    field: &str,
    reader: &LR,
) -> Result<Option<LRDisis<LR>>>
where
    LR: LeafReader,
{
    let field_info = reader.get_field_infos()?.field_info_by_name(field);

    let Some(fi) = field_info else {
        return Ok(None);
    };
    let doc_value_type = *fi.get_doc_values_type();
    match doc_value_type {
        DocValuesType::Numeric => match reader.get_numeric_doc_values(field)? {
            Some(numeric) => Ok(Some(LRDisis::<LR>::A(numeric))),
            None => Ok(None),
        },

        DocValuesType::Binary => match reader.get_binary_doc_values(field)? {
            Some(binary) => Ok(Some(LRDisis::<LR>::B(binary))),
            None => Ok(None),
        },

        DocValuesType::Sorted => match reader.get_sorted_doc_values(field)? {
            Some(sorted) => Ok(Some(LRDisis::<LR>::C(sorted))),
            None => Ok(None),
        },

        DocValuesType::SortedNumeric => match reader.get_sorted_numeric_doc_values(field)? {
            Some(sorted_numeric) => Ok(Some(LRDisis::<LR>::D(sorted_numeric))),
            None => Ok(None),
        },

        DocValuesType::SortedSet => match reader.get_sorted_set_doc_values(field)? {
            Some(sorted_set) => Ok(Some(LRDisis::<LR>::E(sorted_set))),
            None => Ok(None),
        },
        DocValuesType::None => Ok(None),
    }
}
