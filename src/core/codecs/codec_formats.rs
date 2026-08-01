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
use crate::core::codecs::doc_values_consumer::DocValuesConsumerEnum2;
use crate::core::codecs::doc_values_format::DocValuesFormat;
#[cfg(test)]
use crate::core::codecs::doc_values_producer::DocValuesProducerEnum2;
#[cfg(test)]
use crate::core::codecs::fields_consumer::FieldsConsumerEnum2;
#[cfg(test)]
use crate::core::codecs::fields_producer::FieldsProducerEnum2;
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
#[cfg(test)]
use crate::core::codecs::knn_vectors_reader::KnnVectorsReaderEnum2;
#[cfg(test)]
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriterEnum2;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::codecs::lucene90_live_docs_format::Lucene90LiveDocsFormat;
use crate::core::codecs::lucene90_norms_format::Lucene90NormsFormat;
use crate::core::codecs::lucene90_points_format::Lucene90PointsFormat;
use crate::core::codecs::lucene90_stored_fields_format::Lucene90StoredFieldsFormat;
use crate::core::codecs::lucene90_term_vectors_format::Lucene90TermVectorsFormat;
use crate::core::codecs::lucene101_codec::{
  Lucene101CodecDocValuesFormat, Lucene101CodecKnnVectorsFormat, Lucene101CodecPostingsFormat,
};
#[cfg(test)]
use crate::core::codecs::norms_consumer::NormsConsumerEnum2;
use crate::core::codecs::norms_format::NormsFormat;
#[cfg(test)]
use crate::core::codecs::norms_producer::NormsProducerEnum2;
use crate::core::codecs::points_format::PointsFormat;
#[cfg(test)]
use crate::core::codecs::points_reader::PointsReaderEnum2;
#[cfg(test)]
use crate::core::codecs::points_writer::PointsWriterEnum2;
use crate::core::codecs::postings_format::PostingsFormat;
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
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::index_reader::Identity;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IOContext, IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::bits::Bits;
#[cfg(test)]
use crate::core::util::bits::BitsEnum2;
use crate::core::util::error::lucene_error::{LuceneError, Result};
#[cfg(test)]
use crate::test_framework::core::codecs::asserting_codec::AssertingCodec;
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

pub enum CodecPostingsFormat {
  Lucene101(Lucene101CodecPostingsFormat),
  #[cfg(test)]
  Asserting(AssertingPostingsFormat),
}

pub enum CodecDocValuesFormat {
  Lucene101(Lucene101CodecDocValuesFormat),
  #[cfg(test)]
  Asserting(AssertingDocValuesFormat),
}

pub enum CodecStoredFieldsFormat {
  Lucene90(Lucene90StoredFieldsFormat),
  #[cfg(test)]
  Asserting(AssertingStoredFieldsFormat),
}

pub enum CodecTermVectorsFormat {
  Lucene90(Lucene90TermVectorsFormat),
  #[cfg(test)]
  Asserting(AssertingTermVectorsFormat),
}

pub enum CodecNormsFormat {
  Lucene90(Lucene90NormsFormat),
  #[cfg(test)]
  Asserting(AssertingNormsFormat),
}

pub enum CodecLiveDocsFormat {
  Lucene90(Lucene90LiveDocsFormat),
  #[cfg(test)]
  Asserting(AssertingLiveDocsFormat),
}

pub enum CodecPointsFormat {
  Lucene90(Lucene90PointsFormat),
  #[cfg(test)]
  Asserting(AssertingPointsFormat),
}

pub enum CodecKnnVectorsFormat {
  Lucene101(Lucene101CodecKnnVectorsFormat),
  #[cfg(test)]
  Asserting(AssertingKnnVectorsFormat),
}

#[cfg(not(test))]
pub type CodecFieldsConsumer<O> =
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsConsumer<O>;
#[cfg(test)]
pub type CodecFieldsConsumer<O> = FieldsConsumerEnum2<
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsConsumer<O>,
  <AssertingPostingsFormat as PostingsFormat>::FieldsConsumer<O>,
>;

#[cfg(not(test))]
pub type CodecFieldsProducer<I> =
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsProducer<I>;
#[cfg(test)]
pub type CodecFieldsProducer<I> = FieldsProducerEnum2<
  <Lucene101CodecPostingsFormat as PostingsFormat>::FieldsProducer<I>,
  <AssertingPostingsFormat as PostingsFormat>::FieldsProducer<I>,
>;

impl HasIdentity for CodecPostingsFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Lucene101(format) => format.identity(),
      #[cfg(test)]
      Self::Asserting(format) => format.identity(),
    }
  }
}

