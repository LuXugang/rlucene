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
use crate::index::dummy::dummy_point_tree::DummyPointTree;
use crate::index::point_values::PointValuesBase;
use crate::util::error::lucene_error::Result;

pub struct DummyPointValuesBase;
impl PointValuesBase for DummyPointValuesBase {
    fn get_min_packed_value(&self) -> Result<Option<Vec<u8>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_max_packed_value(&self) -> Result<Option<Vec<u8>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_num_dimensions(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_num_index_dimensions(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_bytes_per_dimension(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn size(&self) -> Result<i64> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_doc_count(&self) -> Result<i32> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type PointTree = DummyPointTree;

    fn get_point_tree(&self) -> Result<Self::PointTree> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
