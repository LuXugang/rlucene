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

#[cfg(test)]
use crate::core::codecs::Codec;
#[cfg(test)]
use crate::core::codecs::compound_format::CompoundFormat;
#[cfg(test)]
use crate::core::codecs::doc_values_consumer::DocValuesConsumerEnum2;
use crate::core::codecs::doc_values_format::DocValuesFormat;
use crate::core::codecs::doc_values_producer::DocValuesProducer;
#[cfg(test)]
use crate::core::codecs::doc_values_producer::DocValuesProducerEnum2;
#[cfg(test)]
use crate::core::codecs::field_infos_format::FieldInfosFormat;
#[cfg(test)]
use crate::core::codecs::fields_consumer::FieldsConsumerEnum2;
#[cfg(test)]
use crate::core::codecs::fields_producer::FieldsProducerEnum2;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
#[cfg(test)]
use crate::core::codecs::knn_vectors_reader::{KnnVectorsReader, KnnVectorsReaderEnum2};
#[cfg(test)]
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriterEnum2;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
#[cfg(test)]
use crate::core::codecs::lucene90::compressing::lucene90_compressing_stored_fields_format::Lucene90CompressingStoredFieldsFormat;
#[cfg(test)]
use crate::core::codecs::lucene90::compressing::lucene90_compressing_term_vectors_format::Lucene90CompressingTermVectorsFormat;
use crate::core::codecs::lucene90_live_docs_format::Lucene90LiveDocsFormat;
use crate::core::codecs::lucene90_norms_format::Lucene90NormsFormat;
use crate::core::codecs::lucene90_points_format::Lucene90PointsFormat;
use crate::core::codecs::lucene90_stored_fields_format::Lucene90StoredFieldsFormat;
use crate::core::codecs::lucene90_term_vectors_format::Lucene90TermVectorsFormat;
#[cfg(test)]
use crate::core::codecs::lucene101_codec::Lucene101Codec;
use crate::core::codecs::lucene101_codec::{
  Lucene101CodecDocValuesFormat, Lucene101CodecKnnVectorsFormat, Lucene101CodecPostingsFormat,
};
#[cfg(test)]
use crate::core::codecs::norms_consumer::NormsConsumerEnum2;
use crate::core::codecs::norms_format::NormsFormat;
use crate::core::codecs::norms_producer::NormsProducer;
#[cfg(test)]
use crate::core::codecs::norms_producer::NormsProducerEnum2;
use crate::core::codecs::points_format::PointsFormat;
#[cfg(test)]
use crate::core::codecs::points_reader::{PointsReader, PointsReaderEnum2};
#[cfg(test)]
use crate::core::codecs::points_writer::PointsWriter;
use crate::core::codecs::postings_format::PostingsFormat;
#[cfg(test)]
use crate::core::codecs::segment_info_format::SegmentInfoFormat;
use crate::core::codecs::stored_fields_format::StoredFieldsFormat;
#[cfg(test)]
use crate::core::codecs::stored_fields_reader::StoredFieldsReaderEnum2;
#[cfg(test)]
use crate::core::codecs::stored_fields_writer::StoredFieldsWriterEnum2;
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
#[cfg(test)]
use crate::core::codecs::term_vectors_reader::TermVectorsReaderEnum2;
#[cfg(test)]
use crate::core::codecs::term_vectors_writer::TermVectorsWriterEnum2;
#[cfg(test)]
use crate::core::index::codec_reader::CodecReader;
#[cfg(test)]
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::Identity;
#[cfg(test)]
use crate::core::index::knn_vector_values::KnnVectorValues;
#[cfg(test)]
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IOContext, IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
#[cfg(test)]
use crate::core::util::StringHelper;
use crate::core::util::bits::Bits;
#[cfg(test)]
use crate::core::util::bits::BitsEnum2;
#[cfg(test)]
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[cfg(test)]
use crate::core::{
  index::byte_vector_values::ByteVectorValues, index::float_vector_values::FloatVectorValues,
};
#[cfg(test)]
use crate::test_framework::core::codecs::asserting_codec::AssertingCodec;
#[cfg(test)]
use crate::test_framework::core::codecs::cranky::cranky_codec::CrankyCodec;
#[cfg(test)]
use crate::test_framework::core::geo::random_distance_codec::RandomDistanceCodec;
#[cfg(test)]
use crate::test_framework::core::index::base_postings_format_test_case::{
  InvertedWriteFieldsConsumer, InvertedWritePostingsFormat,
};
#[cfg(test)]
use crate::test_framework::core::index::test_index_sorting::AssertingNeedsIndexSortCodec;
#[cfg(test)]
use crate::test_framework::core::index::test_index_writer_force_merge::{
  MergePerFieldCodec, MergePerFieldDocValuesFormat, MergePerFieldPostingsFormat,
};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[cfg(test)]
type AssertingPostingsFormat = <AssertingCodec as Codec>::PostingsFormat;
#[cfg(test)]
type AssertingDocValuesFormat = <AssertingCodec as Codec>::DocValuesFormat;
#[cfg(test)]
type AssertingStoredFieldsFormat = <AssertingCodec as Codec>::StoredFieldsFormat;
#[cfg(test)]
type AssertingTermVectorsFormat = <AssertingCodec as Codec>::TermVectorsFormat;
#[cfg(test)]
type AssertingNormsFormat = <AssertingCodec as Codec>::NormsFormat;
#[cfg(test)]
type AssertingLiveDocsFormat = <AssertingCodec as Codec>::LiveDocsFormat;
#[cfg(test)]
type AssertingPointsFormat = <AssertingCodec as Codec>::PointsFormat;
#[cfg(test)]
type AssertingKnnVectorsFormat = <AssertingCodec as Codec>::KnnVectorsFormat;
#[cfg(test)]
type AssertingNeedsIndexSortPointsFormat = <AssertingNeedsIndexSortCodec as Codec>::PointsFormat;
#[cfg(test)]
type RandomDistancePointsFormat = <RandomDistanceCodec as Codec>::PointsFormat;
#[cfg(test)]
type CrankyLucene101Codec = CrankyCodec<Lucene101Codec>;
#[cfg(test)]
type CrankyAssertingCodec = CrankyCodec<AssertingCodec>;
#[cfg(test)]
type CrankyLucene101PostingsFormat = <CrankyLucene101Codec as Codec>::PostingsFormat;
#[cfg(test)]
type CrankyAssertingPostingsFormat = <CrankyAssertingCodec as Codec>::PostingsFormat;
#[cfg(test)]
type CrankyLucene101DocValuesFormat = <CrankyLucene101Codec as Codec>::DocValuesFormat;
#[cfg(test)]
type CrankyAssertingDocValuesFormat = <CrankyAssertingCodec as Codec>::DocValuesFormat;
#[cfg(test)]
type MergePerFieldCodecPostingsFormat = <MergePerFieldCodec as Codec>::PostingsFormat;
#[cfg(test)]
type MergePerFieldCodecDocValuesFormat = <MergePerFieldCodec as Codec>::DocValuesFormat;
#[cfg(test)]
type CrankyLucene101StoredFieldsFormat = <CrankyLucene101Codec as Codec>::StoredFieldsFormat;
#[cfg(test)]
type CrankyAssertingStoredFieldsFormat = <CrankyAssertingCodec as Codec>::StoredFieldsFormat;
#[cfg(test)]
type CrankyLucene101TermVectorsFormat = <CrankyLucene101Codec as Codec>::TermVectorsFormat;
#[cfg(test)]
type CrankyAssertingTermVectorsFormat = <CrankyAssertingCodec as Codec>::TermVectorsFormat;
#[cfg(test)]
type CrankyLucene101NormsFormat = <CrankyLucene101Codec as Codec>::NormsFormat;
#[cfg(test)]
type CrankyAssertingNormsFormat = <CrankyAssertingCodec as Codec>::NormsFormat;
#[cfg(test)]
type CrankyLucene101LiveDocsFormat = <CrankyLucene101Codec as Codec>::LiveDocsFormat;
#[cfg(test)]
type CrankyAssertingLiveDocsFormat = <CrankyAssertingCodec as Codec>::LiveDocsFormat;
#[cfg(test)]
type CrankyLucene101PointsFormat = <CrankyLucene101Codec as Codec>::PointsFormat;
#[cfg(test)]
type CrankyAssertingPointsFormat = <CrankyAssertingCodec as Codec>::PointsFormat;

