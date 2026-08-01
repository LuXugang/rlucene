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
use crate::core::codecs::dummy::dummy_mutable_point_tree::DummyMutablePointTree;
use crate::core::codecs::lucene90_points_writer::Lucene90PointsWriter;
use crate::core::codecs::points_reader::PointsReader;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::merge_state::{DocMap, MergeState, MergeStateDocMap};
use crate::core::index::point_values::Relation::CellCrossesQuery;
use crate::core::index::point_values::{
  IntersectVisitor, PointTree, PointTreeEnum, PointValues, Relation,
};
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::directory::Directory;
use crate::core::util::clone::TryClone;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

/// Write points
pub trait PointsWriter: Closeable {
  /// Write all values contained in the provided reader
  fn write_field<PR, D1, D2>(
    &mut self,
    field_info: &Arc<FieldInfo>,
    values: &mut PR,
    dir: &D1,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<()>
  where
    PR: PointsReader,
    D1: Directory,
    D2: Directory;

  /// Called once at the end before close
  fn finish(&mut self) -> Result<()>;

  fn merge_one_field<D1, D2, CR>(
    &mut self,
    merge_state: &MergeState<D1, CR>,
    field_info: &Arc<FieldInfo>,
    dir: &D2,
  ) -> Result<()>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    let mut max_point_count = 0;
    let mut point_values = Vec::with_capacity(merge_state.points_readers.len());
    let mut doc_maps = Vec::with_capacity(merge_state.points_readers.len());
    for (i, points_reader_opt) in merge_state.points_readers.iter().enumerate() {
      let points_reader = match points_reader_opt.as_ref() {
        Some(v) => v,
        None => continue,
      };

      let reader_field_info =
        match merge_state.field_infos[i].field_info_by_name(&field_info.name)? {
          Some(v) => v,
          None => continue,
        };

      if reader_field_info.get_point_dimension_count() == 0 {
        continue;
      }

      let values = match points_reader.get_values(&field_info.name)? {
        Some(v) => v,
        None => continue,
      };

      max_point_count += values.size()?;
      point_values.push(values);
      doc_maps.push(merge_state.doc_maps[i].clone())
    }
    let mut points_reader: PointsReaderImpl<_, MergeStateDocMap<CR>> =
      PointsReaderImpl::new(field_info.clone(), max_point_count, point_values, doc_maps);
    self.write_field(
      field_info,
      &mut points_reader,
      dir,
      merge_state.segment_info,
    )?;
    Ok(())
  }
  /// Default merge implementation to merge incoming points readers by visiting all their points and adding to this writer
  fn merge<D1, D2, CR>(&mut self, merge_state: &MergeState<D1, CR>, dir: &D2) -> Result<()>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    // check each incoming reader
    for reader in merge_state.points_readers.iter().flatten() {
      reader.check_integrity()?;
    }
    // merge field at a time
    for field_info in merge_state.merge_field_infos.iter() {
      if field_info.get_point_dimension_count() != 0 {
        self.merge_one_field(merge_state, field_info, dir)?;
      }
    }
    self.finish()
  }
}
pub type PointsWriterType<O> = Lucene90PointsWriter<O>;

pub enum PointsWriterEnum2<A, B> {
  A(A),
  B(B),
}

impl<A, B> Closeable for PointsWriterEnum2<A, B>
where
  A: PointsWriter,
  B: PointsWriter,
{
  fn close(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.close(),
      Self::B(inner) => inner.close(),
    }
  }
}

impl<A, B> PointsWriter for PointsWriterEnum2<A, B>
where
  A: PointsWriter,
  B: PointsWriter,
{
  fn write_field<PR, D1, D2>(
    &mut self,
    field_info: &Arc<FieldInfo>,
    values: &mut PR,
    dir: &D1,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<()>
  where
    PR: PointsReader,
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::A(inner) => inner.write_field(field_info, values, dir, segment_info),
      Self::B(inner) => inner.write_field(field_info, values, dir, segment_info),
    }
  }

  fn finish(&mut self) -> Result<()> {
    match self {
      Self::A(inner) => inner.finish(),
      Self::B(inner) => inner.finish(),
    }
  }

  fn merge_one_field<D1, D2, CR>(
    &mut self,
    merge_state: &MergeState<D1, CR>,
    field_info: &Arc<FieldInfo>,
    dir: &D2,
  ) -> Result<()>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    match self {
      Self::A(inner) => inner.merge_one_field(merge_state, field_info, dir),
      Self::B(inner) => inner.merge_one_field(merge_state, field_info, dir),
    }
  }

  fn merge<D1, D2, CR>(&mut self, merge_state: &MergeState<D1, CR>, dir: &D2) -> Result<()>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    match self {
      Self::A(inner) => inner.merge(merge_state, dir),
      Self::B(inner) => inner.merge(merge_state, dir),
    }
  }
}

struct PointsReaderImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  field_info: Arc<FieldInfo>,
  final_max_point_count: usize,
  point_value: Rc<Vec<P>>,
  doc_map: Vec<Rc<DM>>,
}

impl<P, DM> CloseableRef for PointsReaderImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
}

