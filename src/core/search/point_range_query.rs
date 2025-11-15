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
use crate::core::index::index_reader_context::{IRCTermState, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::{IntersectVisitor, PointTree, PointValues, Relation};
use crate::core::index::query_timeout::QueryTimeout;
use crate::core::index::term_states::TermStates;
use crate::core::search::QueryCache;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::{AllDISI, DocIdSetIterator};
use crate::core::search::dummy::dummy_matches::DummyMatches;
use crate::core::search::dummy::dummy_scorer_supplier::DummyScorerSupplier;
use crate::core::search::dummy::dummy_two_phase_iterator::DummyTwoPhaseIterator;
use crate::core::search::dummy::dummy_weight::DummyWeight;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::similarities_impl::similarities::Similarity;
use crate::core::search::weight::{DefaultBulkScorer, Weight};
use crate::core::util::array_util::{ByteArrayComparator, ByteArrayComparatorEnum};
use crate::core::util::bit_set::BitSet;
use crate::core::util::doc_id_set_builder::DocIdSetBuilder;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::ints_ref::IntsRef;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Debug)]
pub struct PointRangeQuery {
    field: String,
    num_dims: i32,
    bytes_per_dim: i32,
    lower_point: Arc<Vec<u8>>,
    upper_point: Arc<Vec<u8>>,
}
impl PointRangeQuery {
    fn new(
        field: String,
        lower_point: Vec<u8>,
        upper_point: Vec<u8>,
        num_dims: i32,
    ) -> Result<Self> {
        Self::check_args(&field, lower_point.as_ref(), upper_point.as_ref())?;
        if num_dims <= 0 {
            return Err(LuceneError::illegal_argument(format!(
                "num_dims must be positive, got {}",
                num_dims
            )));
        }
        if lower_point.is_empty() {
            return Err(LuceneError::illegal_argument(
                "lower_point has length of zero".to_string(),
            ));
        }
        if !lower_point.len().is_multiple_of(num_dims as usize) {
            return Err(LuceneError::illegal_argument(
                "lower_point is not a fixed multiple of num_dims".to_string(),
            ));
        }
        if lower_point.len() != upper_point.len() {
            return Err(LuceneError::illegal_argument(format!(
                "lower_point has length={} but upper_point has different length={}",
                lower_point.len(),
                upper_point.len()
            )));
        }

        let bytes_per_dim = lower_point.len() as i32 / num_dims;

        Ok(Self {
            field,
            num_dims,
            bytes_per_dim,
            lower_point: Arc::new(lower_point),
            upper_point: Arc::new(upper_point),
        })
    }
    pub fn check_args(
        _field: &String,
        _lower_point: &Vec<u8>,
        _upper_point: &Vec<u8>,
    ) -> Result<()> {
        Ok(())
    }
    fn equals_to(&self, other: &PointRangeQuery) -> bool {
        self.field == other.field
            && self.num_dims == other.num_dims
            && self.bytes_per_dim == other.bytes_per_dim
            && self.lower_point == other.lower_point
            && self.upper_point == other.upper_point
    }
    fn to_string(&self, field: &str, lower_point_message: &str, up_point_message: &str) -> String {
        let mut sb = String::new();

        if self.field != field {
            sb.push_str(&self.field);
            sb.push(':');
        }

        for i in 0..self.num_dims {
            if i > 0 {
                sb.push(',');
            }

            sb.push('[');
            sb.push_str(lower_point_message);
            sb.push_str(" TO ");
            sb.push_str(up_point_message);
            sb.push(']');
        }

        sb
    }
    #[cfg(test)]
    fn get_lower_point(&self) -> &[u8] {
        &self.lower_point
    }
    #[cfg(test)]
    fn get_upper_point(&self) -> &[u8] {
        &self.upper_point
    }
}

impl Eq for PointRangeQuery {}

