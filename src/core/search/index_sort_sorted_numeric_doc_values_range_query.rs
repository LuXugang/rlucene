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
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::point_values::{IntersectVisitor, PointTree, Relation};
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::doc_id_set_iterator::{
    AllDISI, DocIdSetIterator, Either4DocIdSetIterator, EmptyDISI, RangeDISI,
};
use crate::core::search::dummy::dummy_scorer::DummyScorer;
use crate::core::search::field_comparator::{FieldComparator, FieldComparatorEnum};
use crate::core::search::leaf_field_comparator::{LeafFieldComparator, LeafFieldComparatorEnum};
use crate::core::search::pruning::Pruning;
use crate::core::search::sort_field::{SortFieldType, SortFiledBase};
use crate::core::search::sort_field_enum::SortFieldEnum;
use crate::core::util::array_util::{ByteArrayComparator, ByteArrayComparatorEnum};
use crate::core::util::error::lucene_error::{LuceneError, Result};
pub struct IndexSortSortedNumericDocValuesRangeQuery {}

struct ValueAndDoc {
    value: Option<Vec<u8>>,
    doc_id: i32,
    done: bool,
}
impl ValueAndDoc {
    pub fn new() -> Self {
        Self {
            value: None,
            doc_id: 0,
            done: false,
        }
    }
}
fn find_next_value<P>(
    point_tree: &mut P,
    value: &[u8],
    allow_equal: bool,
    comparator: &ByteArrayComparatorEnum,
    last_doc: bool,
) -> Result<Option<ValueAndDoc>>
where
    P: PointTree,
{
    let cmp = comparator.compare(point_tree.get_max_packed_value()?, 0, value, 0);

    if cmp < 0 || (cmp == 0 && !allow_equal) {
        return Ok(None);
    }

    if !point_tree.move_to_child()? {
        let mut vd = ValueAndDoc::new();
        let mut visitor =
            IntersectVisitorImpl::new(&mut vd, comparator, value, last_doc, allow_equal);
        point_tree.visit_doc_values(&mut visitor)?;

        if vd.value.is_some() {
            return Ok(Some(vd));
        } else {
            return Ok(None);
        }
    }
    loop {
        if let Some(vd) = find_next_value(point_tree, value, allow_equal, comparator, last_doc)? {
            return Ok(Some(vd));
        }

        if !point_tree.move_to_sibling()? {
            break;
        }
    }

    let moved = point_tree.move_to_parent()?;
    debug_assert!(moved);
    Ok(None)
}
fn next_doc<P>(
    point_tree: &mut P,
    value: &[u8],
    allow_equal: bool,
    comparator: &ByteArrayComparatorEnum,
    last_doc_flag: bool,
) -> Result<i32>
where
    P: PointTree,
{
    let vd_opt = find_next_value(point_tree, value, allow_equal, comparator, last_doc_flag)?;

    let vd = match vd_opt {
        Some(v) => v,
        None => return Ok(-1),
    };

    if !last_doc_flag || vd.done {
        return Ok(vd.doc_id);
    }

    // We found the next value, now we need the last doc ID.
    let doc = last_doc(point_tree, vd.value.as_ref().unwrap(), comparator)?;

    if doc == -1 {
        // vd.docID was actually the last doc ID
        Ok(vd.doc_id)
    } else {
        Ok(doc)
    }
}
fn last_doc<P>(
    point_tree: &mut P,
    value: &[u8],
    comparator: &ByteArrayComparatorEnum,
) -> Result<i32>
where
    P: PointTree,
{
    // Create a stack of nodes that may contain value that we'll use to search for the last leaf
    // node that contains `value`.
    // While the logic looks a bit complicated due to the fact that the PointTree API doesn't allow
    // moving back to previous siblings, this effectively performs a binary search.
    let mut stack: Vec<P> = Vec::new();

    // outer:
    loop {
        // Move to the next node
        loop {
            if point_tree.move_to_sibling()? {
                break;
            }
            if !point_tree.move_to_parent()? {
                // No next node
                break;
            }
        }

        let cmp = comparator.compare(point_tree.get_min_packed_value()?, 0, value, 0);
        if cmp > 0 {
            // This node doesn't have the value → next nodes also can't
            break;
        }

        // Push clone
        stack.push(point_tree.clone());
    }

    // Now search stack nodes
    while let Some(mut next) = stack.pop() {
        if !next.move_to_child()? {
            let mut visitor = IntersectVisitorImpl1::new(value, comparator);
            next.visit_doc_values(&mut visitor)?;

            if visitor.last_doc != -1 {
                return Ok(visitor.last_doc);
            }
        } else {
            loop {
                let cmp = comparator.compare(next.get_min_packed_value()?, 0, value, 0);
                if cmp > 0 {
                    // This node doesn't have `value`, so next nodes can't either
                    break;
                }

                stack.push(next.clone());

                if !next.move_to_sibling()? {
                    break;
                }
            }
        }
    }

    Ok(-1)
}

