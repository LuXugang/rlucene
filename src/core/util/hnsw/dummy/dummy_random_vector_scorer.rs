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
use crate::core::util::bits::MatchNoBits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::hnsw::random_vector_scorer::RandomVectorScorer;

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
