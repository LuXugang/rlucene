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
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::index::impacts_enum::ImpactsEnum;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::util::TryIntoInt;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::priority_queue::{Compare, PriorityQueue};

pub struct ExactPhraseMatcher {
    freq: i32,
    upto: i32,
    pos: i32,
}

/// Merge impacts for multiple terms of an exact phrase.
fn merge_impacts<IE>(impacts_enums: Vec<IE>) -> Result<ImpactsSourceImpl<IE>>
where
    IE: ImpactsEnum,
{
    // Iteration of block boundaries uses the impacts enum with the lower cost.
    // This is consistent with BlockMaxConjunctionScorer.
    let mut tmp_lead_index: i32 = -1;
    for i in 0..impacts_enums.len() {
        if tmp_lead_index == -1
            || impacts_enums[i].cost()? < impacts_enums[tmp_lead_index as usize].cost()?
        {
            tmp_lead_index = i as i32;
        }
    }
    let lead_index: usize = tmp_lead_index.try_convert()?;
    Ok(ImpactsSourceImpl::new(impacts_enums, lead_index))
}

pub(crate) struct ImpactsSourceImpl<IE>
where
    IE: ImpactsEnum,
{
    impacts_enums: Vec<IE>,
    lead_index: usize,
}
impl<IE> ImpactsSourceImpl<IE>
where
    IE: ImpactsEnum,
{
    pub fn new(impacts_enums: Vec<IE>, lead_index: usize) -> Self {
        Self {
            impacts_enums,
            lead_index,
        }
    }
}

impl<IE> ImpactsSource for ImpactsSourceImpl<IE>
where
    IE: ImpactsEnum,
{
    fn advance_shallow(&mut self, target: i32) -> Result<()> {
        for impacts_enum in self.impacts_enums.iter_mut() {
            impacts_enum.advance_shallow(target)?;
        }
        Ok(())
    }

    type Impacts<'a>
        = ImpactsImpl<IE::Impacts<'a>>
    where
        Self: 'a;

    fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
        let mut impacts = Vec::with_capacity(self.impacts_enums.len());
        for v in self.impacts_enums.iter() {
            impacts.push(v.get_impacts()?);
        }
        Ok(ImpactsImpl::new(impacts, self.lead_index))
    }
}
pub(crate) struct ImpactsImpl<I>
where
    I: Impacts,
{
    impacts: Vec<I>,
    lead_index: usize,
}

impl<I> ImpactsImpl<I>
where
    I: Impacts,
{
    fn new(impacts: Vec<I>, lead_index: usize) -> Self {
        Self {
            impacts,
            lead_index,
        }
    }
}
fn get_level<I>(impacts: &I, doc_id_up_to: i32) -> i32
where
    I: Impacts,
{
    let num_levels = impacts.num_levels();
    for level in 0..num_levels {
        if impacts.get_doc_id_upto(level) >= doc_id_up_to {
            return level;
        }
    }
    -1
}

