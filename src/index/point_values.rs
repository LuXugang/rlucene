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
use crate::util::bkd::bkd_config::BKDConfig;

pub struct PointValues;
impl PointValues {
    pub const MAX_NUM_BYTES: i32 = 16;
    pub const MAX_DIMENSIONS: i32 = BKDConfig::MAX_DIMS;
    pub const MAX_INDEX_DIMENSIONS: i32 = BKDConfig::MAX_INDEX_DIMS;
}
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::util::error::lucene_error::LuceneError;
use crate::util::ints_ref::IntsRef;
/// Used by `intersect` to check how each recursive cell corresponds to the query.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Relation {
    /// Return this if the cell is fully contained by the query.
    CellInsideQuery,
    /// Return this if the cell and query do not overlap.
    CellOutsideQuery,
    /// Return this if the cell partially overlaps the query.
    CellCrossesQuery,
}
/// Basic operations to read the KD-tree.
pub trait PointTree: Clone {
    /// Clone, the current node becomes the root of the new tree.
    fn clone_tree(&self) -> Result<Self, LuceneError> {
        Err(LuceneError::need_implemented(
            "clone_tree is not implemented",
        ))
    }

    /// Move to the first child node and return `true` upon success.
    /// Returns `false` for leaf nodes and `true` otherwise.
    fn move_to_child(&mut self) -> Result<bool, LuceneError> {
        Err(LuceneError::need_implemented(
            "move_to_child is not implemented",
        ))
    }

    /// Move to the next sibling node and return `true` upon success.
    /// Returns `false` if the current node has no more siblings.
    fn move_to_sibling(&mut self) -> Result<bool, LuceneError> {
        Err(LuceneError::need_implemented(
            "move_to_sibling is not implemented",
        ))
    }

    /// Move to the parent node and return `true` upon success.
    /// Returns `false` for the root node and `true` otherwise.
    fn move_to_parent(&mut self) -> Result<bool, LuceneError> {
        Err(LuceneError::need_implemented(
            "move_to_parent is not implemented",
        ))
    }

    /// Return the minimum packed value of the current node.
    fn get_min_packed_value(&self) -> Result<&[u8], LuceneError> {
        Err(LuceneError::need_implemented(
            "get_min_packed_value is not implemented",
        ))
    }

    /// Return the maximum packed value of the current node.
    fn get_max_packed_value(&self) -> Result<&[u8], LuceneError> {
        Err(LuceneError::need_implemented(
            "get_max_packed_value is not implemented",
        ))
    }

    /// Return the number of points below the current node.
    fn size(&self) -> Result<i64, LuceneError> {
        Err(LuceneError::need_implemented("size is not implemented"))
    }

    /// Visit all the docs below the current node.
    fn visit_doc_ids(&self, visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        Err(LuceneError::need_implemented(
            "visit_doc_ids is not implemented",
        ))
    }

    /// Visit all the docs and values below the current node.
    fn visit_doc_values(&self, visitor: &mut impl IntersectVisitor) -> Result<(), LuceneError> {
        Err(LuceneError::need_implemented(
            "visit_doc_values is not implemented",
        ))
    }
}
/// We recurse the [PointTree], using a provided instance of this to guide the recursion.
pub trait IntersectVisitor {
    /// Called for all documents in a leaf cell that's fully contained by the query.
    /// The consumer should blindly accept the docID.
    fn visit(&mut self, doc_id: i32) -> Result<(), LuceneError>;

    /// Similar to `visit(doc_id)`, but a bulk visit and implementations may have their optimizations.
    /// Default implementation that iterates over the provided `DocIdSetIterator`.
    fn visit_with_iterator(
        &mut self,
        iterator: &mut impl DocIdSetIterator,
    ) -> Result<(), LuceneError> {
        loop {
            let doc_id = iterator.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            self.visit(doc_id)?;
        }
        Ok(())
    }

    /// Similar to `visit(doc_id)`, but a bulk visit and implementations may have their optimizations.
    /// Even if the implementation does the same thing as this method, this may be a speed improvement
    /// due to fewer virtual calls.
    fn visit_with_ints_ref(&mut self, ints_ref: &IntsRef) -> Result<(), LuceneError> {
        let ints = ints_ref.ints.borrow();
        for i in ints_ref.offset as usize..(ints_ref.offset + ints_ref.length) as usize {
            self.visit(ints[i])?;
        }
        Ok(())
    }

    /// Called for all documents in a leaf cell that crosses the query.
    /// The consumer should scrutinize the `packed_value` to decide whether to accept it.
    /// In the 1D case, values are visited in increasing order, and in the case of ties,
    /// in increasing docID order.
    fn visit_with_packed_value(
        &mut self,
        doc_id: i32,
        packed_value: &[u8],
    ) -> Result<(), LuceneError>;

    /// Similar to `visit_with_packed_value(doc_id, packed_value)` but in this case the `packed_value`
    /// can have more than one docID associated to it.
    /// The provided iterator should not escape the scope of this method so that implementations of PointValues
    /// are free to reuse it.
    fn visit_iterator_with_packed_value(
        &mut self,
        iterator: &mut impl DocIdSetIterator,
        packed_value: &[u8],
    ) -> Result<(), LuceneError> {
        loop {
            let doc_id = iterator.next_doc()?;
            if doc_id == NO_MORE_DOCS {
                break;
            }
            self.visit_with_packed_value(doc_id, packed_value)?;
        }
        Ok(())
    }

    /// Called for non-leaf cells to test how the cell relates to the query,
    /// to determine how to further recurse down the tree.
    fn compare(&self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Relation;

    /// Notifies the caller that this many documents are about to be visited.
    fn grow(&mut self, _count: usize) {}
}
