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
use crate::core::codecs::hnsw::flat_vectors_scorer::FlatVectorsScorer;
use std::marker::PhantomData;

pub(crate) const NAME: &str = "Lucene99FlatVectorsFormat";
pub(crate) const META_CODEC_NAME: &str = "Lucene99FlatVectorsFormatMeta";
pub(crate) const VECTOR_DATA_CODEC_NAME: &str = "Lucene99FlatVectorsFormatData";
pub(crate) const META_EXTENSION: &str = "vemf";
pub(crate) const VECTOR_DATA_EXTENSION: &str = "vec";

pub(crate) const VERSION_START: i32 = 0;
pub(crate) const VERSION_CURRENT: i32 = VERSION_START;

pub(crate) const DIRECT_MONOTONIC_BLOCK_SHIFT: i32 = 16;
pub struct Lucene99FlatVectorsFormat<F, V>
where
  F: FlatVectorsScorer,
  V: Clone,
{
  vectors_scorer: F,
  _marker: PhantomData<V>,
}
impl<F, V> Lucene99FlatVectorsFormat<F, V>
where
  F: FlatVectorsScorer + Clone,
  V: Clone,
{
  pub fn new(vectors_scorer: F) -> Self {
    Self {
      vectors_scorer,
      _marker: PhantomData,
    }
  }
}

// impl<F,V> KnnVectorsFormat for Lucene99FlatVectorsFormat<F,V>
// where
//   F: Clone + FlatVectorsScorer,
//     V:Clone,
// {
//   type KnnVectorsWriter<T: IndexInput> = ();
//
//   fn fields_writer<D1, D2>(
//     &self,
//     state: &SegmentWriteState<D1>,
//     segment_info: &SegmentInfo<D2>,
//     field_infos: Arc<FieldInfos>,
//     context: &IOContext,
//   ) -> Result<Self::KnnVectorsWriter<D1::IndexInput>>
//   where
//     D1: Directory,
//     D2: Directory,
//   {
//     todo!()
//   }
//
//   type KnnVectorsReader<T: IndexOutput> = ();
//
//   fn fields_reader<D1, D2>(
//     &self,
//     state: &SegmentReadState<D1>,
//     segment_info: &mut SegmentInfo<D2>,
//   ) -> Result<Self::KnnVectorsReader<D1::IndexOutput>>
//   where
//     D1: Directory,
//     D2: Directory,
//   {
//     todo!()
//   }
//
//   fn get_max_dimensions(&self, field_name: &str) -> usize {
//     todo!()
//   }
// }
//
// impl<F,V> FlatVectorsFormat for Lucene99FlatVectorsFormat<F,V>
// where
//   F: FlatVectorsScorer + Clone,
//     V:Clone,
// {
//   type FlatVectorsWriter<T: IndexOutput> = Lucene99FlatVectorsWriter<T, F, V>;
//
//   fn fields_writer<D1, D2>(
//     &self,
//     state: &SegmentWriteState<D1>,
//     segment_info: &SegmentInfo<D2>,
//   ) -> Result<Self::FlatVectorsWriter<D1::IndexOutput>>
//   where
//     D1: Directory,
//     D2: Directory,
//   {
//     let v = Lucene99FlatVectorsWriter::new(state, self.vectors_scorer.clone(), segment_info)?;
//     todo!()
//   }
//
//   type FlatVectorsReader<T: IndexInput> = ();
//
//   fn fields_reader<D1, D2>(
//     &self,
//     state: &SegmentReadState<D1>,
//     segment_info: &mut SegmentInfo<D2>,
//   ) -> Result<Self::FlatVectorsReader<D1::IndexInput>>
//   where
//     D1: Directory,
//     D2: Directory,
//   {
//     todo!()
//   }
// }
