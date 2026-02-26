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
use crate::core::document::sorted_numeric_doc_values_range_query::DISI;
use crate::core::index::BytesRef;
use crate::core::index::doc_values::{DocValues, SortedSet};
use crate::core::index::doc_values_skipper::DocValuesSkipper;
use crate::core::index::index_reader::{Identity, IndexReader};
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::search::constant_score_scorer::ConstantScoreScorer;
use crate::core::search::constant_score_weight::ConstantScoreWeight;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{AllDISI, DocIdSetIterator, EmptyDISI, RangeDISI};
use crate::core::search::doc_values_range_iterator::DocValuesRangeIterator;
use crate::core::search::explanation::Explanation;
use crate::core::search::field_exists_query::FieldExistsQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
    Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::two_phase_iterator::{TwoPhaseIterator, TwoPhaseIteratorEnum2};
use crate::core::search::weight::Weight;
use crate::core::util::core_helper::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SortedSetDocValuesRangeQuery {
    id: Identity,
    pub field: String,
    pub lower_value: Option<BytesRef<Vec<u8>>>,
    pub upper_value: Option<BytesRef<Vec<u8>>>,
    pub lower_inclusive: bool,
    pub upper_inclusive: bool,
}
impl SortedSetDocValuesRangeQuery {
    pub fn new(
        field: String,
        lower_value: Option<BytesRef<Vec<u8>>>,
        upper_value: Option<BytesRef<Vec<u8>>>,
        lower_inclusive: bool,
        upper_inclusive: bool,
    ) -> Self {
        let lower_inclusive = lower_inclusive && lower_value.is_some();
        let upper_inclusive = upper_inclusive && upper_value.is_some();

        SortedSetDocValuesRangeQuery {
            id: Identity::new(),
            field,
            lower_value,
            upper_value,
            lower_inclusive,
            upper_inclusive,
        }
    }
}
impl PartialEq for SortedSetDocValuesRangeQuery {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field
            && self.lower_value == other.lower_value
            && self.upper_value == other.upper_value
            && self.lower_inclusive == other.lower_inclusive
            && self.upper_inclusive == other.upper_inclusive
    }
}
impl Eq for SortedSetDocValuesRangeQuery {}

impl Hash for SortedSetDocValuesRangeQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.field.hash(state);
        self.lower_value.hash(state);
        self.upper_value.hash(state);
        self.lower_inclusive.hash(state);
        self.upper_inclusive.hash(state);
    }
}

impl HasIdentity for SortedSetDocValuesRangeQuery {
    fn identity(&self) -> &Identity {
        &self.id
    }
}
impl QueryBase for SortedSetDocValuesRangeQuery {
    fn as_string(&self, field: &str) -> String {
        let mut b = String::new();
        if self.field.as_str() != field {
            b.push_str(self.field.as_str());
            b.push(':');
        }
        b.push(if self.lower_inclusive { '[' } else { '{' });
        match &self.lower_value {
            None => b.push('*'),
            Some(v) => b.push_str(&format!("{}", v)),
        }
        b.push_str(" TO ");
        match &self.upper_value {
            None => b.push('*'),
            Some(v) => b.push_str(&format!("{}", v)),
        }
        b.push(if self.upper_inclusive { ']' } else { '}' });
        b
    }

    fn create_weight<IRC>(
        self,
        _searcher: &IndexSearcher<IRC>,
        score_mode: &ScoreMode,
        boost: f32,
    ) -> Result<QueryWeight<IRC>>
    where
        IRC: IndexReaderContext,
        Self: Sized,
        IRCLeafReader<IRC>: 'static,
    {
        Ok(Box::new(SortedSetDocValuesRangeQueryWeight::new(
            self,
            boost,
            *score_mode,
        )))
    }

    fn rewrite<IRC>(self, _searcher: &IndexSearcher<IRC>) -> Result<Query>
    where
        IRC: IndexReaderContext,
        Self: Sized,
    {
        if self.lower_value.is_none() && self.upper_value.is_none() {
            return Ok(FieldExistsQuery::new(self.field).into());
        }
        Ok(self.into())
    }

    fn visit<QV>(&self, _visitor: &QV)
    where
        QV: QueryVisitor,
    {
        todo!()
    }
}

