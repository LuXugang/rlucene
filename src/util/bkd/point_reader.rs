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

use crate::store::IndexInput;
use crate::util::bkd::heap_point_reader::HeapPointReader;
use crate::util::bkd::offline_point_reader::OfflinePointReader;
use crate::util::bkd::point_value::PointValueEnum;
use crate::util::error::lucene_error::Result;

/// One-pass iterator through all points previously written with a PointWriter,
/// abstracting away whether points are read from offline disk or from arrays in
/// heap.
pub trait PointReader {
    /// Advances the iterator.
    ///
    /// Returns `Ok(true)` if there is another point available,
    /// or `Ok(false)` if iteration is complete.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if an I/O error occurs during iteration.
    fn next(&mut self) -> Result<bool>;

    /// Returns the current point value.
    fn point_value(&mut self) -> &PointValueEnum;
}

pub enum PointReaderEnum<I>
where
    I: IndexInput,
{
    Offline(OfflinePointReader<I>),
    Heap(HeapPointReader),
}
impl<I> PointReaderEnum<I>
where
    I: IndexInput,
{
    pub fn remove_points(&mut self) -> Option<PointValueEnum> {
        match self {
            PointReaderEnum::Offline(_) => None,
            PointReaderEnum::Heap(heap) => heap.remove_points(),
        }
    }
}
impl<I> PointReader for PointReaderEnum<I>
where
    I: IndexInput,
{
    fn next(&mut self) -> Result<bool> {
        match self {
            PointReaderEnum::Offline(offline) => offline.next(),
            PointReaderEnum::Heap(heap) => heap.next(),
        }
    }

    fn point_value(&mut self) -> &PointValueEnum {
        match self {
            PointReaderEnum::Offline(offline) => offline.point_value(),
            PointReaderEnum::Heap(heap) => heap.point_value(),
        }
    }
}
