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
use crate::core::codecs::term_vectors_format::TermVectorsFormat;
use crate::core::codecs::term_vectors_writer::{TermVectorsWriter, TermVectorsWriterDefaults};
use crate::core::index::BytesRef;
use crate::core::index::codec_reader::CodecReader;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::merge_state::MergeState;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, IOContext, IndexInput};
use crate::core::util::accountable::Accountable;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::io::Error;
use std::sync::Arc;

pub struct CrankyTermVectorsFormat<TVF> {
  delegate: TVF,
  random: Arc<Mutex<StdRng>>,
}

impl<TVF> CrankyTermVectorsFormat<TVF> {
  pub fn new(delegate: TVF, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<TVF> TermVectorsFormat for CrankyTermVectorsFormat<TVF>
where
  TVF: TermVectorsFormat,
{
  type TermVectorsReader<T: IndexInput> = TVF::TermVectorsReader<T>;

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
    self
      .delegate
      .vectors_reader(directory, segment_info, field_infos, context)
  }

  type TermVectorsWriter<D: Directory> = CrankyTermVectorsWriter<TVF::TermVectorsWriter<D>>;

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
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsFormat.vectorsWriter()",
      )));
    }
    Ok(CrankyTermVectorsWriter::new(
      self
        .delegate
        .vectors_writer(directory, segment_info, context)?,
      Arc::clone(&self.random),
    ))
  }
}

pub struct CrankyTermVectorsWriter<TVW> {
  delegate: TVW,
  random: Arc<Mutex<StdRng>>,
}

impl<TVW> CrankyTermVectorsWriter<TVW> {
  fn new(delegate: TVW, random: Arc<Mutex<StdRng>>) -> Self {
    Self { delegate, random }
  }
}

impl<TVW> TermVectorsWriter for CrankyTermVectorsWriter<TVW>
where
  TVW: TermVectorsWriter,
{
  fn start_document(&mut self, num_vector_fields: i32) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.startDocument()",
      )));
    }
    self.delegate.start_document(num_vector_fields)
  }

  fn finish_document(&mut self) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.finishDocument()",
      )));
    }
    self.delegate.finish_document()
  }

  fn start_field(
    &mut self,
    field_info: &FieldInfo,
    num_terms: usize,
    positions: bool,
    offsets: bool,
    payloads: bool,
  ) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.startField()",
      )));
    }
    self
      .delegate
      .start_field(field_info, num_terms, positions, offsets, payloads)
  }

  fn finish_field(&mut self) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.finishField()",
      )));
    }
    self.delegate.finish_field()
  }

  fn start_term(&mut self, term: &BytesRef<Vec<u8>>, freq: i32) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.startTerm()",
      )));
    }
    self.delegate.start_term(term, freq)
  }

  fn finish_term(&mut self) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.finishTerm()",
      )));
    }
    self.delegate.finish_term()
  }

  fn add_position(
    &mut self,
    position: i32,
    start_offset: i32,
    end_offset: i32,
    payload: Option<&BytesRef<Vec<u8>>>,
  ) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.addPosition()",
      )));
    }
    self
      .delegate
      .add_position(position, start_offset, end_offset, payload)
  }

  fn finish(&mut self, num_docs: i32) -> Result<()> {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.finish()",
      )));
    }
    self.delegate.finish(num_docs)
  }

  fn add_prox(
    &mut self,
    num_prox: usize,
    positions: Option<&mut impl DataInput>,
    offsets: Option<&mut impl DataInput>,
  ) -> Result<()> {
    if self.random.lock().random_range(0..10000) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.addProx()",
      )));
    }
    TermVectorsWriterDefaults::add_prox(self, num_prox, positions, offsets)
  }

  fn merge<D, CR>(&mut self, merge_state: &mut MergeState<D, CR>) -> Result<i32>
  where
    D: Directory,
    CR: CodecReader,
  {
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.merge()",
      )));
    }
    TermVectorsWriterDefaults::merge(self, merge_state)
  }
}

impl<TVW> Closeable for CrankyTermVectorsWriter<TVW>
where
  TVW: TermVectorsWriter,
{
  fn close(&mut self) -> Result<()> {
    self.delegate.close()?;
    if self.random.lock().random_range(0..100) == 0 {
      return Err(LuceneError::io(Error::other(
        "Fake IOException from TermVectorsWriter.close()",
      )));
    }
    Ok(())
  }
}

impl<TVW> Accountable for CrankyTermVectorsWriter<TVW>
where
  TVW: TermVectorsWriter,
{
  fn ram_bytes_used(&self) -> Result<i64> {
    self.delegate.ram_bytes_used()
  }
}