impl PartialEq<Self> for PointRangeQuery {
    fn eq(&self, other: &Self) -> bool {
        self.equals_to(other)
    }
}

impl Hash for PointRangeQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.field.hash(state);
        self.num_dims.hash(state);
        self.bytes_per_dim.hash(state);
        self.lower_point.hash(state);
        self.upper_point.hash(state);
    }
}
impl QueryBase for PointRangeQuery {
    fn as_string(&self, _field: &str) -> String {
        debug_assert!(false, "should never be called");
        "".to_string()
    }

    type Weight<S, IRC, QCP, QC>
        = DummyWeight<IRC::LeafReader>
    where
        S: Similarity,
        IRC: IndexReaderContext,
        QCP: QueryCachingPolicy,
        QC: QueryCache<IRC::LeafReader>;

    fn create_weight<S, IRC, QT, QCP, QC>(
        self,
        _searcher: &IndexSearcher<IRC, S, QT, QCP, QC>,
        _score_mode: &ScoreMode,
        _boost: f32,
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
        todo!()
    }

    type RewriteQuery = PointRangeQuery;

    fn visit<QV>(&self, visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

pub struct PointRangeWeight<LR>
where
    LR: LeafReader,
{
    query: Arc<Query>,
    comparator: ByteArrayComparatorEnum,
    _leaf_reader: PhantomData<LR>,
}
impl<LR> PointRangeWeight<LR>
where
    LR: LeafReader,
{
    pub fn new(query: PointRangeQuery, comparator: ByteArrayComparatorEnum) -> Self {
        Self {
            query: Arc::new(query.into()),
            comparator,
            _leaf_reader: PhantomData,
        }
    }
    fn matches(&self, packed_value: &[u8]) -> Result<bool> {
        let query = self.point_range_query()?;

        let num_dims = query.num_dims as usize;
        let bytes_per_dim = query.bytes_per_dim as usize;
        let mut offset = 0usize;
        for _ in 0..num_dims {
            if self
                .comparator
                .compare(packed_value, offset, query.lower_point.as_ref(), offset)
                < 0
            {
                // Doc's value is too low, in this dimension
                return Ok(false);
            }
            if self
                .comparator
                .compare(packed_value, offset, query.upper_point.as_ref(), offset)
                > 0
            {
                // Doc's value is too high, in this dimension
                return Ok(false);
            }
            offset += bytes_per_dim;
        }
        Ok(true)
    }
    fn relate(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        let q = self.point_range_query()?;

        let num_dims = q.num_dims as usize;
        let bytes_per_dim = q.bytes_per_dim as usize;

        let mut crosses = false;
        let mut offset = 0usize;

        for _ in 0..num_dims {
            if self
                .comparator
                .compare(min_packed_value, offset, &q.upper_point, offset)
                > 0
                || self
                    .comparator
                    .compare(max_packed_value, offset, &q.lower_point, offset)
                    < 0
            {
                return Ok(Relation::CellOutsideQuery);
            }

            if self
                .comparator
                .compare(min_packed_value, offset, &q.lower_point, offset)
                < 0
                || self
                    .comparator
                    .compare(max_packed_value, offset, &q.upper_point, offset)
                    > 0
            {
                crosses = true;
            }

            offset += bytes_per_dim;
        }

        if crosses {
            Ok(Relation::CellCrossesQuery)
        } else {
            Ok(Relation::CellInsideQuery)
        }
    }
    pub fn check_valid_point_values<PV>(&self, values: Option<&PV>) -> Result<bool>
    where
        PV: PointValues,
    {
        let values = match values {
            Some(v) => v,
            None => return Ok(false),
        };

        let q = self.point_range_query()?;
        let num_dims = q.num_dims;
        let bytes_per_dim = q.bytes_per_dim;
        let field = &q.field;

        if values.get_num_index_dimensions()? != num_dims {
            return Err(LuceneError::illegal_argument(format!(
                "field=\"{}\" was indexed with numIndexDimensions={} but this query has numDims={}",
                field,
                values.get_num_index_dimensions()?,
                num_dims
            )));
        }

        if values.get_bytes_per_dimension()? != bytes_per_dim {
            return Err(LuceneError::illegal_argument(format!(
                "field=\"{}\" was indexed with bytesPerDim={} but this query has bytesPerDim={}",
                field,
                values.get_bytes_per_dimension()?,
                bytes_per_dim
            )));
        }

        Ok(true)
    }
    fn get_intersect_visitor(
        result: DocIdSetBuilder,
        weight: &'_ PointRangeWeight<LR>,
    ) -> IntersectVisitorImpl1<'_, LR> {
        IntersectVisitorImpl1::new(result, weight)
    }

    fn get_inverse_intersect_visitor<'a>(
        result: &'a mut FixedBitSet,
        cost: &'a mut [i64],
        weight: &'a PointRangeWeight<LR>,
    ) -> IntersectVisitorImpl<'a, LR> {
        IntersectVisitorImpl::new(result, cost, weight)
    }

    fn point_count(&self, point_tree: &mut impl PointTree) -> Result<i64> {
        let mut visitor = IntersectVisitorImpl2::new(self);
        self.point_count_with_visitor(&mut visitor, point_tree)?;
        Ok(visitor.matching_node_count)
    }

    fn point_count_with_visitor(
        &self,
        visitor: &mut IntersectVisitorImpl2<LR>,
        point_tree: &mut impl PointTree,
    ) -> Result<()> {
        let relation = visitor.compare(
            point_tree.get_min_packed_value()?,
            point_tree.get_max_packed_value()?,
        )?;

        match relation {
            Relation::CellOutsideQuery => {
                // This cell is fully outside the query shape: return 0 as the count of its nodes
                Ok(())
            },

            Relation::CellInsideQuery => {
                // This cell is fully inside the query shape: return the size of the entire node as the
                // count
                visitor.matching_node_count += point_tree.size()?;
                Ok(())
            },

            Relation::CellCrossesQuery => {
                // The cell crosses the shape boundary, or the cell fully contains the query, so we fall
                // through and do full counting.
                if point_tree.move_to_child()? {
                    loop {
                        self.point_count_with_visitor(visitor, point_tree)?;
                        if !point_tree.move_to_sibling()? {
                            break;
                        }
                    }
                    point_tree.move_to_parent()?;
                } else {
                    // we have reached a leaf node here.
                    point_tree.visit_doc_values(visitor)?;
                    // leaf node count is saved in the matchingNodeCount array by the visitor
                }
                Ok(())
            },
        }
    }
    fn point_range_query(&self) -> Result<&PointRangeQuery> {
        match self.query.as_ref() {
            Query::PointRange(q) => Ok(q),
            _ => Err(LuceneError::illegal_state("should never be here")),
        }
    }
}

