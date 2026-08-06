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
use crate::core::codecs::Codec;
use crate::core::codecs::lucene90::lucene90_points_reader::Lucene90PointsReader;
use crate::core::codecs::lucene90::lucene90_points_writer::Lucene90PointsWriter;
use crate::core::codecs::points_format::PointsFormat;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::test_util::DefaultCodec;
use std::fmt::{Display, Formatter};

#[allow(dead_code)] // for quick search
struct TestLucene90PointsFormat;

#[derive(Clone)]
pub struct TestLucene90PointsFormatCodec {
  delegate: DefaultCodec,
  points_format: TestLucene90PointsFormatPointsFormat,
}

impl TestLucene90PointsFormatCodec {
  pub(crate) fn new(
    delegate: DefaultCodec,
    max_points_in_leaf_node: usize,
    max_mb_sort_in_heap: f64,
  ) -> Self {
    Self {
      delegate,
      points_format: TestLucene90PointsFormatPointsFormat {
        max_points_in_leaf_node,
        max_mb_sort_in_heap,
      },
    }
  }
}

impl Display for TestLucene90PointsFormatCodec {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.delegate, f)
  }
}

impl Codec for TestLucene90PointsFormatCodec {
  type PostingsFormat = <DefaultCodec as Codec>::PostingsFormat;
  type DocValuesFormat = <DefaultCodec as Codec>::DocValuesFormat;
  type StoredFieldsFormat = <DefaultCodec as Codec>::StoredFieldsFormat;
  type TermVectorsFormat = <DefaultCodec as Codec>::TermVectorsFormat;
  type FieldInfosFormat = <DefaultCodec as Codec>::FieldInfosFormat;
  type SegmentInfoFormat = <DefaultCodec as Codec>::SegmentInfoFormat;
  type NormsFormat = <DefaultCodec as Codec>::NormsFormat;
  type LiveDocsFormat = <DefaultCodec as Codec>::LiveDocsFormat;
  type CompoundFormat = <DefaultCodec as Codec>::CompoundFormat;
  type PointsFormat = TestLucene90PointsFormatPointsFormat;
  type KnnVectorsFormat = <DefaultCodec as Codec>::KnnVectorsFormat;

  fn postings_format(&self) -> Self::PostingsFormat {
    self.delegate.postings_format()
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    self.delegate.doc_values_format()
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    self.delegate.stored_fields_format()
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    self.delegate.term_vectors_format()
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    self.delegate.field_infos_format()
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    self.delegate.segment_info_format()
  }

  fn norms_format(&self) -> Self::NormsFormat {
    self.delegate.norms_format()
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    self.delegate.live_docs_format()
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    self.delegate.compound_format()
  }

  fn points_format(&self) -> Self::PointsFormat {
    self.points_format.clone()
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    self.delegate.knn_vectors_format()
  }

  fn get_name(&self) -> &str {
    self.delegate.get_name()
  }
}

#[derive(Clone)]
pub struct TestLucene90PointsFormatPointsFormat {
  max_points_in_leaf_node: usize,
  max_mb_sort_in_heap: f64,
}

impl PointsFormat for TestLucene90PointsFormatPointsFormat {
  type PointsWriter<T: IndexOutput> = Lucene90PointsWriter<T>;

  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    info: &SegmentInfo<D2>,
  ) -> Result<Self::PointsWriter<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    Lucene90PointsWriter::new(
      state,
      self.max_points_in_leaf_node,
      self.max_mb_sort_in_heap,
      info,
    )
  }

  type PointsReader<T: IndexInput> = Lucene90PointsReader<T>;

  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    info: &SegmentInfo<D2>,
  ) -> Result<Self::PointsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    Lucene90PointsReader::new(state, info)
  }
}
