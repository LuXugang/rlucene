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
use crate::store::directory::Directory;
use crate::store::{IndexInput, IndexOutput};
use crate::util::bkd::heap_point_write::HeapPointWriter;
use crate::util::bkd::offline_point_write::OfflinePointWriter;
use crate::util::bkd::point_reader::{PointReader, PointReaderEnum};
use crate::util::bkd::point_value::PointValueEnum;
use crate::util::error::lucene_error::Result;

/// Appends many points, and then at the end provides a PointReader to iterate
/// those points. This abstracts away whether we write to disk, or use simple
/// arrays in heap.
pub trait PointWriter {
    /// Add a new point from the packed value and docId
    fn append_bytes(&mut self, packed_value: &[u8], doc_id: i32) -> Result<()>;

    /// Add a new point from a PointValue
    fn append_point_value(&mut self, point_value: &PointValueEnum) -> Result<()>;

    /// Returns a PointReader iterator to step through all previously added
    /// points
    type PointReader<I>: PointReader
    where
        I: IndexInput;
    fn get_reader<D>(
        &mut self,
        start_point: i64,
        length: i64,
        temp_dir: &mut D,
    ) -> Result<Self::PointReader<D::IndexInputType>>
    where
        D: Directory;

    /// Return the number of points in this writer
    fn count(&self) -> i64;

    /// Removes any temp files behind this writer
    fn destroy<D>(&mut self, dir: &mut D) -> Result<()>
    where
        D: Directory;

    fn close(&mut self);
}

pub enum PointWriterEnum<O>
where
    O: IndexOutput,
{
    Heap(HeapPointWriter),
    Offline(OfflinePointWriter<O>),
}
impl<O> Default for PointWriterEnum<O>
where
    O: IndexOutput,
{
    fn default() -> Self {
        PointWriterEnum::Heap(HeapPointWriter::default())
    }
}
impl<O> PointWriterEnum<O>
where
    O: IndexOutput,
{
    pub fn take_data(&mut self, v: Option<PointValueEnum>) {
        match self {
            PointWriterEnum::Offline(_) => {},
            PointWriterEnum::Heap(heap) => heap.take_data(v),
        }
    }
}
impl<O> PointWriter for PointWriterEnum<O>
where
    O: IndexOutput,
{
    fn append_bytes(&mut self, packed_value: &[u8], doc_id: i32) -> Result<()> {
        match self {
            PointWriterEnum::Offline(offline) => offline.append_bytes(packed_value, doc_id),
            PointWriterEnum::Heap(heap) => heap.append_bytes(packed_value, doc_id),
        }
    }

    fn append_point_value(&mut self, point_value: &PointValueEnum) -> Result<()> {
        match self {
            PointWriterEnum::Offline(offline) => offline.append_point_value(point_value),
            PointWriterEnum::Heap(heap) => heap.append_point_value(point_value),
        }
    }

    type PointReader<I>
        = PointReaderEnum<I>
    where
        I: IndexInput;

    fn get_reader<D: Directory>(
        &mut self,
        start_point: i64,
        length: i64,
        temp_dir: &mut D,
    ) -> Result<Self::PointReader<D::IndexInputType>>
    where
        D: Directory,
    {
        match self {
            PointWriterEnum::Offline(offline) => Ok(PointReaderEnum::Offline(offline.get_reader(
                start_point,
                length,
                temp_dir,
            )?)),
            PointWriterEnum::Heap(heap) => Ok(PointReaderEnum::Heap(heap.get_reader(
                start_point,
                length,
                temp_dir,
            )?)),
        }
    }

    fn count(&self) -> i64 {
        match self {
            PointWriterEnum::Offline(offline) => offline.count(),
            PointWriterEnum::Heap(heap) => heap.count(),
        }
    }

    fn destroy<D>(&mut self, dir: &mut D) -> Result<()>
    where
        D: Directory,
    {
        match self {
            PointWriterEnum::Offline(offline) => offline.destroy(dir),
            PointWriterEnum::Heap(heap) => heap.destroy(dir),
        }
    }

    fn close(&mut self) {
        match self {
            PointWriterEnum::Offline(offline) => offline.close(),
            PointWriterEnum::Heap(heap) => heap.close(),
        }
    }
}
