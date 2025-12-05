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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::collection_util::CollectionUtil;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{Comparator, ToInt};

pub struct ConjunctionDISI<D>
where
    D: DocIdSetIterator,
{
    lead1: D,
    lead2: D,
    others: Vec<D>,
}
impl<D> ConjunctionDISI<D>
where
    D: DocIdSetIterator,
{
    fn new(iterators: Vec<D>) -> Result<Self> {
        debug_assert!(iterators.len() >= 2);
        let mut cost = Vec::with_capacity(iterators.len());
        let mut temp_iterators = Vec::with_capacity(iterators.len());
        for (idx, v) in iterators.into_iter().enumerate() {
            cost.push(idx);
            temp_iterators.push(Some(v));
        }
        let cmp = DisiCmp::new(temp_iterators.as_ref());
        CollectionUtil::tim_sort_with_comparator(&mut cost, cmp)?;
        let mut iters = Vec::with_capacity(temp_iterators.len());
        for idx in cost {
            iters.push(temp_iterators[idx].take().unwrap());
        }
        let lead1 = iters.remove(0);
        let lead2 = iters.remove(0);
        Ok(Self {
            lead1,
            lead2,
            others: iters,
        })
    }
    fn do_next(&mut self, mut doc: i32) -> Result<i32> {
        'advance_head: loop {
            debug_assert_eq!(doc, self.lead1.doc_id());
            // find agreement between the two iterators with the lower costs
            // we special case them because they do not need the
            // 'other.docID() < doc' check that the 'others' iterators need
            let next2 = self.lead2.advance(doc)?;
            if next2 != doc {
                doc = self.lead1.advance(next2)?;
                if doc != next2 {
                    continue;
                }
            }
            // then find agreement with other iterators
            for other in &mut self.others {
                let other_doc = other.doc_id();
                // other.doc may already be equal to doc if we "continued advanceHead"
                // on the previous iteration and the advance on the lead scorer exactly matched.
                if other_doc < doc {
                    let next = other.advance(doc)?;

                    if next > doc {
                        // iterator beyond the current doc - advance lead and continue to the new highest doc.
                        doc = self.lead1.advance(next)?;
                        continue 'advance_head;
                    }
                }
            }
            return Ok(doc);
        }
    }
    // Returns {@code true} if all sub-iterators are on the same doc ID, {@code false} otherwise
    fn assert_iters_on_same_doc(&self) -> bool {
        let cur_doc = self.lead1.doc_id();
        let mut iterators_on_the_same_doc = self.lead2.doc_id() == cur_doc;
        let mut i = 0;
        while i < self.others.len() && iterators_on_the_same_doc {
            iterators_on_the_same_doc =
                iterators_on_the_same_doc && (self.others[i].doc_id() == cur_doc);
            i += 1;
        }
        iterators_on_the_same_doc
    }
}
impl<D> DocIdSetIterator for ConjunctionDISI<D>
where
    D: DocIdSetIterator,
{
    fn doc_id(&self) -> i32 {
        self.lead1.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        debug_assert!(
            self.assert_iters_on_same_doc(),
            "Sub-iterators of ConjunctionDISI are not on the same document!"
        );
        let doc = self.lead1.next_doc()?;
        self.do_next(doc)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        debug_assert!(
            self.assert_iters_on_same_doc(),
            "Sub-iterators of ConjunctionDISI are not on the same document!"
        );
        let doc = self.lead1.advance(target)?;
        self.do_next(doc)
    }

    fn cost(&self) -> Result<i64> {
        self.lead1.cost()
    }
}

struct DisiCmp<'a, D>
where
    D: DocIdSetIterator,
{
    disi: &'a [Option<D>],
}
impl<'a, D> DisiCmp<'a, D>
where
    D: DocIdSetIterator,
{
    fn new(disi: &'a [Option<D>]) -> Self {
        DisiCmp { disi }
    }
}
impl<D> Comparator<usize> for DisiCmp<'_, D>
where
    D: DocIdSetIterator,
{
    const TYPE: &'static str = "DisiCmp";

    fn compare(&self, a: &usize, b: &usize) -> Result<i32> {
        Ok(self.disi[*a]
            .as_ref()
            .unwrap()
            .cost()?
            .cmp(&self.disi[*b].as_ref().unwrap().cost()?)
            .to_int())
    }
}
