/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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
