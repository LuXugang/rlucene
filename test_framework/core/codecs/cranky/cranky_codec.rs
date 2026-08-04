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
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::codecs::cranky::cranky_compound_format::CrankyCompoundFormat;
use crate::test_framework::core::codecs::cranky::cranky_doc_values_format::CrankyDocValuesFormat;
use crate::test_framework::core::codecs::cranky::cranky_field_infos_format::CrankyFieldInfosFormat;
use crate::test_framework::core::codecs::cranky::cranky_live_docs_format::CrankyLiveDocsFormat;
use crate::test_framework::core::codecs::cranky::cranky_norms_format::CrankyNormsFormat;
use crate::test_framework::core::codecs::cranky::cranky_points_format::CrankyPointsFormat;
use crate::test_framework::core::codecs::cranky::cranky_postings_format::CrankyPostingsFormat;
use crate::test_framework::core::codecs::cranky::cranky_segment_info_format::CrankySegmentInfoFormat;
use crate::test_framework::core::codecs::cranky::cranky_stored_fields_format::CrankyStoredFieldsFormat;
use crate::test_framework::core::codecs::cranky::cranky_term_vectors_format::CrankyTermVectorsFormat;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Codec for testing that throws random IOExceptions.
pub struct CrankyCodec<C> {
  delegate: C,
  random: Arc<Mutex<StdRng>>,
}

impl<C> CrankyCodec<C> {
  /// Wrap the provided codec with crankiness. Try passing Asserting for the most fun.
  pub fn new(delegate: C, random: StdRng) -> Self {
    // We impersonate the passed-in codec, so we don't need to be in SPI,
    // and so we don't change file formats.
    Self {
      delegate,
      random: Arc::new(Mutex::new(random)),
    }
  }
}

impl<C> Clone for CrankyCodec<C>
where
  C: Clone,
{
  fn clone(&self) -> Self {
    Self {
      delegate: self.delegate.clone(),
      random: Arc::clone(&self.random),
    }
  }
}

impl<C> Display for CrankyCodec<C>
where
  C: Display,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Cranky({})", self.delegate)
  }
}

impl<C> Codec for CrankyCodec<C>
where
  C: Codec,
{
  type PostingsFormat = CrankyPostingsFormat<C::PostingsFormat>;
  type DocValuesFormat = CrankyDocValuesFormat<C::DocValuesFormat>;
  type StoredFieldsFormat = CrankyStoredFieldsFormat<C::StoredFieldsFormat>;
  type TermVectorsFormat = CrankyTermVectorsFormat<C::TermVectorsFormat>;
  type FieldInfosFormat = CrankyFieldInfosFormat<C::FieldInfosFormat>;
  type SegmentInfoFormat = CrankySegmentInfoFormat<C::SegmentInfoFormat>;
  type NormsFormat = CrankyNormsFormat<C::NormsFormat>;
  type LiveDocsFormat = CrankyLiveDocsFormat<C::LiveDocsFormat>;
  type CompoundFormat = CrankyCompoundFormat<C::CompoundFormat>;
  type PointsFormat = CrankyPointsFormat<C::PointsFormat>;
  type KnnVectorsFormat = C::KnnVectorsFormat;

  fn postings_format(&self) -> Self::PostingsFormat {
    CrankyPostingsFormat::new(self.delegate.postings_format(), Arc::clone(&self.random))
  }

  fn doc_values_format(&self) -> Self::DocValuesFormat {
    CrankyDocValuesFormat::new(self.delegate.doc_values_format(), Arc::clone(&self.random))
  }

  fn stored_fields_format(&self) -> Self::StoredFieldsFormat {
    CrankyStoredFieldsFormat::new(
      self.delegate.stored_fields_format(),
      Arc::clone(&self.random),
    )
  }

  fn term_vectors_format(&self) -> Self::TermVectorsFormat {
    CrankyTermVectorsFormat::new(
      self.delegate.term_vectors_format(),
      Arc::clone(&self.random),
    )
  }

  fn field_infos_format(&self) -> Self::FieldInfosFormat {
    CrankyFieldInfosFormat::new(self.delegate.field_infos_format(), Arc::clone(&self.random))
  }

  fn segment_info_format(&self) -> Self::SegmentInfoFormat {
    CrankySegmentInfoFormat::new(
      self.delegate.segment_info_format(),
      Arc::clone(&self.random),
    )
  }

  fn norms_format(&self) -> Self::NormsFormat {
    CrankyNormsFormat::new(self.delegate.norms_format(), Arc::clone(&self.random))
  }

  fn live_docs_format(&self) -> Self::LiveDocsFormat {
    CrankyLiveDocsFormat::new(self.delegate.live_docs_format(), Arc::clone(&self.random))
  }

  fn compound_format(&self) -> Self::CompoundFormat {
    CrankyCompoundFormat::new(self.delegate.compound_format(), Arc::clone(&self.random))
  }

  fn points_format(&self) -> Self::PointsFormat {
    CrankyPointsFormat::new(self.delegate.points_format(), Arc::clone(&self.random))
  }

  fn knn_vectors_format(&self) -> Result<Self::KnnVectorsFormat> {
    self.delegate.knn_vectors_format()
  }

  fn get_name(&self) -> &str {
    self.delegate.get_name()
  }
}
