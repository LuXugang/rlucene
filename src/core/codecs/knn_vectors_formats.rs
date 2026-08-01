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
use crate::codec::bitvectors::hnsw_bit_vectors_format::{
  HnswBitVectorsFormat, NAME as HNSW_BIT_VECTORS_FORMAT_NAME,
};
use crate::core::codecs::knn_vectors_format::KnnVectorsFormat;
use crate::core::codecs::knn_vectors_reader::KnnVectorsReaderEnum2;
use crate::core::codecs::knn_vectors_writer::KnnVectorsWriterEnum2;
use crate::core::codecs::lucene99::lucene99_hnsw_scalar_quantized_vectors_format::{
  Lucene99HnswScalarQuantizedVectorsFormat,
  NAME as LUCENE99_HNSW_SCALAR_QUANTIZED_VECTORS_FORMAT_NAME,
};
use crate::core::codecs::lucene99::lucene99_hnsw_vectors_format::Lucene99HnswVectorsFormat;
use crate::core::codecs::lucene99::lucene99_scalar_quantized_vectors_format::{
  Lucene99ScalarQuantizedVectorsFormat, NAME as LUCENE99_SCALAR_QUANTIZED_VECTORS_FORMAT_NAME,
};
use crate::core::index::index_reader::Identity;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

const LUCENE99_HNSW_VECTORS_FORMAT_NAME: &str = "Lucene99HnswVectorsFormat";

/// Static-dispatch registry for the production [`KnnVectorsFormat`] implementations.
#[derive(Clone)]
pub enum KnnVectorsFormats {
  Lucene99Hnsw(Arc<Lucene99HnswVectorsFormat>),
  Lucene99ScalarQuantized(Arc<Lucene99ScalarQuantizedVectorsFormat>),
  Lucene99HnswScalarQuantized(Arc<Lucene99HnswScalarQuantizedVectorsFormat>),
  HnswBit(Arc<HnswBitVectorsFormat>),
}

impl From<Lucene99HnswVectorsFormat> for KnnVectorsFormats {
  fn from(format: Lucene99HnswVectorsFormat) -> Self {
    Self::Lucene99Hnsw(Arc::new(format))
  }
}

impl From<Lucene99ScalarQuantizedVectorsFormat> for KnnVectorsFormats {
  fn from(format: Lucene99ScalarQuantizedVectorsFormat) -> Self {
    Self::Lucene99ScalarQuantized(Arc::new(format))
  }
}

impl From<Lucene99HnswScalarQuantizedVectorsFormat> for KnnVectorsFormats {
  fn from(format: Lucene99HnswScalarQuantizedVectorsFormat) -> Self {
    Self::Lucene99HnswScalarQuantized(Arc::new(format))
  }
}

impl From<HnswBitVectorsFormat> for KnnVectorsFormats {
  fn from(format: HnswBitVectorsFormat) -> Self {
    Self::HnswBit(Arc::new(format))
  }
}

impl Display for KnnVectorsFormats {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Lucene99Hnsw(format) => Display::fmt(format.as_ref(), f),
      Self::Lucene99ScalarQuantized(format) => Display::fmt(format.as_ref(), f),
      Self::Lucene99HnswScalarQuantized(format) => Display::fmt(format.as_ref(), f),
      Self::HnswBit(format) => Display::fmt(format.as_ref(), f),
    }
  }
}

impl HasIdentity for KnnVectorsFormats {
  fn identity(&self) -> &Identity {
    match self {
      Self::Lucene99Hnsw(format) => format.identity(),
      Self::Lucene99ScalarQuantized(format) => format.identity(),
      Self::Lucene99HnswScalarQuantized(format) => format.identity(),
      Self::HnswBit(format) => format.identity(),
    }
  }
}

type Lucene99HnswVectorsWriter<O> =
  <Lucene99HnswVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>;
type Lucene99ScalarQuantizedVectorsWriter<O> =
  <Lucene99ScalarQuantizedVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>;
type Lucene99HnswScalarQuantizedVectorsWriter<O> =
  <Lucene99HnswScalarQuantizedVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>;
type HnswBitVectorsWriter<O> = <HnswBitVectorsFormat as KnnVectorsFormat>::KnnVectorsWriter<O>;

pub type KnnVectorsFormatsWriter<O> = KnnVectorsWriterEnum2<
  KnnVectorsWriterEnum2<Lucene99HnswVectorsWriter<O>, Lucene99ScalarQuantizedVectorsWriter<O>>,
  KnnVectorsWriterEnum2<Lucene99HnswScalarQuantizedVectorsWriter<O>, HnswBitVectorsWriter<O>>,
>;

type Lucene99HnswVectorsReader<I> =
  <Lucene99HnswVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>;
type Lucene99ScalarQuantizedVectorsReader<I> =
  <Lucene99ScalarQuantizedVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>;
type Lucene99HnswScalarQuantizedVectorsReader<I> =
  <Lucene99HnswScalarQuantizedVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>;
