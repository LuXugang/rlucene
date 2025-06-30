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
use crate::util::bits::MatchNoBits;
use crate::util::error::lucene_error::Result;
use crate::util::hnsw::random_vector_scorer::RandomVectorScorer;

#[derive(Default)]
pub struct DummyRandomVectorScorer;
impl RandomVectorScorer for DummyRandomVectorScorer {
    fn score(&self, _node: i32) -> Result<f32> {
        Ok(0f32)
    }

    fn max_ord(&self) -> i32 {
        0
    }

    fn ord_to_doc(&self, _ord: i32) -> i32 {
        0
    }

    type Bits = MatchNoBits;
    type BitsR = MatchNoBits;

    fn get_accept_ords(&self, _accept_docs: Self::Bits) -> Self::Bits {
        MatchNoBits::default()
    }
}