pub struct SortedSetDocValuesRangeQueryWeight<IRC>
where
    IRC: IndexReaderContext,
{
    query: SortedSetDocValuesRangeQuery,
    base: ConstantScoreWeight,
    parent_query: Arc<Query>,
    score_mode: ScoreMode,
    _irc: PhantomData<IRC>,
}
impl<IRC> SortedSetDocValuesRangeQueryWeight<IRC>
where
    IRC: IndexReaderContext,
{
    pub(crate) fn new(
        query: SortedSetDocValuesRangeQuery,
        score: f32,
        score_mode: ScoreMode,
    ) -> Self {
        let query_clone = query.clone();
        let parent_query = Arc::new(query.into());
        SortedSetDocValuesRangeQueryWeight {
            query: query_clone,
            base: ConstantScoreWeight::new(score),
            parent_query,
            score_mode,
            _irc: PhantomData,
        }
    }
}

impl<IRC> SegmentCacheable for SortedSetDocValuesRangeQueryWeight<IRC>
where
    IRC: IndexReaderContext,
{
    type IRC = IRC;

    fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
        let field = vec![self.query.field.clone()];
        DocValues::is_cacheable(ctx, field.as_ref())
    }
}

impl<IRC> Weight for SortedSetDocValuesRangeQueryWeight<IRC>
where
    IRCLeafReader<IRC>: LeafReader + 'static,
    IRC: IndexReaderContext,
{
    type Matches = MatchWithNoTerms;

    fn matches(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::Matches>> {
        self.default_matches(context, doc, searcher)
    }

    fn explain(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        doc: i32,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Explanation> {
        let scorer = self.scorer(context, searcher)?;
        self.base
            .explain(scorer, doc, self.parent_query.as_string(""))
    }

    fn get_query(&self) -> Arc<Query> {
        self.parent_query.clone()
    }

    type ScorerSupplier = QueryWeightSs<IRC>;

    fn scorer_supplier(
        &self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::ScorerSupplier>> {
        if context
            .reader()
            .get_field_infos()?
            .field_info_by_name(&self.query.field)
            .is_none()
        {
            return Ok(None);
        }

        let values = DocValues::get_sorted_set(context.reader(), &self.query.field)?;
        Ok(Some(Box::new(ScorerSupplierImpl3::new(
            self.query.clone(),
            values,
            self.base.score(),
            self.score_mode,
        )?)))
    }
}
fn get_doc_id_set_iterator_or_null_for_primary_sort<LR, SDV, SK>(
    reader: &LR,
    sorted_doc_values: &mut SDV,
    skipper: &mut SK,
    min_ord: i64,
    max_ord: i64,
    field: &str,
) -> Result<Option<DISI>>
where
    SDV: SortedDocValues,
    SK: DocValuesSkipper,
    LR: LeafReader,
{
    if skipper.doc_count() != reader.max_doc()? {
        return Ok(None);
    }

    let index_sort = reader.get_metadata()?.get_sort();
    if index_sort.is_none() {
        return Ok(None);
    }
    let index_sort = index_sort.as_ref().unwrap();

    let sort_fields = index_sort.get_sort();
    let Some(first) = sort_fields.first() else {
        return Ok(None);
    };

    if first.get_field() != Some(field) {
        return Ok(None);
    }

    let min_doc_id: i32;
    let max_doc_id: i32;

    if first.get_reverse() {
        if skipper.max_value() <= max_ord {
            min_doc_id = 0;
        } else {
            skipper.advance_with_range(i64::MIN, max_ord)?;
            min_doc_id = next_doc_ord(skipper.min_doc_id_with_level(0), sorted_doc_values, |l| {
                l <= max_ord
            })?;
        }

        if skipper.min_value() >= min_ord {
            max_doc_id = skipper.doc_count();
        } else {
            skipper.advance_with_range(i64::MIN, min_ord)?;
            max_doc_id = next_doc_ord(skipper.min_doc_id_with_level(0), sorted_doc_values, |l| {
                l < min_ord
            })?;
        }
    } else {
        if skipper.min_value() >= min_ord {
            min_doc_id = 0;
        } else {
            skipper.advance_with_range(min_ord, i64::MAX)?;
            min_doc_id = next_doc_ord(skipper.min_doc_id_with_level(0), sorted_doc_values, |l| {
                l >= min_ord
            })?;
        }

        if skipper.max_value() <= max_ord {
            max_doc_id = skipper.doc_count();
        } else {
            skipper.advance_with_range(max_ord, i64::MAX)?;
            max_doc_id = next_doc_ord(skipper.min_doc_id_with_level(0), sorted_doc_values, |l| {
                l > max_ord
            })?;
        }
    }

    if min_doc_id == max_doc_id {
        return Ok(Some(DISI::A(EmptyDISI::default())));
    }

    Ok(Some(DISI::B(RangeDISI::new(min_doc_id, max_doc_id)?)))
}

fn next_doc_ord<SDV, P>(start_doc: i32, doc_values: &mut SDV, predicate: P) -> Result<i32>
where
    SDV: SortedDocValues,
    P: Fn(i64) -> bool,
{
    let mut doc = doc_values.doc_id();
    if start_doc > doc {
        doc = doc_values.advance(start_doc)?;
    }

    while doc < NO_MORE_DOCS {
        if predicate(doc_values.ord_value()? as i64) {
            break;
        }
        doc = doc_values.next_doc()?;
    }

    Ok(doc)
}
pub struct ScorerSupplierImpl3<IRC>
where
    IRC: IndexReaderContext,
{
    query: SortedSetDocValuesRangeQuery,
    values: Option<SortedSet<IRCLeafReader<IRC>>>,
    cost: i64,
    score: f32,
    score_mode: ScoreMode,
}
impl<IRC> ScorerSupplierImpl3<IRC>
where
    IRC: IndexReaderContext,
{
    pub fn new(
        query: SortedSetDocValuesRangeQuery,
        values: SortedSet<IRCLeafReader<IRC>>,
        score: f32,
        score_mode: ScoreMode,
    ) -> Result<Self> {
        let cost = values.cost()?;
        Ok(ScorerSupplierImpl3 {
            query,
            values: Some(values),
            cost,
            score,
            score_mode,
        })
    }
}
impl<IRC> ScorerSupplier<IRC> for ScorerSupplierImpl3<IRC>
where
    IRC: IndexReaderContext,
    IRCLeafReader<IRC>: 'static,
{
    type Scorer = QueryWeightSsScorer;
    type BulkScorer = QueryWeightSsBulkScorer;

    fn get(
        &mut self,
        _lead_cost: i64,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<Self::Scorer> {
        let mut skipper_opt = context.reader().get_doc_values_skipper(&self.query.field)?;
        let mut values = match self.values.take() {
            Some(v) => v,
            None => DocValues::get_sorted_set(context.reader(), &self.query.field)?,
        };
        let min_ord: i64 = match self.query.lower_value.as_ref() {
            None => 0,
            Some(lv) => {
                let ord = values.lookup_term(lv)?;
                if ord < 0 {
                    -1 - ord
                } else if self.query.lower_inclusive {
                    ord
                } else {
                    ord + 1
                }
            },
        };

        let max_ord: i64 = match self.query.upper_value.as_ref() {
            None => values.get_value_count()? - 1,
            Some(uv) => {
                let ord = values.lookup_term(uv)?;
                if ord < 0 {
                    -2 - ord
                } else if self.query.upper_inclusive {
                    ord
                } else {
                    ord - 1
                }
            },
        };

        // no terms matched
        if min_ord > max_ord {
            let v =
                ConstantScoreScorer::from_disi(self.score, self.score_mode, EmptyDISI::default());
            return Ok(Box::new(v));
        }

        if let Some(ref skipper) = skipper_opt
            && (min_ord > skipper.max_value() || max_ord < skipper.min_value())
        {
            let v =
                ConstantScoreScorer::from_disi(self.score, self.score_mode, EmptyDISI::default());
            return Ok(Box::new(v));
        }

        if let Some(ref skipper) = skipper_opt
            && skipper.doc_count() == context.reader().max_doc()?
            && skipper.min_value() >= min_ord
            && skipper.max_value() <= max_ord
        {
            let v = ConstantScoreScorer::from_disi(
                self.score,
                self.score_mode,
                AllDISI::new(skipper.doc_count()),
            );
            return Ok(Box::new(v));
        }
        let iterator = if values.is_single_valued() {
            let mut singleton = DocValues::unwrap_singleton_sorted(&mut values)?;
            match skipper_opt {
                Some(ref mut skipper) => {
                    let ps_iterator_opt = get_doc_id_set_iterator_or_null_for_primary_sort(
                        context.reader(),
                        &mut singleton,
                        skipper,
                        min_ord,
                        max_ord,
                        &self.query.field,
                    )?;
                    match ps_iterator_opt {
                        Some(ps_iterator) => {
                            let v = ConstantScoreScorer::from_disi(
                                self.score,
                                self.score_mode,
                                ps_iterator,
                            );
                            return Ok(Box::new(v));
                        },
                        None => TwoPhaseIteratorEnum2::A(TwoPhaseIterator5::new(
                            singleton, min_ord, max_ord,
                        )),
                    }
                },
                None => {
                    TwoPhaseIteratorEnum2::A(TwoPhaseIterator5::new(singleton, min_ord, max_ord))
                },
            }
        } else {
            TwoPhaseIteratorEnum2::B(TwoPhaseIterator6::new(values, min_ord, max_ord))
        };
        match skipper_opt {
            Some(skipper) => {
                let v = DocValuesRangeIterator::new(iterator, skipper, min_ord, max_ord, false);
                let scorer = ConstantScoreScorer::from_tpi(self.score, self.score_mode, v);
                Ok(Box::new(scorer))
            },
            None => {
                let scorer = ConstantScoreScorer::from_tpi(self.score, self.score_mode, iterator);
                Ok(Box::new(scorer))
            },
        }
    }

    fn bulk_scorer(
        &mut self,
        context: &LeafReaderContext<IRCLeafReader<IRC>>,
        searcher: &IndexSearcher<IRC>,
    ) -> Result<Option<Self::BulkScorer>> {
        Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
    }

    fn cost(
        &mut self,
        _context: &LeafReaderContext<IRCLeafReader<IRC>>,
        _searcher: &IndexSearcher<IRC>,
    ) -> Result<i64> {
        Ok(self.cost)
    }
}
pub struct TwoPhaseIterator5<S>
where
    S: SortedDocValues,
{
    singleton: S,
    min_ord: i64,
    max_ord: i64,
}

impl<S> TwoPhaseIterator5<S>
where
    S: SortedDocValues,
{
    pub fn new(singleton: S, min_ord: i64, max_ord: i64) -> Self {
        TwoPhaseIterator5 {
            singleton,
            min_ord,
            max_ord,
        }
    }
}

impl<S> TwoPhaseIterator for TwoPhaseIterator5<S>
where
    S: SortedDocValues,
{
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.singleton)
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.singleton)
    }

    fn matches(&mut self) -> Result<bool> {
        let ord = self.singleton.ord_value()? as i64;
        Ok(ord >= self.min_ord && ord <= self.max_ord)
    }

    fn match_cost(&self) -> f32 {
        2f32
    }
}
pub struct TwoPhaseIterator6<S>
where
    S: SortedSetDocValues,
{
    values: S,
    min_ord: i64,
    max_ord: i64,
}

impl<S> TwoPhaseIterator6<S>
where
    S: SortedSetDocValues,
{
    pub fn new(values: S, min_ord: i64, max_ord: i64) -> Self {
        TwoPhaseIterator6 {
            values,
            min_ord,
            max_ord,
        }
    }
}

impl<S> TwoPhaseIterator for TwoPhaseIterator6<S>
where
    S: SortedSetDocValues,
{
    fn approximation_mut(&mut self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&mut self.values)
    }

    fn approximation(&self) -> Box<dyn DocIdSetIterator + '_> {
        Box::new(&self.values)
    }

    fn matches(&mut self) -> Result<bool> {
        let count = self.values.doc_value_count()?;
        for _ in 0..count {
            let ord = self.values.next_ord()?;

            if ord < self.min_ord {
                continue;
            }
            // Values are sorted, so the first ord that is >= minOrd is our best
            // candidate
            return Ok(ord <= self.max_ord);
        }

        Ok(false) // all ords were < minOrd
    }

    fn match_cost(&self) -> f32 {
        2f32
    }
}