pub enum CodecPostingsFormat {
  Lucene101(Lucene101CodecPostingsFormat),
  #[cfg(test)]
  Asserting(AssertingPostingsFormat),
  #[cfg(test)]
  MergePerField(MergePerFieldPostingsFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101PostingsFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingPostingsFormat),
  #[cfg(test)]
  InvertedWrite(InvertedWritePostingsFormat),
}

pub enum CodecDocValuesFormat {
  Lucene101(Lucene101CodecDocValuesFormat),
  #[cfg(test)]
  Asserting(AssertingDocValuesFormat),
  #[cfg(test)]
  MergePerField(MergePerFieldDocValuesFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101DocValuesFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingDocValuesFormat),
}

pub enum CodecStoredFieldsFormat {
  Lucene90(Lucene90StoredFieldsFormat),
  #[cfg(test)]
  Compressing(Lucene90CompressingStoredFieldsFormat),
  #[cfg(test)]
  Asserting(AssertingStoredFieldsFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101StoredFieldsFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingStoredFieldsFormat),
}

pub enum CodecTermVectorsFormat {
  Lucene90(Lucene90TermVectorsFormat),
  #[cfg(test)]
  Compressing(Lucene90CompressingTermVectorsFormat),
  #[cfg(test)]
  Asserting(AssertingTermVectorsFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101TermVectorsFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingTermVectorsFormat),
}

pub enum CodecNormsFormat {
  Lucene90(Lucene90NormsFormat),
  #[cfg(test)]
  Asserting(AssertingNormsFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101NormsFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingNormsFormat),
}

pub enum CodecLiveDocsFormat {
  Lucene90(Lucene90LiveDocsFormat),
  #[cfg(test)]
  Asserting(AssertingLiveDocsFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101LiveDocsFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingLiveDocsFormat),
}

pub enum CodecPointsFormat {
  Lucene90(Lucene90PointsFormat),
  #[cfg(test)]
  Asserting(AssertingPointsFormat),
  #[cfg(test)]
  AssertingNeedsIndexSort(AssertingNeedsIndexSortPointsFormat),
  #[cfg(test)]
  RandomDistance(RandomDistancePointsFormat),
  #[cfg(test)]
  CrankyLucene101(CrankyLucene101PointsFormat),
  #[cfg(test)]
  CrankyAsserting(CrankyAssertingPointsFormat),
}

pub enum CodecKnnVectorsFormat {
  Lucene101(Lucene101CodecKnnVectorsFormat),
  #[cfg(test)]
  Asserting(AssertingKnnVectorsFormat),
}

#[cfg(test)]
pub enum CodecFieldInfosFormat {
  Lucene101(<Lucene101Codec as Codec>::FieldInfosFormat),
  Cranky(<CrankyLucene101Codec as Codec>::FieldInfosFormat),
}

#[cfg(test)]
impl FieldInfosFormat for CodecFieldInfosFormat {
  fn read<D>(
    &self,
    directory: &impl Directory,
    segment_info: &SegmentInfo<D>,
    segment_suffix: &str,
    io_context: &IOContext,
  ) -> Result<FieldInfos>
  where
    D: Directory,
  {
    match self {
      Self::Lucene101(format) => format.read(directory, segment_info, segment_suffix, io_context),
      Self::Cranky(format) => format.read(directory, segment_info, segment_suffix, io_context),
    }
  }

  fn write<D>(
    &self,
    directory: &impl Directory,
    segment_info: &SegmentInfo<D>,
    segment_suffix: &str,
    infos: &FieldInfos,
    io_context: &IOContext,
  ) -> Result<()>
  where
    D: Directory,
  {
    match self {
      Self::Lucene101(format) => {
        format.write(directory, segment_info, segment_suffix, infos, io_context)
      },
      Self::Cranky(format) => {
        format.write(directory, segment_info, segment_suffix, infos, io_context)
      },
    }
  }
}

#[cfg(test)]
pub enum CodecSegmentInfoFormat {
  Lucene101(<Lucene101Codec as Codec>::SegmentInfoFormat),
  Cranky(<CrankyLucene101Codec as Codec>::SegmentInfoFormat),
}

#[cfg(test)]
impl SegmentInfoFormat for CodecSegmentInfoFormat {
  fn read<D>(
    &self,
    directory: Arc<D>,
    segment_name: &str,
    segment_id: &[u8; StringHelper::ID_LENGTH],
    context: &IOContext,
  ) -> Result<SegmentInfo<D>>
  where
    D: Directory,
  {
    match self {
      Self::Lucene101(format) => format.read(directory, segment_name, segment_id, context),
      Self::Cranky(format) => format.read(directory, segment_name, segment_id, context),
    }
  }

  fn write<D>(
    &self,
    directory: &impl Directory,
    info: &mut SegmentInfo<D>,
    context: &IOContext,
  ) -> Result<()>
  where
    D: Directory,
  {
    match self {
      Self::Lucene101(format) => format.write(directory, info, context),
      Self::Cranky(format) => format.write(directory, info, context),
    }
  }
}

#[cfg(test)]
pub enum CodecCompoundFormat {
  Lucene101(<Lucene101Codec as Codec>::CompoundFormat),
  Cranky(<CrankyLucene101Codec as Codec>::CompoundFormat),
}

#[cfg(test)]
impl CompoundFormat for CodecCompoundFormat {
  type Directory<D>
    = <<Lucene101Codec as Codec>::CompoundFormat as CompoundFormat>::Directory<D>
  where
    D: Directory;