impl<LR> SegmentCacheable<LR> for PointRangeWeight<LR>
where
    LR: LeafReader,
{
    fn is_cacheable(&self, _ctx: &LeafReaderContext<LR>) -> bool {
        true
    }
}

impl<LR> Weight<LR> for PointRangeWeight<LR>
where
    LR: LeafReader,
{
    type Matches = DummyMatches;

    fn matches(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Option<Self::Matches>> {
        todo!()
    }

    fn explain(&self, context: &LeafReaderContext<LR>, doc: i32) -> Result<Explanation> {
        todo!()
    }

    fn get_query(&self) -> Arc<Query> {
        todo!()
    }

    type ScorerSupplier = DummyScorerSupplier;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        todo!()
    }

    fn count(&self, context: &LeafReaderContext<LR>) -> Result<i32> {
        let query = self.point_range_query()?;
        let reader = context.reader();

        let values = reader.get_point_values(query.field.as_str())?;

        if !self.check_valid_point_values(values.as_ref())? {
            return Ok(0);
        }

        if !reader.has_deletions()? {
            let values = values.unwrap();

            let relation = self.relate(
                values.get_min_packed_value()?.as_ref().unwrap().as_ref(),
                values.get_max_packed_value()?.as_ref().unwrap().as_ref(),
            )?;

            if relation == Relation::CellInsideQuery {
                return values.get_doc_count();
            }

            // only 1D: we have the guarantee that it will actually run fast since there are at most 2
            // crossing leaves.
            // docCount == size : counting according number of points in leaf node, so must be
            // single-valued.
            if query.num_dims == 1 && values.get_doc_count()? == values.size()? as i32 {
                let mut tree = values.get_point_tree()?;
                return Ok(self.point_count(&mut tree)? as i32);
            }
        }
        self.default_count(context)
    }
}