struct IntersectVisitorImpl1<'a> {
    value: &'a [u8],
    comparator: &'a ByteArrayComparatorEnum,
    last_doc: i32,
}
impl<'a> IntersectVisitorImpl1<'a> {
    pub fn new(value: &'a [u8], comparator: &'a ByteArrayComparatorEnum) -> Self {
        Self {
            value,
            comparator,
            last_doc: -1,
        }
    }
}
impl<'a> IntersectVisitor for IntersectVisitorImpl1<'a> {
    fn visit(&mut self, _doc_id: i32) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        let cmp = self.comparator.compare(self.value, 0, packed_value, 0);
        if cmp == 0 {
            self.last_doc = doc_id;
        }
        Ok(())
    }

    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        Ok(Relation::CellCrossesQuery)
    }
}

struct IntersectVisitorImpl<'a> {
    vd: &'a mut ValueAndDoc,
    comparator: &'a ByteArrayComparatorEnum,
    value: &'a [u8],
    last_doc: bool,
    allow_equal: bool,
}
impl<'a> IntersectVisitorImpl<'a> {
    pub fn new(
        vd: &'a mut ValueAndDoc,
        comparator: &'a ByteArrayComparatorEnum,
        value: &'a [u8],
        last_doc: bool,
        allow_equal: bool,
    ) -> Self {
        Self {
            vd,
            comparator,
            value,
            last_doc,
            allow_equal,
        }
    }
}
impl<'a> IntersectVisitor for IntersectVisitorImpl<'a> {
    fn visit(&mut self, _doc_id: i32) -> Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        match self.vd.value {
            Some(ref value) if self.last_doc && !self.vd.done => {
                let cmp = self.comparator.compare(packed_value, 0, value, 0);
                debug_assert!(cmp >= 0);
                if cmp > 0 {
                    self.vd.done = true;
                } else {
                    self.vd.doc_id = doc_id;
                }
            },
            None => {
                let cmp = self.comparator.compare(packed_value, 0, self.value, 0);

                if cmp > 0 || (cmp == 0 && self.allow_equal) {
                    self.vd.value = Some(packed_value.to_vec());
                    self.vd.doc_id = doc_id;
                }
            },
            _ => {},
        }
        Ok(())
    }

    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        Ok(Relation::CellCrossesQuery)
    }
}
struct BoundedDocIdSetIterator<D>
where
    D: DocIdSetIterator,
{
    first_doc: i32,
    last_doc: i32,
    delegate: D,
    doc_id: i32,
}

impl<D> BoundedDocIdSetIterator<D>
where
    D: DocIdSetIterator,
{
    pub fn new(first_doc: i32, last_doc: i32, delegate: D) -> Self {
        Self {
            first_doc,
            last_doc,
            delegate,
            doc_id: -1,
        }
    }
}

