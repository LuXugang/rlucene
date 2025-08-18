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
use crate::index::docs_with_field_set::DocsWithFieldSetEnum;
use crate::index::numeric_doc_values_writer::{BufferedNumericDocValues, NumericDocValuesWriter};
use crate::index::segment_info::SegmentInfo;
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::singleton_sorted_set_doc_values::SingletonSortedSetDocValues;
use crate::index::sorted_doc_values_writer::{BufferedSortedDocValues, SortedDocValuesWriter};
use crate::index::sorted_numeric_doc_values::EitherSortedNumericDocValues;
use crate::index::sorted_numeric_doc_values_writer::{
    BufferedSortedNumericDocValues, SortedNumericDocValuesWriter,
};
use crate::index::sorted_set_doc_values_writer::EitherSortedSetDocValues;
use crate::index::sorted_set_doc_values_writer::{
    BufferedSortedSetDocValues, SortedSetDocValuesWriter,
};
use crate::index::sorter::DocMap;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
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
    fn get_doc_values(&mut self) -> Result<Self::DocIdSetIterator>;
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

    type DocIdSetIterator = DocIdSetIteratorImpl;

    fn get_doc_values(&mut self) -> Result<Self::DocIdSetIterator> {
        match self {
            DocValuesWriterEnum::Binary(writer) => {
                Ok(DocIdSetIteratorImpl::Binary(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::Numeric(writer) => {
                Ok(DocIdSetIteratorImpl::Numeric(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::SortedNumeric(writer) => Ok(DocIdSetIteratorImpl::SortedNumeric(
                writer.get_doc_values()?,
            )),
            DocValuesWriterEnum::Sorted(writer) => {
                Ok(DocIdSetIteratorImpl::Sorted(writer.get_doc_values()?))
            },
            DocValuesWriterEnum::SortedSet(writer) => {
                Ok(DocIdSetIteratorImpl::SortedSet(writer.get_doc_values()?))
            },
        }
    }
}

pub(crate) enum DocIdSetIteratorImpl {
    Binary(BufferedBinaryDocValues<DocsWithFieldSetEnum, PagedBytesDataInput>),
    Numeric(BufferedNumericDocValues),
    SortedNumeric(
        EitherSortedNumericDocValues<
            SingletonSortedNumericDocValues<BufferedNumericDocValues>,
            BufferedSortedNumericDocValues<DocsWithFieldSetEnum>,
        >,
    ),
    Sorted(BufferedSortedDocValues<DocsWithFieldSetEnum>),
    SortedSet(
        EitherSortedSetDocValues<
            SingletonSortedSetDocValues<BufferedSortedDocValues<DocsWithFieldSetEnum>>,
            BufferedSortedSetDocValues<DocsWithFieldSetEnum>,
        >,
    ),
}
impl DocIdSetIterator for DocIdSetIteratorImpl {
    fn doc_id(&self) -> i32 {
        match self {
            DocIdSetIteratorImpl::Binary(iter) => iter.doc_id(),
            DocIdSetIteratorImpl::Numeric(iter) => iter.doc_id(),
            DocIdSetIteratorImpl::SortedNumeric(iter) => iter.doc_id(),
            DocIdSetIteratorImpl::Sorted(iter) => iter.doc_id(),
            DocIdSetIteratorImpl::SortedSet(iter) => iter.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            DocIdSetIteratorImpl::Binary(iter) => iter.next_doc(),
            DocIdSetIteratorImpl::Numeric(iter) => iter.next_doc(),
            DocIdSetIteratorImpl::SortedNumeric(iter) => iter.next_doc(),
            DocIdSetIteratorImpl::Sorted(iter) => iter.next_doc(),
            DocIdSetIteratorImpl::SortedSet(iter) => iter.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            DocIdSetIteratorImpl::Binary(iter) => iter.advance(_target),
            DocIdSetIteratorImpl::Numeric(iter) => iter.advance(_target),
            DocIdSetIteratorImpl::SortedNumeric(iter) => iter.advance(_target),
            DocIdSetIteratorImpl::Sorted(iter) => iter.advance(_target),
            DocIdSetIteratorImpl::SortedSet(iter) => iter.advance(_target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            DocIdSetIteratorImpl::Binary(iter) => iter.slow_advance(target),
            DocIdSetIteratorImpl::Numeric(iter) => iter.slow_advance(target),
            DocIdSetIteratorImpl::SortedNumeric(iter) => iter.slow_advance(target),
            DocIdSetIteratorImpl::Sorted(iter) => iter.slow_advance(target),
            DocIdSetIteratorImpl::SortedSet(iter) => iter.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            DocIdSetIteratorImpl::Binary(iter) => iter.cost(),
            DocIdSetIteratorImpl::Numeric(iter) => iter.cost(),
            DocIdSetIteratorImpl::SortedNumeric(iter) => iter.cost(),
            DocIdSetIteratorImpl::Sorted(iter) => iter.cost(),
            DocIdSetIteratorImpl::SortedSet(iter) => iter.cost(),
        }
    }
}
