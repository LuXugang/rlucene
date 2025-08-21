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
use crate::codecs::doc_values_consumer::DocValuesConsumer;
use crate::index::binary_doc_values_writer::{BinaryDocValuesWriter, BufferedBinaryDocValues};
use crate::index::docs_with_field_set::DocsWithFieldSetDISI;
use crate::index::numeric_doc_values_writer::{BufferedNumericDocValues, NumericDocValuesWriter};
use crate::index::segment_info::SegmentInfo;
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::index::sorted_doc_values_writer::{BufferedSortedDocValues, SortedDocValuesWriter};
use crate::index::sorted_numeric_doc_values::Either2SortedNumericDocValues;
use crate::index::sorted_numeric_doc_values_writer::{
    BufferedSortedNumericDocValues, SortedNumericDocValuesWriter,
};
use crate::index::sorted_set_doc_values_writer::Either2SortedSetDocValues;
use crate::index::sorted_set_doc_values_writer::{
    BufferedSortedSetDocValues, SortedSetDocValuesWriter,
};
use crate::index::sorter::DocMap;
use crate::search::doc_id_set_iterator::{DocIdSetIterator, Either5DocIdSetIterator};
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use crate::util::paged_bytes::PagedBytesDataInput;
use std::fmt::Display;
use std::rc::Rc;

pub(crate) trait DocValuesWriter: Display {
    fn flush<D, DM, DC>(
        &mut self,
        sort_map: Option<Rc<DM>>,
        dv_consumer: &mut DC,
        segment_info: &SegmentInfo<D>,
    ) -> Result<()>
    where
        D: Directory,
        DM: DocMap,
        DC: DocValuesConsumer;

    type DocIdSetIterator: DocIdSetIterator;
    fn get_doc_values(&self) -> Result<Self::DocIdSetIterator>;
    fn finish(&mut self) -> Result<()>;
}

pub(crate) enum DocValuesWriterEnum {
    Binary(BinaryDocValuesWriter),
    Numeric(NumericDocValuesWriter),
    SortedNumeric(SortedNumericDocValuesWriter),
    Sorted(SortedDocValuesWriter),
    SortedSet(SortedSetDocValuesWriter),
}
impl Display for DocValuesWriterEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocValuesWriterEnum::Binary(writer) => writer.fmt(f),
            DocValuesWriterEnum::Numeric(writer) => writer.fmt(f),
            DocValuesWriterEnum::SortedNumeric(writer) => writer.fmt(f),
            DocValuesWriterEnum::Sorted(writer) => writer.fmt(f),
            DocValuesWriterEnum::SortedSet(writer) => writer.fmt(f),
        }
    }
}
impl DocValuesWriter for DocValuesWriterEnum {
    fn flush<D, DM, DC>(
        &mut self,
        sort_map: Option<Rc<DM>>,
        dv_consumer: &mut DC,
        segment_info: &SegmentInfo<D>,
    ) -> Result<()>
    where
        D: Directory,
        DM: DocMap,
        DC: DocValuesConsumer,
    {
        match self {
            DocValuesWriterEnum::Binary(writer) => {
                writer.flush(sort_map, dv_consumer, segment_info)
            },
            DocValuesWriterEnum::Numeric(writer) => {
                writer.flush(sort_map, dv_consumer, segment_info)
            },
            DocValuesWriterEnum::SortedNumeric(writer) => {
                writer.flush(sort_map, dv_consumer, segment_info)
            },
            DocValuesWriterEnum::Sorted(writer) => {
                writer.flush(sort_map, dv_consumer, segment_info)
            },
            DocValuesWriterEnum::SortedSet(writer) => {
                writer.flush(sort_map, dv_consumer, segment_info)
            },
        }
    }

    type DocIdSetIterator = DocValuesWriterDISI;

    fn get_doc_values(&self) -> Result<Self::DocIdSetIterator> {
        match self {
            DocValuesWriterEnum::Binary(writer) => {
                Ok(Either5DocIdSetIterator::F(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::Numeric(writer) => {
                Ok(Either5DocIdSetIterator::S(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::SortedNumeric(writer) => {
                Ok(Either5DocIdSetIterator::T(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::Sorted(writer) => {
                Ok(Either5DocIdSetIterator::U(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::SortedSet(writer) => {
                Ok(Either5DocIdSetIterator::V(writer.get_doc_values()?))
            },
        }
    }

    fn finish(&mut self) -> Result<()> {
        match self {
            DocValuesWriterEnum::Binary(writer) => writer.finish(),
            DocValuesWriterEnum::Numeric(writer) => writer.finish(),
            DocValuesWriterEnum::SortedNumeric(writer) => writer.finish(),
            DocValuesWriterEnum::Sorted(writer) => writer.finish(),
            DocValuesWriterEnum::SortedSet(writer) => writer.finish(),
        }
    }
}
pub(crate) type DocValuesWriterDISI = Either5DocIdSetIterator<
    BufferedBinaryDocValues<DocsWithFieldSetDISI, PagedBytesDataInput>,
    BufferedNumericDocValues,
    Either2SortedNumericDocValues<
        SingletonSortedNumericDocValues<BufferedNumericDocValues>,
        BufferedSortedNumericDocValues<DocsWithFieldSetDISI>,
    >,
    BufferedSortedDocValues<DocsWithFieldSetDISI>,
    Either2SortedSetDocValues<
        SingletonSortedSetDocValues<BufferedSortedDocValues<DocsWithFieldSetDISI>>,
        BufferedSortedSetDocValues<DocsWithFieldSetDISI>,
    >,
>;