  fn get_compound_reader<D>(&self, dir: &D, si: &SegmentInfo<D>) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    match self {
      Self::Lucene101(format) => format.get_compound_reader(dir, si),
      Self::Cranky(format) => format.get_compound_reader(dir, si),
    }
  }

  fn write<D>(&self, dir: &impl Directory, si: &SegmentInfo<D>, context: &IOContext) -> Result<()>
  where
    D: Directory,
  {
    match self {
      Self::Lucene101(format) => format.write(dir, si, context),
      Self::Cranky(format) => format.write(dir, si, context),
    }
  }
}

#[cfg(not(test))]
pub type CodecFieldsConsumer<O> =
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsConsumer<O>;
#[cfg(test)]
pub type BaseCodecFieldsConsumer<O> = FieldsConsumerEnum2<
  FieldsConsumerEnum2<
    FieldsConsumerEnum2<
      <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsConsumer<O>,
      <AssertingPostingsFormat as PostingsFormat>::FieldsConsumer<O>,
    >,
    FieldsConsumerEnum2<
      <CrankyLucene101PostingsFormat as PostingsFormat>::FieldsConsumer<O>,
      <CrankyAssertingPostingsFormat as PostingsFormat>::FieldsConsumer<O>,
    >,
  >,
  <MergePerFieldCodecPostingsFormat as PostingsFormat>::FieldsConsumer<O>,
>;
#[cfg(test)]
pub type CodecFieldsConsumer<O> = FieldsConsumerEnum2<
  BaseCodecFieldsConsumer<O>,
  InvertedWriteFieldsConsumer<BaseCodecFieldsConsumer<O>>,
>;

#[cfg(not(test))]
pub type CodecFieldsProducer<I> =
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsProducer<I>;
#[cfg(test)]
pub type BaseCodecFieldsProducer<I> = FieldsProducerEnum2<
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsProducer<I>,
  <AssertingPostingsFormat as PostingsFormat>::FieldsProducer<I>,
>;
#[cfg(test)]
pub type CodecFieldsProducer<I> =
  FieldsProducerEnum2<BaseCodecFieldsProducer<I>, BaseCodecFieldsProducer<I>>;

#[cfg(test)]
impl CodecPostingsFormat {
  pub(crate) fn base_fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<BaseCodecFieldsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene101(format) => format.fields_consumer(state, segment_info).map(|consumer| {
        FieldsConsumerEnum2::A(FieldsConsumerEnum2::A(FieldsConsumerEnum2::A(consumer)))
      }),
      Self::Asserting(format) => format.fields_consumer(state, segment_info).map(|consumer| {
        FieldsConsumerEnum2::A(FieldsConsumerEnum2::A(FieldsConsumerEnum2::B(consumer)))
      }),
      Self::MergePerField(format) => format
        .fields_consumer(state, segment_info)
        .map(FieldsConsumerEnum2::B),
      Self::CrankyLucene101(format) => {
        format.fields_consumer(state, segment_info).map(|consumer| {
          FieldsConsumerEnum2::A(FieldsConsumerEnum2::B(FieldsConsumerEnum2::A(consumer)))
        })
      },
      Self::CrankyAsserting(format) => {
        format.fields_consumer(state, segment_info).map(|consumer| {
          FieldsConsumerEnum2::A(FieldsConsumerEnum2::B(FieldsConsumerEnum2::B(consumer)))
        })
      },
      Self::InvertedWrite(_) => Err(LuceneError::illegal_state(
        "InvertedWritePostingsFormat cannot wrap itself",
      )),
    }
  }

  pub(crate) fn base_fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<BaseCodecFieldsProducer<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene101(format) => format
        .fields_producer(state, segment_info)
        .map(FieldsProducerEnum2::A),
      Self::Asserting(format) => format
        .fields_producer(state, segment_info)
        .map(FieldsProducerEnum2::B),
      Self::MergePerField(format) => format
        .fields_producer(state, segment_info)
        .map(FieldsProducerEnum2::A),
      Self::CrankyLucene101(format) => format
        .fields_producer(state, segment_info)
        .map(FieldsProducerEnum2::A),
      Self::CrankyAsserting(format) => format
        .fields_producer(state, segment_info)
        .map(FieldsProducerEnum2::B),
      Self::InvertedWrite(_) => Err(LuceneError::illegal_state(
        "InvertedWritePostingsFormat cannot wrap itself",
      )),
    }
  }
}

impl HasIdentity for CodecPostingsFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Lucene101(format) => format.identity(),
      #[cfg(test)]
      Self::Asserting(format) => format.identity(),
      #[cfg(test)]
      Self::MergePerField(format) => format.identity(),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format.identity(),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format.identity(),
      #[cfg(test)]
      Self::InvertedWrite(format) => format.identity(),
    }
  }
}

impl PostingsFormat for CodecPostingsFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Lucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::Asserting(format) => format.get_name(),
      #[cfg(test)]
      Self::MergePerField(format) => format.get_name(),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format.get_name(),
      #[cfg(test)]
      Self::InvertedWrite(format) => format.get_name(),
    }
  }

  type FieldsConsumer<O: IndexOutput> = CodecFieldsConsumer<O>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    #[cfg(not(test))]
    {
      match self {
        Self::Lucene101(format) => format.fields_consumer(state, segment_info),
      }
    }
    #[cfg(test)]
    {
      match self {
        Self::InvertedWrite(format) => format
          .fields_consumer(state, segment_info)
          .map(FieldsConsumerEnum2::B),
        _ => self
          .base_fields_consumer(state, segment_info)
          .map(FieldsConsumerEnum2::A),
      }
    }
  }

  type FieldsProducer<I: IndexInput> = CodecFieldsProducer<I>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsProducer<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    #[cfg(not(test))]
    {
      match self {
        Self::Lucene101(format) => format.fields_producer(state, segment_info),
      }
    }
    #[cfg(test)]
    {
      match self {
        Self::InvertedWrite(format) => format
          .fields_producer(state, segment_info)
          .map(FieldsProducerEnum2::B),
        _ => self
          .base_fields_producer(state, segment_info)
          .map(FieldsProducerEnum2::A),
      }
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Err(LuceneError::illegal_argument(format!(
      "Could not load postings format named \"{name}\""
    )))
  }
}

#[cfg(not(test))]
pub type CodecDocValuesConsumer<O> =
  <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>;
#[cfg(test)]
pub type CodecDocValuesConsumer<O> = DocValuesConsumerEnum2<
  DocValuesConsumerEnum2<
    DocValuesConsumerEnum2<
      <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
      <AssertingDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
    >,
    DocValuesConsumerEnum2<
      <CrankyLucene101DocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
      <CrankyAssertingDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
    >,
  >,
  <MergePerFieldCodecDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
>;

#[cfg(not(test))]
pub type CodecDocValuesProducer<I> =
  <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>;