impl<D> DocIdSetIterator for BoundedDocIdSetIterator<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.doc_id
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc_id + 1)
    }

    fn advance(&mut self, mut target: i32) -> Result<i32> {
        if target < self.first_doc {
            target = self.first_doc;
        }

        let result = self.delegate.advance(target)?;

        if result < self.last_doc {
            self.doc_id = result;
        } else {
            self.doc_id = NO_MORE_DOCS;
        }

        Ok(self.doc_id)
    }

    fn cost(&self) -> Result<i64> {
        let delegate_cost = self.delegate.cost()?;
        let bound_cost = (self.last_doc - self.first_doc) as i64;
        Ok(delegate_cost.min(bound_cost))
    }
}

trait ValueComparator {
    fn compare(&mut self, doc_id: i32) -> Result<i32>;
}
struct ValueComparatorImpl<LR>
where
    LR: LeafReader,
{
    field_comparator: FieldComparatorEnum,
    leaf_field_comparator: LeafFieldComparatorEnum<LR>,
    direction: i32,
}
impl<LR> ValueComparatorImpl<LR>
where
    LR: LeafReader,
{
    pub fn new(
        mut field_comparator: FieldComparatorEnum,
        direction: i32,
        context: &LeafReaderContext<LR>,
    ) -> Result<Self> {
        let leaf_field_comparator = field_comparator.get_leaf_comparator(context)?;
        Ok(Self {
            field_comparator,
            leaf_field_comparator,
            direction,
        })
    }
}
impl<LR> ValueComparator for ValueComparatorImpl<LR>
where
    LR: LeafReader,
{
    fn compare(&mut self, doc_id: i32) -> Result<i32> {
        let mut v = DummyScorer;
        let value =
            self.leaf_field_comparator
                .compare_top(doc_id, &mut v, &mut self.field_comparator)?;
        Ok(self.direction * value)
    }
}
fn load_comparator<LR>(
    sort_field: &mut SortFieldEnum,
    top_value: i64,
    context: &LeafReaderContext<LR>,
) -> Result<ValueComparatorImpl<LR>>
where
    LR: LeafReader,
{
    let mut field_comparator = sort_field.get_comparator(1, Pruning::None)?;

    field_comparator.set_top_value(top_value.into());

    let direction = if sort_field.get_reverse() { -1 } else { 1 };

    ValueComparatorImpl::new(field_comparator, direction, context)
}

fn get_sort_field_type(sort_field: &SortFieldEnum) -> SortFieldType {
    // We expect the sortField to be SortedNumericSortField
    match sort_field {
        SortFieldEnum::SortedNumeric(sf) => sf.get_numeric_type(),
        _ => sort_field.get_type(),
    }
}

struct IteratorAndCount<D>
where
    D: DocIdSetIterator,
{
    it: DISI<D>,
    count: i32,
}

impl<D> IteratorAndCount<D>
where
    D: DocIdSetIterator,
{
    fn new(it: DISI<D>, count: i32) -> Self {
        Self { it, count }
    }

    fn empty() -> Self {
        IteratorAndCount::new(Either4DocIdSetIterator::A(EmptyDISI::default()), 0)
    }

    fn all(max_doc: i32) -> Self {
        IteratorAndCount::new(Either4DocIdSetIterator::B(AllDISI::new(max_doc)), max_doc)
    }

    fn dense_range(min_doc: i32, max_doc: i32) -> Result<Self> {
        Ok(IteratorAndCount::new(
            Either4DocIdSetIterator::C(RangeDISI::new(min_doc, max_doc)?),
            max_doc - min_doc,
        ))
    }

    fn sparse_range(min_doc: i32, max_doc: i32, delegate: D) -> IteratorAndCount<D> {
        let v = BoundedDocIdSetIterator::new(min_doc, max_doc, delegate);
        IteratorAndCount::new(Either4DocIdSetIterator::D(v), -1)
    }
}

pub type DISI<D> =
    Either4DocIdSetIterator<EmptyDISI, AllDISI, RangeDISI, BoundedDocIdSetIterator<D>>;