pub struct ScorerSupplierImpl1 {
    weight: ConstantScoreWeight,
    score_mode: ScoreMode,
    max_doc: i32,
    cost: i64,
}
impl ScorerSupplierImpl1 {
    pub fn new(
        weight: ConstantScoreWeight,
        score_mode: ScoreMode,
        max_doc: i32,
        cost: i64,
    ) -> Self {
        Self {
            weight,
            score_mode,
            max_doc,
            cost,
        }
    }
}

pub struct ScorerSupplierImpl {
    weight: ConstantScoreWeight,
    score_mode: ScoreMode,
    max_doc: i32,
}
impl ScorerSupplierImpl {
    pub fn new(weight: ConstantScoreWeight, score_mode: ScoreMode, max_doc: i32) -> Self {
        Self {
            weight,
            score_mode,
            max_doc,
        }
    }
}
impl<LR> ScorerSupplier<LR> for ScorerSupplierImpl
where
    LR: LeafReader,
{
    type Scorer = ConstantScoreScorer<AllDISI, DummyTwoPhaseIterator>;
    type BulkScorer = DefaultBulkScorer<Self::Scorer>;

    fn get(
        &mut self,
        _lead_cost: i64,
        context: &LeafReaderContext<LR>,
    ) -> Result<Option<Self::Scorer>> {
        debug_assert!(context.reader().max_doc()? == self.max_doc);
        let score = self.weight.score();
        Ok(Some(ConstantScoreScorer::with_disi(
            score,
            self.score_mode,
            AllDISI::new(self.max_doc),
        )))
    }

    fn bulk_scorer(&mut self, context: &LeafReaderContext<LR>) -> Result<Option<Self::BulkScorer>> {
        Ok(Some(self.default_bulk_scorer(context)?))
    }

    fn cost(&mut self, context: &LeafReaderContext<LR>) -> Result<i64> {
        debug_assert!(context.reader().max_doc()? == self.max_doc);
        Ok(self.max_doc as i64)
    }
}