#[cfg(test)]
pub type CodecDocValuesProducer<I> = DocValuesProducerEnum2<
  <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>,
  <AssertingDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>,
>;

pub type CodecNumericDocValues<I> =
  <CodecDocValuesProducer<I> as DocValuesProducer>::NumericDocValues;
pub type CodecBinaryDocValues<I> =
  <CodecDocValuesProducer<I> as DocValuesProducer>::BinaryDocValues;
pub type CodecSortedDocValues<I> =
  <CodecDocValuesProducer<I> as DocValuesProducer>::SortedDocValues;
pub type CodecSortedNumericDocValues<I> =
  <CodecDocValuesProducer<I> as DocValuesProducer>::SortedNumericDocValues;
pub type CodecSortedSetDocValues<I> =
  <CodecDocValuesProducer<I> as DocValuesProducer>::SortedSetDocValues;
pub type CodecDocValuesSkipper<I> =
  <CodecDocValuesProducer<I> as DocValuesProducer>::DocValuesSkipper;

impl Display for CodecDocValuesFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Lucene101(format) => Display::fmt(format, f),
      #[cfg(test)]
      Self::Asserting(format) => Display::fmt(format, f),
      #[cfg(test)]
      Self::MergePerField(format) => Display::fmt(format, f),
      #[cfg(test)]
      Self::CrankyLucene101(format) => Display::fmt(format, f),
      #[cfg(test)]
      Self::CrankyAsserting(format) => Display::fmt(format, f),
    }
  }
}

impl HasIdentity for CodecDocValuesFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Lucene101(format) => format.identity(),
      #[cfg(test)]
      Self::Asserting(format) => format.identity(),
      #[cfg(test)]
      Self::MergePerField(format) => format.identity(),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format.identity(),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format.identity(),
    }
  }
}

impl DocValuesFormat for CodecDocValuesFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Lucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::Asserting(format) => format.get_name(),
      #[cfg(test)]
      Self::MergePerField(format) => format.get_name(),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format.get_name(),
    }
  }

  type DocValuesConsumer<O: IndexOutput> = CodecDocValuesConsumer<O>;

  fn fields_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesConsumer<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene101(format) => {
        #[cfg(not(test))]
        {
          format.fields_consumer(state, segment_info)
        }
        #[cfg(test)]
        {
          format.fields_consumer(state, segment_info).map(|consumer| {
            DocValuesConsumerEnum2::A(DocValuesConsumerEnum2::A(DocValuesConsumerEnum2::A(
              consumer,
            )))
          })
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format.fields_consumer(state, segment_info).map(|consumer| {
        DocValuesConsumerEnum2::A(DocValuesConsumerEnum2::A(DocValuesConsumerEnum2::B(
          consumer,
        )))
      }),
      #[cfg(test)]
      Self::MergePerField(format) => format
        .fields_consumer(state, segment_info)
        .map(DocValuesConsumerEnum2::B),
      #[cfg(test)]
      Self::CrankyLucene101(format) => {
        format.fields_consumer(state, segment_info).map(|consumer| {
          DocValuesConsumerEnum2::A(DocValuesConsumerEnum2::B(DocValuesConsumerEnum2::A(
            consumer,
          )))
        })
      },
      #[cfg(test)]
      Self::CrankyAsserting(format) => {
        format.fields_consumer(state, segment_info).map(|consumer| {
          DocValuesConsumerEnum2::A(DocValuesConsumerEnum2::B(DocValuesConsumerEnum2::B(
            consumer,
          )))
        })
      },
    }
  }

  type DocValuesProducer<I: IndexInput> = CodecDocValuesProducer<I>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::DocValuesProducer<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene101(format) => {
        #[cfg(not(test))]
        {
          format.fields_producer(state, segment_info)
        }
        #[cfg(test)]
        {
          format
            .fields_producer(state, segment_info)
            .map(DocValuesProducerEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_producer(state, segment_info)
        .map(DocValuesProducerEnum2::B),
      #[cfg(test)]
      Self::MergePerField(format) => format
        .fields_producer(state, segment_info)
        .map(DocValuesProducerEnum2::A),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .fields_producer(state, segment_info)
        .map(DocValuesProducerEnum2::A),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .fields_producer(state, segment_info)
        .map(DocValuesProducerEnum2::B),
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Err(LuceneError::illegal_argument(format!(
      "Could not load doc values format named \"{name}\""
    )))
  }
}

#[cfg(not(test))]
pub type CodecStoredFieldsReader<I> =
  <Lucene90StoredFieldsFormat as StoredFieldsFormat>::StoredFieldsReader<I>;
#[cfg(test)]
pub type CodecStoredFieldsReader<I> = StoredFieldsReaderEnum2<
  <Lucene90StoredFieldsFormat as StoredFieldsFormat>::StoredFieldsReader<I>,
  <AssertingStoredFieldsFormat as StoredFieldsFormat>::StoredFieldsReader<I>,
>;

#[cfg(not(test))]
pub type CodecStoredFieldsWriter<D> =
  <Lucene90StoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>;
#[cfg(test)]
pub type CodecStoredFieldsWriter<D> = StoredFieldsWriterEnum2<
  StoredFieldsWriterEnum2<
    <Lucene90StoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>,
    <AssertingStoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>,
  >,
  StoredFieldsWriterEnum2<
    <CrankyLucene101StoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>,
    <CrankyAssertingStoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>,
  >,
>;

impl StoredFieldsFormat for CodecStoredFieldsFormat {
  type StoredFieldsReader<I: IndexInput> = CodecStoredFieldsReader<I>;

  fn fields_reader<D1, D2>(
    &self,
    directory: &D1,
    segment_info: &SegmentInfo<D2>,
    field_infos: Arc<FieldInfos>,
    context: &IOContext,
  ) -> Result<Self::StoredFieldsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.fields_reader(directory, segment_info, field_infos, context)
        }
        #[cfg(test)]
        {
          format
            .fields_reader(directory, segment_info, field_infos, context)
            .map(StoredFieldsReaderEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Compressing(format) => format
        .fields_reader(directory, segment_info, field_infos, context)
        .map(StoredFieldsReaderEnum2::A),
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_reader(directory, segment_info, field_infos, context)
        .map(StoredFieldsReaderEnum2::B),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .fields_reader(directory, segment_info, field_infos, context)
        .map(StoredFieldsReaderEnum2::A),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .fields_reader(directory, segment_info, field_infos, context)
        .map(StoredFieldsReaderEnum2::B),
    }
  }

  type StoredFieldsWriter<D: Directory> = CodecStoredFieldsWriter<D>;

  fn fields_writer<D1, D2>(
    &self,
    directory: D1,
    segment_info: &mut SegmentInfo<D2>,
    context: &IOContext,
  ) -> Result<Self::StoredFieldsWriter<D1>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.fields_writer(directory, segment_info, context)
        }
        #[cfg(test)]
        {
          format
            .fields_writer(directory, segment_info, context)
            .map(|writer| StoredFieldsWriterEnum2::A(StoredFieldsWriterEnum2::A(writer)))
        }
      },
      #[cfg(test)]
      Self::Compressing(format) => format
        .fields_writer(directory, segment_info, context)
        .map(|writer| StoredFieldsWriterEnum2::A(StoredFieldsWriterEnum2::A(writer))),
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_writer(directory, segment_info, context)
        .map(|writer| StoredFieldsWriterEnum2::A(StoredFieldsWriterEnum2::B(writer))),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .fields_writer(directory, segment_info, context)
        .map(|writer| StoredFieldsWriterEnum2::B(StoredFieldsWriterEnum2::A(writer))),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .fields_writer(directory, segment_info, context)
        .map(|writer| StoredFieldsWriterEnum2::B(StoredFieldsWriterEnum2::B(writer))),
    }
  }
}

