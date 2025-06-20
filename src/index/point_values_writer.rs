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
use crate::codecs::mutable_point_tree::MutablePointTree;
use crate::index::point_values::{IntersectVisitor, PointTree, Relation};
use crate::index::sorter::DocMap;
use crate::index::BytesRef;
use crate::util::error::lucene_error::Result;
use crate::util::paged_bytes::PagedBytesReader;
use crate::util::SliceCopyOps;
use std::rc::Rc;

pub(crate) struct PointValuesWriter;
pub(crate) struct MutableSortingPointValues<M, DM>
where
    M: MutablePointTree,
    DM: DocMap,
{
    input: M,
    doc_map: Rc<DM>,
}
impl<M, DM> MutableSortingPointValues<M, DM>
where
    M: MutablePointTree,
    DM: DocMap,
{
    pub(crate) fn new(input: M, doc_map: Rc<DM>) -> Self {
        Self { input, doc_map }
    }
}

impl<M, DM> MutablePointTree for MutableSortingPointValues<M, DM>
where
    M: MutablePointTree,
    DM: DocMap,
{
    fn get_value(&self, i: usize, packed_value: &mut BytesRef<Vec<u8>>) {
        self.input.get_value(i, packed_value)
    }

    fn get_byte_at(&self, i: usize, k: usize) -> u8 {
        self.input.get_byte_at(i, k)
    }

    fn get_doc_id(&self, i: usize) -> i32 {
        self.doc_map.old_to_new(self.input.get_doc_id(i))
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.input.swap(i, j);
    }

    fn save(&mut self, i: usize, j: usize) {
        self.input.save(i, j)
    }

    fn restore(&mut self, i: usize, j: usize) {
        self.input.restore(i, j)
    }
}

impl<M, DM> Clone for MutableSortingPointValues<M, DM>
where
    M: MutablePointTree,
    DM: DocMap,
{
    fn clone(&self) -> Self {
        Self {
            input: self.input.clone(),
            doc_map: self.doc_map.clone(),
        }
    }
}

impl<M, DM> PointTree for MutableSortingPointValues<M, DM>
where
    M: MutablePointTree,
    DM: DocMap,
{
    fn size(&self) -> Result<i64> {
        self.input.size()
    }

    fn visit_doc_values<IV>(&mut self, visitor: &mut IV) -> Result<()>
    where
        IV: IntersectVisitor,
    {
        let mut intersect_visitor = IntersectVisitorImpl::new(visitor, self.doc_map.clone());
        self.input.visit_doc_values(&mut intersect_visitor)
    }
}

struct IntersectVisitorImpl<'a, IV, DM>
where
    IV: IntersectVisitor,
    DM: DocMap,
{
    visitor: &'a mut IV,
    doc_map: Rc<DM>,
}
impl<'a, IV, DM> IntersectVisitorImpl<'a, IV, DM>
where
    IV: IntersectVisitor,
    DM: DocMap,
{
    pub(crate) fn new(visitor: &'a mut IV, doc_map: Rc<DM>) -> Self {
        Self { visitor, doc_map }
    }
}
impl<'a, IV, DM> IntersectVisitor for IntersectVisitorImpl<'a, IV, DM>
where
    IV: IntersectVisitor,
    DM: DocMap,
{
    fn visit(&mut self, doc_id: i32) -> Result<()> {
        self.visitor.visit(self.doc_map.old_to_new(doc_id))
    }

    fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
        self.visitor
            .visit_with_packed_value(self.doc_map.old_to_new(doc_id), packed_value)
    }

    fn compare(&mut self, min_packed_value: &[u8], max_packed_value: &[u8]) -> Result<Relation> {
        self.visitor.compare(min_packed_value, max_packed_value)
    }
}

struct MutablePointTreeImpl {
    num_points: usize,
    ords: Vec<i32>,
    temp: Vec<i32>,
    doc_ids: Rc<Vec<i32>>,
    packed_bytes_length: usize,
    bytes_reader: PagedBytesReader,
}
impl MutablePointTreeImpl {
    pub(crate) fn new(
        num_points: usize,
        doc_ids: Rc<Vec<i32>>,
        bytes_reader: PagedBytesReader,
        packed_bytes_length: usize,
    ) -> Self {
        let mut ords: Vec<i32> = vec![0; num_points];
        for i in 0..num_points {
            ords[i] = i as i32;
        }
        let temp: Vec<i32> = vec![0; num_points];
        Self {
            num_points,
            ords,
            temp,
            doc_ids,
            packed_bytes_length,
            bytes_reader,
        }
    }
}

impl PointTree for MutablePointTreeImpl {
    fn size(&self) -> Result<i64> {
        Ok(self.num_points as i64)
    }

    fn visit_doc_values<IV>(&mut self, visitor: &mut IV) -> Result<()>
    where
        IV: IntersectVisitor,
    {
        let mut scratch = BytesRef::new();
        let mut packed_value = vec![0u8; self.packed_bytes_length];
        for i in 0..self.num_points {
            self.get_value(i, &mut scratch);
            debug_assert_eq!(scratch.length, self.packed_bytes_length);
            packed_value.copy_from(
                &scratch.bytes[scratch.offset..scratch.offset + self.packed_bytes_length],
                0,
            );
            let doc_id = self.get_doc_id(i);
            visitor.visit_with_packed_value(doc_id, &packed_value)?;
        }
        Ok(())
    }
}

impl Clone for MutablePointTreeImpl {
    fn clone(&self) -> Self {
        let ords = self.ords.clone();
        let temp = self.temp.clone();
        let doc_ids = Rc::clone(&self.doc_ids);
        let bytes_reader = self.bytes_reader.clone();
        Self {
            num_points: self.num_points,
            ords,
            temp,
            doc_ids,
            packed_bytes_length: self.packed_bytes_length,
            bytes_reader,
        }
    }
}

impl MutablePointTree for MutablePointTreeImpl {
    fn get_value(&self, i: usize, packed_value: &mut BytesRef<Vec<u8>>) {
        let offset = self.packed_bytes_length * self.ords[i] as usize;
        // self.bytes_reader
        //     .fill_slice(packed_value, offset, self.packed_bytes_length);
    }

    fn get_byte_at(&self, i: usize, k: usize) -> u8 {
        let offset = self.packed_bytes_length * self.ords[i] as usize + k;
        self.bytes_reader.get_byte(offset)
    }

    fn get_doc_id(&self, i: usize) -> i32 {
        self.doc_ids[self.ords[i] as usize]
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.ords.swap(i, j);
    }

    fn save(&mut self, i: usize, j: usize) {
        self.temp[j] = self.ords[i];
    }

    fn restore(&mut self, i: usize, j: usize) {
        self.ords.copy_from(&self.temp[i..j], i);
    }
}
