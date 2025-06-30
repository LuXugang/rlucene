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
use crate::codecs::lucene90_points_reader::Lucene90PointsReader;
use crate::index::point_values::{PointValues, PointValuesBase};
use crate::store::IndexInput;
use crate::util::bkd::bkd_reader::BKDReader;
use crate::util::error::lucene_error::Result;
/// Abstract API to visit point values.
pub trait PointsReader {
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&mut self) -> Result<()>;

    type PointValuesBase: PointValuesBase;
    fn get_values(&mut self, field: &str) -> Result<PointValues<Self::PointValuesBase>>;

    /// Returns an instance optimized for merging. This instance may only be
    /// cloned
    /// # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        Ok(None)
    }
}

pub enum PointsReaderEnum<I>
where
    I: IndexInput,
{
    Lucene90(Lucene90PointsReader<I>),
}
impl<I> PointsReader for PointsReaderEnum<I>
where
    I: IndexInput,
{
    fn check_integrity(&mut self) -> Result<()> {
        match self {
            PointsReaderEnum::Lucene90(reader) => reader.check_integrity(),
        }
    }

    type PointValuesBase = BKDReader<I>;

    fn get_values(&mut self, field: &str) -> Result<PointValues<Self::PointValuesBase>> {
        match self {
            PointsReaderEnum::Lucene90(reader) => reader.get_values(field),
        }
    }

    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        match self {
            PointsReaderEnum::Lucene90(reader) => {
                let merge_instance = reader.get_merge_instance()?;
                Ok(merge_instance.map(PointsReaderEnum::Lucene90))
            },
        }
    }
}