#[cfg(not(test))]
pub type CodecTermVectorsReader<I> =
  <Lucene90TermVectorsFormat as TermVectorsFormat>::TermVectorsReader<I>;
#[cfg(test)]
pub type CodecTermVectorsReader<I> = TermVectorsReaderEnum2<
  <Lucene90TermVectorsFormat as TermVectorsFormat>::TermVectorsReader<I>,
  <AssertingTermVectorsFormat as TermVectorsFormat>::TermVectorsReader<I>,
>;

#[cfg(not(test))]
pub type CodecTermVectorsWriter<D> =
  <Lucene90TermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>;
#[cfg(test)]
pub type CodecTermVectorsWriter<D> = TermVectorsWriterEnum2<
  TermVectorsWriterEnum2<
    <Lucene90TermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>,
    <AssertingTermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>,
  >,
  TermVectorsWriterEnum2<
    <CrankyLucene101TermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>,
    <CrankyAssertingTermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>,
  >,
>;

impl TermVectorsFormat for CodecTermVectorsFormat {
  type TermVectorsReader<I: IndexInput> = CodecTermVectorsReader<I>;

  fn vectors_reader<D1, D2>(
    &self,
    directory: &D1,
    segment_info: &SegmentInfo<D2>,
    field_infos: Arc<FieldInfos>,
    context: &IOContext,
  ) -> Result<Self::TermVectorsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.vectors_reader(directory, segment_info, field_infos, context)
        }
        #[cfg(test)]
        {
          format
            .vectors_reader(directory, segment_info, field_infos, context)
            .map(TermVectorsReaderEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Compressing(format) => format
        .vectors_reader(directory, segment_info, field_infos, context)
        .map(TermVectorsReaderEnum2::A),
      #[cfg(test)]
      Self::Asserting(format) => format
        .vectors_reader(directory, segment_info, field_infos, context)
        .map(TermVectorsReaderEnum2::B),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .vectors_reader(directory, segment_info, field_infos, context)
        .map(TermVectorsReaderEnum2::A),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .vectors_reader(directory, segment_info, field_infos, context)
        .map(TermVectorsReaderEnum2::B),
    }
  }

  type TermVectorsWriter<D: Directory> = CodecTermVectorsWriter<D>;

  fn vectors_writer<D1, D2>(
    &self,
    directory: D1,
    segment_info: &SegmentInfo<D2>,
    context: &IOContext,
  ) -> Result<Self::TermVectorsWriter<D1>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.vectors_writer(directory, segment_info, context)
        }
        #[cfg(test)]
        {
          format
            .vectors_writer(directory, segment_info, context)
            .map(|writer| TermVectorsWriterEnum2::A(TermVectorsWriterEnum2::A(writer)))
        }
      },
      #[cfg(test)]
      Self::Compressing(format) => format
        .vectors_writer(directory, segment_info, context)
        .map(|writer| TermVectorsWriterEnum2::A(TermVectorsWriterEnum2::A(writer))),
      #[cfg(test)]
      Self::Asserting(format) => format
        .vectors_writer(directory, segment_info, context)
        .map(|writer| TermVectorsWriterEnum2::A(TermVectorsWriterEnum2::B(writer))),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .vectors_writer(directory, segment_info, context)
        .map(|writer| TermVectorsWriterEnum2::B(TermVectorsWriterEnum2::A(writer))),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .vectors_writer(directory, segment_info, context)
        .map(|writer| TermVectorsWriterEnum2::B(TermVectorsWriterEnum2::B(writer))),
    }
  }
}

#[cfg(not(test))]
pub type CodecNormsConsumer<O> = <Lucene90NormsFormat as NormsFormat>::NormsConsumer<O>;
#[cfg(test)]
pub type CodecNormsConsumer<O> = NormsConsumerEnum2<
  NormsConsumerEnum2<
    <Lucene90NormsFormat as NormsFormat>::NormsConsumer<O>,
    <AssertingNormsFormat as NormsFormat>::NormsConsumer<O>,
  >,
  NormsConsumerEnum2<
    <CrankyLucene101NormsFormat as NormsFormat>::NormsConsumer<O>,
    <CrankyAssertingNormsFormat as NormsFormat>::NormsConsumer<O>,
  >,
>;

#[cfg(not(test))]
pub type CodecNormsProducer<I> = <Lucene90NormsFormat as NormsFormat>::NormsProducer<I>;
#[cfg(test)]
pub type CodecNormsProducer<I> = NormsProducerEnum2<
  <Lucene90NormsFormat as NormsFormat>::NormsProducer<I>,
  <AssertingNormsFormat as NormsFormat>::NormsProducer<I>,
>;

pub type CodecNormNumericDocValues<I> = <CodecNormsProducer<I> as NormsProducer>::NumericDocValues;

impl NormsFormat for CodecNormsFormat {
  type NormsConsumer<O: IndexOutput> = CodecNormsConsumer<O>;

  fn norms_consumer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::NormsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.norms_consumer(state, segment_info)
        }
        #[cfg(test)]
        {
          format
            .norms_consumer(state, segment_info)
            .map(|consumer| NormsConsumerEnum2::A(NormsConsumerEnum2::A(consumer)))
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .norms_consumer(state, segment_info)
        .map(|consumer| NormsConsumerEnum2::A(NormsConsumerEnum2::B(consumer))),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .norms_consumer(state, segment_info)
        .map(|consumer| NormsConsumerEnum2::B(NormsConsumerEnum2::A(consumer))),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .norms_consumer(state, segment_info)
        .map(|consumer| NormsConsumerEnum2::B(NormsConsumerEnum2::B(consumer))),
    }
  }

  type NormsProducer<I: IndexInput> = CodecNormsProducer<I>;

  fn norms_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::NormsProducer<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.norms_producer(state, segment_info)
        }
        #[cfg(test)]
        {
          format
            .norms_producer(state, segment_info)
            .map(NormsProducerEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .norms_producer(state, segment_info)
        .map(NormsProducerEnum2::B),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .norms_producer(state, segment_info)
        .map(NormsProducerEnum2::A),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .norms_producer(state, segment_info)
        .map(NormsProducerEnum2::B),
    }
  }
}

