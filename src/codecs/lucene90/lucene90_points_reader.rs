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
use crate::codecs::points_reader::PointsReader;
use crate::index::point_values::PointValues;
use crate::index::segment_read_state::SegmentReadState;
use crate::store::directory::Directory;
use crate::store::IndexInput;
use crate::util::bkd::bkd_reader::BKDReader;
use crate::util::error::lucene_error::Result;

pub struct Lucene90PointsReader<I>
where
    I: IndexInput,
{
    // TODO 填充值
    input: I,
}

impl<I> Lucene90PointsReader<I>
where
    I: IndexInput,
{
    pub fn new<D>(_state: &SegmentReadState<D>) -> Self
    where
        D: Directory,
    {
        todo!()
    }
}

impl<I> PointsReader for Lucene90PointsReader<I>
where
    I: IndexInput,
{
    fn check_integrity(&mut self) -> Result<()> {
        todo!()
    }

    type PointValuesBase = BKDReader<I>;

    fn get_values(&mut self, _field: &str) -> Result<PointValues<Self::PointValuesBase>> {
        todo!()
    }
}
