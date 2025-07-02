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
use crate::index::binary_doc_values_writer::BinaryDocValuesWriter;
use crate::index::numeric_doc_values_writer::NumericDocValuesWriter;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorted_doc_values_writer::SortedDocValuesWriter;
use crate::index::sorted_numeric_doc_values_writer::SortedNumericDocValuesWriter;
use crate::index::sorted_set_doc_values_writer::SortedSetDocValuesWriter;
use crate::index::sorter::DocMap;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use std::fmt::Display;
use std::rc::Rc;

pub(crate) trait DocValuesWriter: Display {
    fn flush<D, DM, DC>(
        &mut self,
        _state: &SegmentWriteState<D>,
        sort_map: Option<Rc<DM>>,
        dv_consumer: &mut DC,
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
// impl DocValuesWriter for DocValuesWriterEnum {
//     fn flush<D, DM, DC>(
//         &mut self,
//         state: &SegmentWriteState<D>,
//         sort_map: Option<Rc<DM>>,
//         dv_consumer: &mut DC,
//     ) -> Result<()>
//     where
//         D: Directory,
//         DM: DocMap,
//         DC: DocValuesConsumer,
//     {
//         match self {
//             DocValuesWriterEnum::Binary(writer) => writer.flush(state, sort_map, dv_consumer),
//             DocValuesWriterEnum::Numeric(writer) => writer.flush(state, sort_map, dv_consumer),
//             DocValuesWriterEnum::SortedNumeric(writer) => {
//                 writer.flush(state, sort_map, dv_consumer)
//             },
//             DocValuesWriterEnum::Sorted(writer) => writer.flush(state, sort_map, dv_consumer),
//             DocValuesWriterEnum::SortedSet(writer) => writer.flush(state, sort_map, dv_consumer),
//         }
//     }
//
//     type DocIdSetIterator = ();
//
//     fn get_doc_values(&mut self) -> Result<Self::DocIdSetIterator> {
//         match self {
//             DocValuesWriterEnum::Binary(writer) => writer.get_doc_values(),
//             DocValuesWriterEnum::Numeric(writer) => writer.get_doc_values(),
//             DocValuesWriterEnum::SortedNumeric(writer) => writer.get_doc_values(),
//             DocValuesWriterEnum::Sorted(writer) => writer.get_doc_values(),
//             DocValuesWriterEnum::SortedSet(writer) => writer.get_doc_values(),
//         }
//     }
// }