#[cfg(not(test))]
pub type CodecLiveDocsBits = <Lucene90LiveDocsFormat as LiveDocsFormat>::Bits;
#[cfg(test)]
pub type CodecLiveDocsBits = BitsEnum2<
  <Lucene90LiveDocsFormat as LiveDocsFormat>::Bits,
  <AssertingLiveDocsFormat as LiveDocsFormat>::Bits,
>;

impl LiveDocsFormat for CodecLiveDocsFormat {
  type Bits = CodecLiveDocsBits;

  fn read_live_docs<D>(
    &self,
    dir: &impl Directory,
    info: &SegmentCommitInfo<D>,
    context: &IOContext,
  ) -> Result<Self::Bits>
  where
    D: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.read_live_docs(dir, info, context)
        }
        #[cfg(test)]
        {
          format.read_live_docs(dir, info, context).map(BitsEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format.read_live_docs(dir, info, context).map(BitsEnum2::B),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format.read_live_docs(dir, info, context).map(BitsEnum2::A),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format.read_live_docs(dir, info, context).map(BitsEnum2::B),
    }
  }

  fn write_live_docs<D>(
    &self,
    bits: &impl Bits,
    dir: &impl Directory,
    info: &SegmentCommitInfo<D>,
    new_del_count: i32,
    context: &IOContext,
  ) -> Result<()>
  where
    D: Directory,
  {
    match self {
      Self::Lucene90(format) => format.write_live_docs(bits, dir, info, new_del_count, context),
      #[cfg(test)]
      Self::Asserting(format) => format.write_live_docs(bits, dir, info, new_del_count, context),
      #[cfg(test)]
      Self::CrankyLucene101(format) => {
        format.write_live_docs(bits, dir, info, new_del_count, context)
      },
      #[cfg(test)]
      Self::CrankyAsserting(format) => {
        format.write_live_docs(bits, dir, info, new_del_count, context)
      },
    }
  }

  fn files<D>(&self, info: &SegmentCommitInfo<D>, files: &mut HashSet<String>) -> Result<()>
  where
    D: Directory,
  {
    match self {
      Self::Lucene90(format) => format.files(info, files),
      #[cfg(test)]
      Self::Asserting(format) => format.files(info, files),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format.files(info, files),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format.files(info, files),
    }
  }
}

#[cfg(not(test))]
pub type CodecPointsWriter<O> = <Lucene90PointsFormat as PointsFormat>::PointsWriter<O>;
#[cfg(test)]
pub enum CodecPointsWriter<O: IndexOutput> {
  Lucene90(<Lucene90PointsFormat as PointsFormat>::PointsWriter<O>),
  Asserting(<AssertingPointsFormat as PointsFormat>::PointsWriter<O>),
  AssertingNeedsIndexSort(<AssertingNeedsIndexSortPointsFormat as PointsFormat>::PointsWriter<O>),
  CrankyLucene101(<CrankyLucene101PointsFormat as PointsFormat>::PointsWriter<O>),
  CrankyAsserting(<CrankyAssertingPointsFormat as PointsFormat>::PointsWriter<O>),
}

#[cfg(test)]
impl<O: IndexOutput> Closeable for CodecPointsWriter<O> {
  fn close(&mut self) -> Result<()> {
    match self {
      Self::Lucene90(inner) => inner.close(),
      Self::Asserting(inner) => inner.close(),
      Self::AssertingNeedsIndexSort(inner) => inner.close(),
      Self::CrankyLucene101(inner) => inner.close(),
      Self::CrankyAsserting(inner) => inner.close(),
    }
  }
}

#[cfg(test)]
impl<O: IndexOutput> PointsWriter for CodecPointsWriter<O> {
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
      Self::Lucene90(inner) => inner.write_field(field_info, values, dir, segment_info),
      Self::Asserting(inner) => inner.write_field(field_info, values, dir, segment_info),
      Self::AssertingNeedsIndexSort(inner) => {
        inner.write_field(field_info, values, dir, segment_info)
      },
      Self::CrankyLucene101(inner) => inner.write_field(field_info, values, dir, segment_info),
      Self::CrankyAsserting(inner) => inner.write_field(field_info, values, dir, segment_info),
    }
  }

  fn finish(&mut self) -> Result<()> {
    match self {
      Self::Lucene90(inner) => inner.finish(),
      Self::Asserting(inner) => inner.finish(),
      Self::AssertingNeedsIndexSort(inner) => inner.finish(),
      Self::CrankyLucene101(inner) => inner.finish(),
      Self::CrankyAsserting(inner) => inner.finish(),
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
      Self::Lucene90(inner) => inner.merge_one_field(merge_state, field_info, dir),
      Self::Asserting(inner) => inner.merge_one_field(merge_state, field_info, dir),
      Self::AssertingNeedsIndexSort(inner) => inner.merge_one_field(merge_state, field_info, dir),
      Self::CrankyLucene101(inner) => inner.merge_one_field(merge_state, field_info, dir),
      Self::CrankyAsserting(inner) => inner.merge_one_field(merge_state, field_info, dir),
    }
  }

  fn merge<D1, D2, CR>(&mut self, merge_state: &MergeState<D1, CR>, dir: &D2) -> Result<()>
  where
    D1: Directory,
    D2: Directory,
    CR: CodecReader,
  {
    match self {
      Self::Lucene90(inner) => inner.merge(merge_state, dir),
      Self::Asserting(inner) => inner.merge(merge_state, dir),
      Self::AssertingNeedsIndexSort(inner) => inner.merge(merge_state, dir),
      Self::CrankyLucene101(inner) => inner.merge(merge_state, dir),
      Self::CrankyAsserting(inner) => inner.merge(merge_state, dir),
    }
  }
}

#[cfg(not(test))]
pub type CodecPointsReader<I> = <Lucene90PointsFormat as PointsFormat>::PointsReader<I>;
#[cfg(test)]
pub type CodecPointsReader<I> = PointsReaderEnum2<
  PointsReaderEnum2<
    <Lucene90PointsFormat as PointsFormat>::PointsReader<I>,
    <AssertingPointsFormat as PointsFormat>::PointsReader<I>,
  >,
  PointsReaderEnum2<
    <CrankyLucene101PointsFormat as PointsFormat>::PointsReader<I>,
    <CrankyAssertingPointsFormat as PointsFormat>::PointsReader<I>,
  >,
>;

