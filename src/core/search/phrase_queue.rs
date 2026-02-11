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
use crate::core::search::phrase_positions::PhrasePositions;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::priority_queue::{Compare, PriorityQueue};

pub(crate) struct PhraseQueue;
impl PhraseQueue {
    pub(crate) fn new(
        pp: Vec<PhrasePositions>,
        size: usize,
    ) -> Result<PriorityQueue<PhrasePositions, PhraseQueueCmp>> {
        let cmp = PhraseQueueCmp::new(pp);
        PriorityQueue::new(size, cmp)
    }
}

pub(crate) struct PhraseQueueCmp {
    pub(crate) pp: Vec<PhrasePositions>,
}
impl PhraseQueueCmp {
    pub(crate) fn new(pp: Vec<PhrasePositions>) -> Self {
        Self { pp }
    }
}
impl Compare<PhrasePositions> for PhraseQueueCmp {
    fn less_than(
        &self,
        pp1: &PhrasePositions,
        pp2: &PhrasePositions,
    ) -> Result<bool> {
        if pp1.position == pp2.position {
            // same doc and pp.position, so decide by actual term positions.
            // rely on: pp.position == tp.position - offset.
            if pp1.offset == pp2.offset {
                Ok(pp1.ord < pp2.ord)
            } else {
                Ok(pp1.offset < pp2.offset)
            }
        } else {
            Ok(pp1.position < pp2.position)
        }
    }
}