struct IntersectVisitorImpl<'a, LR>
where
    LR: LeafReader,
{
    result: &'a mut FixedBitSet,
    cost: &'a mut [i64],
    weight: &'a PointRangeWeight<LR>,
}
impl<'a, LR> IntersectVisitorImpl<'a, LR>
where
    LR: LeafReader,
{
    fn new(
        result: &'a mut FixedBitSet,
        cost: &'a mut [i64],
        weight: &'a PointRangeWeight<LR>,
    ) -> Self {
        Self {
            result,
            cost,
            weight,
        }
    }
}
impl<LR> IntersectVisitor for IntersectVisitorImpl<'_, LR>
where
    LR: LeafReader,
{
    fn visit(&mut self, doc_id: i32) -> Result<()> {
        self.result.clear_with_index(doc_id);
        self.cost[doc_id as usize] -= 1;
        Ok(())
    }

    fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
        self.result.and_not_iter(iterator)?;
        self.cost[0] = self.cost[0].max(iterator.cost()?);
        Ok(())
    }

    fn visit_with_ints_ref(&mut self, ints_ref: &IntsRef<Vec<i32>>) -> Result<()> {
        for i in ints_ref.offset..(ints_ref.offset + ints_ref.length) {
            self.result.clear_with_index(ints_ref.ints[i])
        }
        self.cost[0] -= ints_ref.length as i64;
        Ok(())
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        if self.weight.matches(packed_value)? {
            self.visit(doc_id)?;
        }
        Ok(())
    }

    fn visit_iterator_with_packed_value(
        &mut self,
        iterator: &mut impl DocIdSetIterator,
        packed_value: &[u8],
    ) -> Result<()> {
        if self.weight.matches(packed_value)? {
            self.visit_with_iterator(iterator)?;
        }
        Ok(())
    }

    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        let relation = self.weight.relate(min_packed_value, max_packed_value)?;

        Ok(match relation {
            // all points match, skip this subtree
            Relation::CellInsideQuery => Relation::CellOutsideQuery,
            // none of the points match, clear all documents
            Relation::CellOutsideQuery => Relation::CellInsideQuery,
            Relation::CellCrossesQuery => Relation::CellCrossesQuery,
        })
    }
}
struct IntersectVisitorImpl1<'a, LR>
where
    LR: LeafReader,
{
    result: DocIdSetBuilder,
    weight: &'a PointRangeWeight<LR>,
}

impl<'a, LR> IntersectVisitorImpl1<'a, LR>
where
    LR: LeafReader,
{
    pub fn new(result: DocIdSetBuilder, weight: &'a PointRangeWeight<LR>) -> Self {
        Self { result, weight }
    }
}

impl<'a, LR> IntersectVisitor for IntersectVisitorImpl1<'a, LR>
where
    LR: LeafReader,
{
    fn grow(&mut self, count: i32) -> Result<()> {
        self.result.grow(count);
        Ok(())
    }

    fn visit(&mut self, doc_id: i32) -> Result<()> {
        self.result.add_doc(doc_id);
        Ok(())
    }

    fn visit_with_iterator(&mut self, iterator: &mut impl DocIdSetIterator) -> Result<()> {
        self.result.add_disi(iterator)?;
        Ok(())
    }

    fn visit_with_ints_ref(&mut self, ints_ref: &IntsRef<Vec<i32>>) -> Result<()> {
        for i in ints_ref.offset..(ints_ref.offset + ints_ref.length) {
            self.result.add_doc(ints_ref.ints[i]);
        }
        Ok(())
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        if self.weight.matches(packed_value)? {
            self.visit(doc_id)?;
        }
        Ok(())
    }

    fn visit_iterator_with_packed_value(
        &mut self,
        iterator: &mut impl DocIdSetIterator,
        packed_value: &[u8],
    ) -> Result<()> {
        if self.weight.matches(packed_value)? {
            self.result.add_disi(iterator)?;
        }
        Ok(())
    }

    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        self.weight.relate(min_packed_value, max_packed_value)
    }
}

struct IntersectVisitorImpl2<'a, LR>
where
    LR: LeafReader,
{
    weight: &'a PointRangeWeight<LR>,
    matching_node_count: i64,
}
impl<'a, LR> IntersectVisitorImpl2<'a, LR>
where
    LR: LeafReader,
{
    pub fn new(weight: &'a PointRangeWeight<LR>) -> Self {
        Self {
            weight,
            matching_node_count: 0,
        }
    }
}
impl<'a, LR> IntersectVisitor for IntersectVisitorImpl2<'a, LR>
where
    LR: LeafReader,
{
    fn visit(&mut self, doc_id: i32) -> Result<()> {
        Err(LuceneError::unsupported_operation(format!(
            "This IntersectVisitor does not perform any actions on a docID={} node being visited",
            doc_id
        )))
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        if self.weight.matches(packed_value)? {
            self.matching_node_count += 1;
        }
        Ok(())
    }

    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        self.weight.relate(min_packed_value, max_packed_value)
    }
}