impl PointsFormat for CodecPointsFormat {
  type PointsWriter<O: IndexOutput> = CodecPointsWriter<O>;

  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    info: &SegmentInfo<D2>,
  ) -> Result<Self::PointsWriter<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.fields_writer(state, info)
        }
        #[cfg(test)]
        {
          format
            .fields_writer(state, info)
            .map(CodecPointsWriter::Lucene90)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_writer(state, info)
        .map(CodecPointsWriter::Asserting),
      #[cfg(test)]
      Self::AssertingNeedsIndexSort(format) => format
        .fields_writer(state, info)
        .map(CodecPointsWriter::AssertingNeedsIndexSort),
      #[cfg(test)]
      Self::RandomDistance(format) => format
        .fields_writer(state, info)
        .map(CodecPointsWriter::Lucene90),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .fields_writer(state, info)
        .map(CodecPointsWriter::CrankyLucene101),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .fields_writer(state, info)
        .map(CodecPointsWriter::CrankyAsserting),
    }
  }

  type PointsReader<I: IndexInput> = CodecPointsReader<I>;

  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    info: &SegmentInfo<D2>,
  ) -> Result<Self::PointsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene90(format) => {
        #[cfg(not(test))]
        {
          format.fields_reader(state, info)
        }
        #[cfg(test)]
        {
          format
            .fields_reader(state, info)
            .map(|reader| PointsReaderEnum2::A(PointsReaderEnum2::A(reader)))
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_reader(state, info)
        .map(|reader| PointsReaderEnum2::A(PointsReaderEnum2::B(reader))),
      #[cfg(test)]
      Self::AssertingNeedsIndexSort(format) => format
        .fields_reader(state, info)
        .map(|reader| PointsReaderEnum2::A(PointsReaderEnum2::A(reader))),
      #[cfg(test)]
      Self::RandomDistance(format) => format
        .fields_reader(state, info)
        .map(|reader| PointsReaderEnum2::A(PointsReaderEnum2::A(reader))),
      #[cfg(test)]
      Self::CrankyLucene101(format) => format
        .fields_reader(state, info)
        .map(|reader| PointsReaderEnum2::B(PointsReaderEnum2::A(reader))),
      #[cfg(test)]
      Self::CrankyAsserting(format) => format
        .fields_reader(state, info)
        .map(|reader| PointsReaderEnum2::B(PointsReaderEnum2::B(reader))),
    }
  }
}

#[cfg(not(test))]
pub type CodecKnnVectorsWriter<O> =
  <Lucene101CodecKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>;
#[cfg(test)]
pub type CodecKnnVectorsWriter<O> = KnnVectorsWriterEnum2<
  <Lucene101CodecKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>,
  <AssertingKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>,
>;

#[cfg(not(test))]
pub type CodecKnnVectorsReader<I> =
  <Lucene101CodecKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>;
#[cfg(test)]
type CodecKnnVectorsReaderInner<I> = KnnVectorsReaderEnum2<
  <Lucene101CodecKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>,
  <AssertingKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>,
>;

#[cfg(test)]
type CodecFloatVectorValuesInner<I> =
  <CodecKnnVectorsReaderInner<I> as KnnVectorsReader>::FloatVectorValues;

#[cfg(test)]
pub struct CodecFloatVectorValues<I>(CodecFloatVectorValuesInner<I>)
where
  I: IndexInput;

#[cfg(test)]
impl<I> KnnVectorValues for CodecFloatVectorValues<I>
where
  I: IndexInput,
{
  fn dimension(&self) -> usize {
    self.0.dimension()
  }

  fn size(&self) -> usize {
    self.0.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.0.ord_to_doc(ord)
  }

  type KnnVectorValues = <CodecFloatVectorValuesInner<I> as KnnVectorValues>::KnnVectorValues;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    self.0.copy()
  }

  fn get_vector_byte_length(&self) -> usize {
    self.0.get_vector_byte_length()
  }

  fn get_encoding(&self) -> crate::core::index::vector_encoding::VectorEncoding {
    KnnVectorValues::get_encoding(&self.0)
  }

  type Bits<'a, B>
    = <CodecFloatVectorValuesInner<I> as KnnVectorValues>::Bits<'a, B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.0.get_accept_ords(accept_docs)
  }

  type DocIndexIterator = <CodecFloatVectorValuesInner<I> as KnnVectorValues>::DocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    self.0.iterator()
  }
}

#[cfg(test)]
impl<I> FloatVectorValues for CodecFloatVectorValues<I>
where
  I: IndexInput,
{
  fn vector_value(
    &self,
    ord: usize,
  ) -> Result<std::borrow::Cow<'_, crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>>
  {
    self.0.vector_value(ord)
  }

  type FloatVectorValues = <CodecFloatVectorValuesInner<I> as FloatVectorValues>::FloatVectorValues;

  fn float_copy(&self) -> Result<Option<Self::FloatVectorValues>> {
    self.0.float_copy()
  }

  type VectorScorer = <CodecFloatVectorValuesInner<I> as FloatVectorValues>::VectorScorer;

  fn scorer(&self, target: Vec<f32>) -> Result<Option<Self::VectorScorer>> {
    self.0.scorer(target)
  }

  fn get_encoding(&self) -> crate::core::index::vector_encoding::VectorEncoding {
    FloatVectorValues::get_encoding(&self.0)
  }

  fn get_vectors_mut(
    &mut self,
  ) -> Result<&mut Vec<crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>> {
    self.0.get_vectors_mut()
  }

  fn get_vectors(
    &self,
  ) -> Result<&[crate::core::codecs::knn_field_vectors_writer::VectorValueEnum]> {
    self.0.get_vectors()
  }

  fn get_vectors_capacity(&self) -> Result<usize> {
    self.0.get_vectors_capacity()
  }
}

#[cfg(test)]
type CodecByteVectorValuesInner<I> =
  <CodecKnnVectorsReaderInner<I> as KnnVectorsReader>::ByteVectorValues;

#[cfg(test)]
pub struct CodecByteVectorValues<I>(CodecByteVectorValuesInner<I>)
where
  I: IndexInput;

#[cfg(test)]
impl<I> KnnVectorValues for CodecByteVectorValues<I>
where
  I: IndexInput,
{
  fn dimension(&self) -> usize {
    self.0.dimension()
  }

  fn size(&self) -> usize {
    self.0.size()
  }

  fn ord_to_doc(&self, ord: usize) -> Result<usize> {
    self.0.ord_to_doc(ord)
  }

  type KnnVectorValues = <CodecByteVectorValuesInner<I> as KnnVectorValues>::KnnVectorValues;

  fn copy(&self) -> Result<Self::KnnVectorValues> {
    self.0.copy()
  }

  fn get_vector_byte_length(&self) -> usize {
    self.0.get_vector_byte_length()
  }

  fn get_encoding(&self) -> crate::core::index::vector_encoding::VectorEncoding {
    KnnVectorValues::get_encoding(&self.0)
  }

  type Bits<'a, B>
    = <CodecByteVectorValuesInner<I> as KnnVectorValues>::Bits<'a, B>
  where
    B: Bits,
    Self: 'a;

  fn get_accept_ords<'a, B>(&'a self, accept_docs: Option<B>) -> Option<Self::Bits<'a, B>>
  where
    B: Bits,
  {
    self.0.get_accept_ords(accept_docs)
  }

  type DocIndexIterator = <CodecByteVectorValuesInner<I> as KnnVectorValues>::DocIndexIterator;

  fn iterator(&self) -> Result<Self::DocIndexIterator> {
    self.0.iterator()
  }
}