type HnswBitVectorsReader<I> = <HnswBitVectorsFormat as KnnVectorsFormat>::KnnVectorsReader<I>;

pub type KnnVectorsFormatsReader<I> = KnnVectorsReaderEnum2<
  KnnVectorsReaderEnum2<Lucene99HnswVectorsReader<I>, Lucene99ScalarQuantizedVectorsReader<I>>,
  KnnVectorsReaderEnum2<Lucene99HnswScalarQuantizedVectorsReader<I>, HnswBitVectorsReader<I>>,
>;

impl KnnVectorsFormat for KnnVectorsFormats {
  fn get_name(&self) -> &str {
    match self {
      Self::Lucene99Hnsw(format) => format.get_name(),
      Self::Lucene99ScalarQuantized(format) => KnnVectorsFormat::get_name(format.as_ref()),
      Self::Lucene99HnswScalarQuantized(format) => format.get_name(),
      Self::HnswBit(format) => format.get_name(),
    }
  }

  type KnnVectorsWriter<O: IndexOutput> = KnnVectorsFormatsWriter<O>;

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
      Self::Lucene99Hnsw(format) => {
        KnnVectorsFormat::fields_writer(format.as_ref(), state, segment_info)
          .map(|writer| KnnVectorsWriterEnum2::A(KnnVectorsWriterEnum2::A(writer)))
      },
      Self::Lucene99ScalarQuantized(format) => {
        KnnVectorsFormat::fields_writer(format.as_ref(), state, segment_info)
          .map(|writer| KnnVectorsWriterEnum2::A(KnnVectorsWriterEnum2::B(writer)))
      },
      Self::Lucene99HnswScalarQuantized(format) => {
        KnnVectorsFormat::fields_writer(format.as_ref(), state, segment_info)
          .map(|writer| KnnVectorsWriterEnum2::B(KnnVectorsWriterEnum2::A(writer)))
      },
      Self::HnswBit(format) => {
        KnnVectorsFormat::fields_writer(format.as_ref(), state, segment_info)
          .map(|writer| KnnVectorsWriterEnum2::B(KnnVectorsWriterEnum2::B(writer)))
      },
    }
  }

  type KnnVectorsReader<I: IndexInput> = KnnVectorsFormatsReader<I>;

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
      Self::Lucene99Hnsw(format) => {
        KnnVectorsFormat::fields_reader(format.as_ref(), state, segment_info)
          .map(|reader| KnnVectorsReaderEnum2::A(KnnVectorsReaderEnum2::A(reader)))
      },
      Self::Lucene99ScalarQuantized(format) => {
        KnnVectorsFormat::fields_reader(format.as_ref(), state, segment_info)
          .map(|reader| KnnVectorsReaderEnum2::A(KnnVectorsReaderEnum2::B(reader)))
      },
      Self::Lucene99HnswScalarQuantized(format) => {
        KnnVectorsFormat::fields_reader(format.as_ref(), state, segment_info)
          .map(|reader| KnnVectorsReaderEnum2::B(KnnVectorsReaderEnum2::A(reader)))
      },
      Self::HnswBit(format) => {
        KnnVectorsFormat::fields_reader(format.as_ref(), state, segment_info)
          .map(|reader| KnnVectorsReaderEnum2::B(KnnVectorsReaderEnum2::B(reader)))
      },
    }
  }

  fn get_max_dimensions(&self, field_name: &str) -> Result<usize> {
    match self {
      Self::Lucene99Hnsw(format) => format.get_max_dimensions(field_name),
      Self::Lucene99ScalarQuantized(format) => {
        KnnVectorsFormat::get_max_dimensions(format.as_ref(), field_name)
      },
      Self::Lucene99HnswScalarQuantized(format) => format.get_max_dimensions(field_name),
      Self::HnswBit(format) => format.get_max_dimensions(field_name),
    }
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    match name {
      LUCENE99_HNSW_VECTORS_FORMAT_NAME => {
        Lucene99HnswVectorsFormat::for_name(name).map(|format| Arc::new(Self::Lucene99Hnsw(format)))
      },
      LUCENE99_SCALAR_QUANTIZED_VECTORS_FORMAT_NAME => {
        Lucene99ScalarQuantizedVectorsFormat::for_name(name)
          .map(|format| Arc::new(Self::Lucene99ScalarQuantized(format)))
      },
      LUCENE99_HNSW_SCALAR_QUANTIZED_VECTORS_FORMAT_NAME => {
        Lucene99HnswScalarQuantizedVectorsFormat::for_name(name)
          .map(|format| Arc::new(Self::Lucene99HnswScalarQuantized(format)))
      },
      HNSW_BIT_VECTORS_FORMAT_NAME => {
        HnswBitVectorsFormat::for_name(name).map(|format| Arc::new(Self::HnswBit(format)))
      },
      _ => Err(LuceneError::illegal_argument(format!(
        "Could not load vectors format named \"{name}\""
      ))),
    }
  }
}