impl<I> Impacts for ImpactsImpl<I>
where
    I: Impacts,
{
    fn num_levels(&self) -> i32 {
        // Delegate to the lead
        self.impacts[self.lead_index].num_levels()
    }

    fn get_doc_id_upto(&self, level: i32) -> i32 {
        // Delegate to the lead
        self.impacts[self.lead_index].get_doc_id_upto(level)
    }

    fn get_impacts(&'_ self, level: i32) -> Result<Vec<Impact>> {
        let doc_id_up_to = self.get_doc_id_upto(level);
        let impact_len = self.impacts.len();
        let mut sub_iterators = Vec::new();
        let mut has_impacts = false;
        let mut only_impact_list = false;
        let mut pq = PriorityQueue::new(impact_len, SubIteratorCmp)?;

        for i in 0..impact_len {
            let impacts_level = get_level(&self.impacts[i], doc_id_up_to);
            if impacts_level == -1 {
                // This instance doesn't have useful impacts, ignore it: this is safe.
                continue;
            }

            let impact_list = self.impacts[i].get_impacts(impacts_level)?;
            let first = &impact_list[0];

            if first.freq == i32::MAX && first.norm == 1 {
                // Dummy impacts, ignore it too.
                continue;
            }

            let sub = SubIterator::new(impact_list);
            sub_iterators.push(sub);

            if !has_impacts {
                has_impacts = true;
                only_impact_list = true;
            } else {
                only_impact_list = false;
            }
        }

        if !has_impacts {
            return Ok(vec![Impact::new(i32::MAX, 1)]);
        } else if only_impact_list {
            if sub_iterators.len() != 1 {
                return Err(LuceneError::illegal_state(
                    "only_impact_list is true but there are multiple sub iterators",
                ));
            }
            return match sub_iterators.pop() {
                Some(sub) => Ok(sub.iterator),
                None => Err(LuceneError::illegal_state(
                    "should at least one sub iterator",
                )),
            };
        }
        // Idea: merge impacts by freq. The tricky thing is that we need to
        // consider freq values that are not in the impacts too. For
        // instance if the list of impacts is [{freq=2,norm=10}, {freq=4,norm=12}],
        // there might well be a document that has a freq of 2 and a length of 11,
        // which was just not added to the list of impacts because {freq=2,norm=10}
        // is more competitive.
        // We walk impacts in parallel through a PQ ordered by freq. At any time,
        // the competitive impact consists of the lowest freq among all entries of
        // the PQ (the top) and the highest norm (tracked separately).
        pq.add_all(sub_iterators)?;

        let mut merged_impacts: Vec<Impact> = Vec::new();

        let mut current_freq = {
            let top = pq
                .top()
                .ok_or_else(|| LuceneError::illegal_state("top is None"))?;
            top.current()?.freq
        };

        let mut current_norm = 0;
        for it in pq.iter_ref() {
            let norm = it.current()?.norm;
            if norm as u64 > current_norm as u64 {
                current_norm = norm;
            }
        }
        let mut top = pq
            .top_mut()
            .ok_or_else(|| LuceneError::illegal_state("top is None"))?;

        'outer: loop {
            if let Some(last) = merged_impacts.last_mut() {
                if last.norm == current_norm {
                    last.freq = current_freq;
                } else {
                    merged_impacts.push(Impact::new(current_freq, current_norm));
                }
            } else {
                merged_impacts.push(Impact::new(current_freq, current_norm));
            }

            loop {
                if !top.next() {
                    // At least one clause doesn't have any more documents below the current norm,
                    // so we can safely ignore further clauses. The only reason why they have more
                    // impacts is because they cover more documents that we are not interested in.
                    break 'outer;
                }

                let top_current = top.current()?;
                if top_current.norm as u64 > current_norm as u64 {
                    current_norm = top_current.norm;
                }
                top = pq.update_top()?;

                let impact = top.current()?;
                if impact.freq != current_freq {
                    break;
                }
            }

            current_freq = top.current()?.freq;
        }

        Ok(merged_impacts)
    }
}

struct SubIterator {
    iterator: Vec<Impact>,
    current: Option<usize>,
}
impl SubIterator {
    fn new(impacts: Vec<Impact>) -> Self {
        let current = if impacts.is_empty() { None } else { Some(0) };

        Self {
            iterator: impacts,
            current,
        }
    }
    fn next(&mut self) -> bool {
        match self.current {
            None => false,
            Some(idx) => {
                let next_idx = idx + 1;
                if next_idx >= self.iterator.len() {
                    self.current = None;
                    false
                } else {
                    self.current = Some(next_idx);
                    true
                }
            },
        }
    }
    fn current(&self) -> Result<&Impact> {
        match self.current {
            Some(idx) => Ok(&self.iterator[idx]),
            None => Err(LuceneError::illegal_state("current is None")),
        }
    }
}
#[derive(Default)]
struct SubIteratorCmp;
impl Compare<SubIterator> for SubIteratorCmp {
    fn less_than(&self, a: &SubIterator, b: &SubIterator) -> Result<bool> {
        match (a.current, a.current) {
            (Some(i1), Some(i2)) => Ok(a.iterator[i1].freq < b.iterator[i2].freq),
            _ => Err(LuceneError::illegal_state(
                "one of the iterators is exhausted",
            )),
        }
    }
}