#[cfg(test)]
impl<I> ByteVectorValues for CodecByteVectorValues<I>
where
  I: IndexInput,
{
  fn vector_value(
    &self,
    ord: usize,
  ) -> Result<std::borrow::Cow<'_, crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>>
  {
    self.0.vector_value(ord)
  }

  type ByteVectorValues = <CodecByteVectorValuesInner<I> as ByteVectorValues>::ByteVectorValues;

  fn byte_copy(&self) -> Result<Option<Self::ByteVectorValues>> {
    self.0.byte_copy()
  }

  type VectorScorer = <CodecByteVectorValuesInner<I> as ByteVectorValues>::VectorScorer;

  fn scorer(&self, target: Vec<u8>) -> Result<Option<Self::VectorScorer>> {
    self.0.scorer(target)
  }

  fn get_encoding(&self) -> crate::core::index::vector_encoding::VectorEncoding {
    ByteVectorValues::get_encoding(&self.0)
  }

  fn get_vectors_mut(
    &mut self,
  ) -> Result<&mut Vec<crate::core::codecs::knn_field_vectors_writer::VectorValueEnum>> {
    self.0.get_vectors_mut()
  }

  fn get_vectors(
    &self,
  ) -> Result<&[crate::core::codecs::knn_field_vectors_writer::VectorValueEnum]> {
    self.0.get_vectors()
  }

  fn get_vectors_capacity(&self) -> Result<usize> {
    self.0.get_vectors_capacity()
  }
}

#[cfg(test)]
pub struct CodecKnnVectorsReader<I>(CodecKnnVectorsReaderInner<I>)
where
  I: IndexInput;

#[cfg(test)]
impl<I> CodecKnnVectorsReader<I>
where
  I: IndexInput,
{
  pub(crate) fn as_inner(&self) -> &CodecKnnVectorsReaderInner<I> {
    &self.0
  }
}

#[cfg(test)]
impl<I> crate::core::util::close::CloseableRef for CodecKnnVectorsReader<I>
where
  I: IndexInput,
{
  fn close(&self) -> Result<()> {
    self.0.close()
  }
}

#[cfg(test)]
impl<I> crate::core::codecs::hnsw::hnsw_graph_provider::HnswGraphProvider
  for CodecKnnVectorsReader<I>
where
  I: IndexInput,
{
  type HnswGraph = <CodecKnnVectorsReaderInner<I> as crate::core::codecs::hnsw::hnsw_graph_provider::HnswGraphProvider>::HnswGraph;

  fn is_hnsw_graph_provider(&self, field: &str) -> bool {
    self.0.is_hnsw_graph_provider(field)
  }

  fn get_graph(&self, field: &str) -> Result<Self::HnswGraph> {
    self.0.get_graph(field)
  }
}

#[cfg(test)]
impl<I> KnnVectorsReader for CodecKnnVectorsReader<I>
where
  I: IndexInput,
{
  fn check_integrity(&self) -> Result<()> {
    self.0.check_integrity()
  }

  type FloatVectorValues = CodecFloatVectorValues<I>;

  fn get_float_vector_values(&self, field: &str) -> Result<Self::FloatVectorValues> {
    self
      .0
      .get_float_vector_values(field)
      .map(CodecFloatVectorValues)
  }

  type ByteVectorValues = CodecByteVectorValues<I>;

  fn get_byte_vector_values(&self, field: &str) -> Result<Self::ByteVectorValues> {
    self
      .0
      .get_byte_vector_values(field)
      .map(CodecByteVectorValues)
  }

  fn get_quantization_state(
    &self,
    field: &str,
  ) -> Result<Option<crate::core::util::quantization::scalar_quantizer::ScalarQuantizer>> {
    self.0.get_quantization_state(field)
  }

  fn is_flat_vectors_reader(&self, field: &str) -> bool {
    self.0.is_flat_vectors_reader(field)
  }

  fn search_f32<B, K>(
    &self,
    field: &str,
    target: Vec<f32>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: crate::core::search::knn_collector::KnnCollector,
  {
    self.0.search_f32(field, target, knn_collector, accept_docs)
  }

  fn search_u8<B, K>(
    &self,
    field: &str,
    target: Vec<u8>,
    knn_collector: &mut K,
    accept_docs: Option<B>,
  ) -> Result<()>
  where
    B: Bits,
    K: crate::core::search::knn_collector::KnnCollector,
  {
    self.0.search_u8(field, target, knn_collector, accept_docs)
  }

  fn get_merge_instance(&self) -> Result<Option<Self>> {
    self
      .0
      .get_merge_instance()
      .map(|reader| reader.map(CodecKnnVectorsReader))
  }

  fn finish_merge(&self) -> Result<()> {
    self.0.finish_merge()
  }
}

impl Display for CodecKnnVectorsFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Lucene101(format) => Display::fmt(format, f),
      #[cfg(test)]
      Self::Asserting(format) => Display::fmt(format, f),
    }
  }
}

impl HasIdentity for CodecKnnVectorsFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Lucene101(format) => format.identity(),
      #[cfg(test)]
      Self::Asserting(format) => format.identity(),
    }
  }
}

impl KnnVectorsFormat for CodecKnnVectorsFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Lucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::Asserting(format) => format.get_name(),
    }
  }

  type KnnVectorsWriter<O: IndexOutput> = CodecKnnVectorsWriter<O>;

  fn fields_writer<D1, D2>(
    &self,
    state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsWriter<D1::IndexOutput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene101(format) => {
        #[cfg(not(test))]
        {
          format.fields_writer(state, segment_info)
        }
        #[cfg(test)]
        {
          format
            .fields_writer(state, segment_info)
            .map(KnnVectorsWriterEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_writer(state, segment_info)
        .map(KnnVectorsWriterEnum2::B),
    }
  }

  type KnnVectorsReader<I: IndexInput> = CodecKnnVectorsReader<I>;

  fn fields_reader<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::KnnVectorsReader<D1::IndexInput>>
  where
    D1: Directory,
    D2: Directory,
  {
    match self {
      Self::Lucene101(format) => {
        #[cfg(not(test))]
        {
          format.fields_reader(state, segment_info)
        }
        #[cfg(test)]
        {
          format
            .fields_reader(state, segment_info)
            .map(|reader| CodecKnnVectorsReader(KnnVectorsReaderEnum2::A(reader)))
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_reader(state, segment_info)
        .map(|reader| CodecKnnVectorsReader(KnnVectorsReaderEnum2::B(reader))),
    }
  }

  fn get_max_dimensions(&self, field_name: &str) -> Result<usize> {
    match self {
      Self::Lucene101(format) => format.get_max_dimensions(field_name),
      #[cfg(test)]
      Self::Asserting(format) => format.get_max_dimensions(field_name),
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Err(LuceneError::illegal_argument(format!(
      "Could not load vectors format named \"{name}\""
    )))
  }
}