impl PostingsFormat for CodecPostingsFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Lucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::Asserting(format) => format.get_name(),
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
    match self {
      Self::Lucene101(format) => {
        #[cfg(not(test))]
        {
          format.fields_consumer(state, segment_info)
        }
        #[cfg(test)]
        {
          format
            .fields_consumer(state, segment_info)
            .map(FieldsConsumerEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_consumer(state, segment_info)
        .map(FieldsConsumerEnum2::B),
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
            .map(FieldsProducerEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_producer(state, segment_info)
        .map(FieldsProducerEnum2::B),
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
  <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
  <AssertingDocValuesFormat as DocValuesFormat>::DocValuesConsumer<O>,
>;

#[cfg(not(test))]
pub type CodecDocValuesProducer<I> =
  <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>;
#[cfg(test)]
pub type CodecDocValuesProducer<I> = DocValuesProducerEnum2<
  <Lucene101CodecDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>,
  <AssertingDocValuesFormat as DocValuesFormat>::DocValuesProducer<I>,
>;

impl Display for CodecDocValuesFormat {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Lucene101(format) => Display::fmt(format, f),
      #[cfg(test)]
      Self::Asserting(format) => Display::fmt(format, f),
    }
  }
}

impl HasIdentity for CodecDocValuesFormat {
  fn identity(&self) -> &Identity {
    match self {
      Self::Lucene101(format) => format.identity(),
      #[cfg(test)]
      Self::Asserting(format) => format.identity(),
    }
  }
}

impl DocValuesFormat for CodecDocValuesFormat {
  fn get_name(&self) -> &str {
    match self {
      Self::Lucene101(format) => format.get_name(),
      #[cfg(test)]
      Self::Asserting(format) => format.get_name(),
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
          format
            .fields_consumer(state, segment_info)
            .map(DocValuesConsumerEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_consumer(state, segment_info)
        .map(DocValuesConsumerEnum2::B),
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
  <Lucene90StoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>,
  <AssertingStoredFieldsFormat as StoredFieldsFormat>::StoredFieldsWriter<D>,
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
      Self::Asserting(format) => format
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
            .map(StoredFieldsWriterEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_writer(directory, segment_info, context)
        .map(StoredFieldsWriterEnum2::B),
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
  <Lucene90TermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>,
  <AssertingTermVectorsFormat as TermVectorsFormat>::TermVectorsWriter<D>,
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
      Self::Asserting(format) => format
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
            .map(TermVectorsWriterEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .vectors_writer(directory, segment_info, context)
        .map(TermVectorsWriterEnum2::B),
    }
  }
}

#[cfg(not(test))]
pub type CodecNormsConsumer<O> = <Lucene90NormsFormat as NormsFormat>::NormsConsumer<O>;
#[cfg(test)]
pub type CodecNormsConsumer<O> = NormsConsumerEnum2<
  <Lucene90NormsFormat as NormsFormat>::NormsConsumer<O>,
  <AssertingNormsFormat as NormsFormat>::NormsConsumer<O>,
>;

#[cfg(not(test))]
pub type CodecNormsProducer<I> = <Lucene90NormsFormat as NormsFormat>::NormsProducer<I>;
#[cfg(test)]
pub type CodecNormsProducer<I> = NormsProducerEnum2<
  <Lucene90NormsFormat as NormsFormat>::NormsProducer<I>,
  <AssertingNormsFormat as NormsFormat>::NormsProducer<I>,
>;

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
            .map(NormsConsumerEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .norms_consumer(state, segment_info)
        .map(NormsConsumerEnum2::B),
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
    }
  }
}

#[cfg(not(test))]
pub type CodecPointsWriter<O> = <Lucene90PointsFormat as PointsFormat>::PointsWriter<O>;
#[cfg(test)]
pub type CodecPointsWriter<O> = PointsWriterEnum2<
  <Lucene90PointsFormat as PointsFormat>::PointsWriter<O>,
  <AssertingPointsFormat as PointsFormat>::PointsWriter<O>,
>;

#[cfg(not(test))]
pub type CodecPointsReader<I> = <Lucene90PointsFormat as PointsFormat>::PointsReader<I>;
#[cfg(test)]
pub type CodecPointsReader<I> = PointsReaderEnum2<
  <Lucene90PointsFormat as PointsFormat>::PointsReader<I>,
  <AssertingPointsFormat as PointsFormat>::PointsReader<I>,
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
          format.fields_writer(state, info).map(PointsWriterEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format.fields_writer(state, info).map(PointsWriterEnum2::B),
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
          format.fields_reader(state, info).map(PointsReaderEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format.fields_reader(state, info).map(PointsReaderEnum2::B),
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
pub type CodecKnnVectorsReader<I> = KnnVectorsReaderEnum2<
  <Lucene101CodecKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>,
  <AssertingKnnVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>,
>;

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
            .map(KnnVectorsReaderEnum2::A)
        }
      },
      #[cfg(test)]
      Self::Asserting(format) => format
        .fields_reader(state, segment_info)
        .map(KnnVectorsReaderEnum2::B),
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