impl<P, DM> PointsReaderImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn new(
    field_info: Arc<FieldInfo>,
    final_max_point_count: usize,
    point_value: Vec<P>,
    doc_map: Vec<Rc<DM>>,
  ) -> Self {
    Self {
      field_info,
      final_max_point_count,
      point_value: Rc::new(point_value),
      doc_map,
    }
  }
}

impl<P, DM> PointsReader for PointsReaderImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn check_integrity(&self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  type PointValuesType = PointValuesImpl<P, DM>;

  fn get_values(&self, field_name: &str) -> Result<Option<Self::PointValuesType>> {
    if field_name != self.field_info.name {
      return Err(LuceneError::illegal_argument(
        "field name must match the field being merged",
      ));
    }
    Ok(Some(PointValuesImpl::new(
      self.final_max_point_count,
      self.point_value.clone(),
      self.doc_map.clone(),
    )))
  }
}

struct PointValuesImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  final_max_point_count: usize,
  point_value: Rc<Vec<P>>,
  doc_map: Vec<Rc<DM>>,
}
impl<P, DM> PointValuesImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn new(final_max_point_count: usize, point_value: Rc<Vec<P>>, doc_map: Vec<Rc<DM>>) -> Self {
    Self {
      final_max_point_count,
      point_value,
      doc_map,
    }
  }
}

impl<P, DM> PointValues for PointValuesImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn get_min_packed_value(&self) -> Result<Option<Cow<'_, [u8]>>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_max_packed_value(&self) -> Result<Option<Cow<'_, [u8]>>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_num_dimensions(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_num_index_dimensions(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_bytes_per_dimension(&self) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn size(&self) -> Result<usize> {
    Ok(self.final_max_point_count)
  }

  fn get_doc_count(&self) -> Result<i32> {
    Err(LuceneError::unsupported_operation(""))
  }

  type PointTree = PointTreeImpl<P, DM>;
  type MutablePointTree = DummyMutablePointTree;

  fn get_point_tree(&self) -> Result<PointTreeEnum<Self::MutablePointTree, Self::PointTree>> {
    Ok(PointTreeEnum::Other(PointTreeImpl::new(
      self.final_max_point_count,
      self.doc_map.clone(),
      self.point_value.clone(),
    )))
  }
}

struct PointTreeImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  final_max_point_count: usize,
  doc_map: Vec<Rc<DM>>,
  point_value: Rc<Vec<P>>,
}
impl<P, DM> PointTreeImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn new(final_max_point_count: usize, doc_map: Vec<Rc<DM>>, point_value: Rc<Vec<P>>) -> Self {
    Self {
      final_max_point_count,
      doc_map,
      point_value,
    }
  }
}

impl<P, DM> TryClone for PointTreeImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn try_clone(&self) -> Result<Self>
  where
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<P, DM> PointTree for PointTreeImpl<P, DM>
where
  P: PointValues,
  DM: DocMap,
{
  fn move_to_child(&mut self) -> Result<bool> {
    Ok(false)
  }

  fn move_to_sibling(&mut self) -> Result<bool> {
    Ok(false)
  }

  fn move_to_parent(&mut self) -> Result<bool> {
    Ok(false)
  }

  fn get_min_packed_value(&self) -> Result<Cow<'_, [u8]>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn get_max_packed_value(&self) -> Result<Cow<'_, [u8]>> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn size(&self) -> Result<usize> {
    Ok(self.final_max_point_count)
  }

  fn visit_doc_ids<IV>(&mut self, _visitor: &mut IV) -> Result<()>
  where
    IV: IntersectVisitor,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn visit_doc_values<IV>(&mut self, visitor: &mut IV) -> Result<()>
  where
    IV: IntersectVisitor,
  {
    for (i, values) in self.point_value.iter().enumerate() {
      let mut v: IntersectVisitorImpl<'_, _, DM> =
        IntersectVisitorImpl::new(self.doc_map[i].as_ref(), visitor);
      values.get_point_tree()?.visit_doc_values(&mut v)?;
    }
    Ok(())
  }
}

struct IntersectVisitorImpl<'a, I, DM>
where
  I: IntersectVisitor,
  DM: DocMap,
{
  doc_map: &'a DM,
  merged_visitor: &'a mut I,
}
impl<'a, I, DM> IntersectVisitorImpl<'a, I, DM>
where
  I: IntersectVisitor,
  DM: DocMap,
{
  fn new(doc_map: &'a DM, merged_visitor: &'a mut I) -> Self {
    Self {
      doc_map,
      merged_visitor,
    }
  }
}
impl<'a, I, DM> IntersectVisitor for IntersectVisitorImpl<'a, I, DM>
where
  I: IntersectVisitor,
  DM: DocMap,
{
  fn visit(&mut self, _doc_id: i32) -> Result<()> {
    Err(LuceneError::illegal_state(""))
  }

  fn visit_with_packed_value(&mut self, doc_id: i32, packed_value: &[u8]) -> Result<()> {
    let new_doc_id = self.doc_map.get(doc_id)?;
    if new_doc_id != -1 {
      self
        .merged_visitor
        .visit_with_packed_value(new_doc_id, packed_value)?;
    }
    Ok(())
  }

  fn compare(&self, _min_packed_value: &[u8], _max_packed_value: &[u8]) -> Result<Relation> {
    Ok(CellCrossesQuery)
  }
}
