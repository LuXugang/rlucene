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
use crate::index::dummy::doc_comparator::DummyDocComparator;
use crate::index::index_sorter::IndexSorter;
use crate::index::leaf_reader::LeafReader;

pub struct DummyIndexSorter;
impl IndexSorter for DummyIndexSorter {
    fn get_provider_name(&self) -> &str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type DocComparator = DummyDocComparator;

    fn get_doc_comparator<LR>(
        &mut self,
        _leaf_reader: &mut LR,
        _max_doc: i32,
    ) -> crate::util::error::lucene_error::Result<Self::DocComparator>
    where
        LR: LeafReader,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
