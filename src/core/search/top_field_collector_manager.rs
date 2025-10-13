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
use crate::core::index::sort::Sort;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::field_comparator::FieldComparator;
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::field_value_hit_queue::create;
use crate::core::search::max_score_accumulator::MaxScoreAccumulator;
use crate::core::search::top_docs::top_docs_util;
use crate::core::search::top_docs_collector::TopDocsCollector;
use crate::core::search::top_field_collector::{
    PagingFieldCollector, SimpleFieldCollector, TopFieldCollectorEnum,
};
use crate::core::search::top_field_docs::TopFieldDocs;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;
/// Creates a [`TopFieldCollectorManager`] which uses a shared hit counter to maintain
/// the number of hits, and a shared [`MaxScoreAccumulator`] to propagate the minimum
/// score across segments when the primary sort is by relevancy.
///
///
/// Note:
/// A new collector manager should be created for each search,
/// since it maintains internal state that is not thread-safe or reusable.
pub struct TopFieldCollectorManager {
    sort: Rc<Sort>,
    num_hits: i32,
    after: Option<FieldDoc>,
    total_hits_threshold: i32,
    min_score_acc: Option<MaxScoreAccumulator>,
}
impl TopFieldCollectorManager {
    /// Creates a new [`TopFieldCollectorManager`] from the given arguments,
    /// with thread-safe internal states.
    ///
    ///
    /// **NOTE:**
    /// The instances returned by this method pre-allocate a full array of length `num_hits`.
    ///
    /// # Arguments
    ///
    /// * `sort` – The sort criteria ([`SortField`](crate::core::search::sort_field_enum::SortFieldEnum)s).
    /// * `num_hits` – The number of results to collect.
    /// * `total_hits_threshold` – The number of documents to count accurately.
    ///   If the query matches more than `total_hits_threshold` hits,
    ///   then its hit count will be a lower bound.
    ///   On the other hand, if the query matches less than or exactly `total_hits_threshold` hits,
    ///   then the hit count of the result will be accurate.
    ///   Use `i32::MAX` to make the hit count fully accurate,
    ///   though this may make query processing slower.
    pub fn new(sort: Rc<Sort>, num_hits: i32, total_hits_threshold: i32) -> Result<Self> {
        Self::new_with_after(sort, num_hits, None, total_hits_threshold)
    }
    /// Creates a new [`TopFieldCollectorManager`] from the given arguments,
    /// with thread-safe internal states.
    ///
    ///
    /// **NOTE:**
    /// The instances returned by this method pre-allocate a full array of length `num_hits`.
    ///
    /// # Arguments
    ///
    /// * `sort` – The sort criteria ([`SortField`](crate::core::search::sort_field_enum::SortFieldEnum)s).
    /// * `num_hits` – The number of results to collect.
    /// * `after` – The previous [`FieldDoc`] after which matching documents will be collected.
    /// * `total_hits_threshold` – The number of documents to count accurately.
    ///   If the query matches more than `total_hits_threshold` hits,
    ///   then its hit count will be a lower bound.
    ///   On the other hand, if the query matches less than or exactly `total_hits_threshold` hits,
    ///   then the hit count of the result will be accurate.
    ///   Use `i32::MAX` to make the hit count fully accurate,
    ///   though this may make query processing slower.
    pub fn new_with_after(
        sort: Rc<Sort>,
        num_hits: i32,
        after: Option<FieldDoc>,
        total_hits_threshold: i32,
    ) -> Result<Self> {
        if total_hits_threshold < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "totalHitsThreshold must be >= 0, got {}",
                total_hits_threshold
            )));
        }

        if num_hits <= 0 {
            return Err(LuceneError::illegal_argument(
                "numHits must be > 0; please use TotalHitCountCollector if you just need the total hit count".to_string(),
            ));
        }

        let sort_fields = sort.get_sort();
        if sort_fields.is_empty() {
            return Err(LuceneError::illegal_argument(
                "Sort must contain at least one field".to_string(),
            ));
        }

        if let Some(ref after_doc) = after {
            if after_doc.fields.is_empty() {
                return Err(LuceneError::illegal_argument(
                    "after.fields wasn't set; you must pass fillFields=true for the previous search".to_string(),
                ));
            }

            if after_doc.fields.len() != sort_fields.len() {
                return Err(LuceneError::illegal_argument(format!(
                    "after.fields has {} values but sort has {}",
                    after_doc.fields.len(),
                    sort_fields.len()
                )));
            }
        }

        let min_score_acc = if total_hits_threshold != i32::MAX {
            Some(MaxScoreAccumulator::new())
        } else {
            None
        };

        Ok(Self {
            sort,
            num_hits,
            after,
            total_hits_threshold,
            min_score_acc,
        })
    }
}
impl CollectorManager for TopFieldCollectorManager {
    type C = TopFieldCollectorEnum;
    type T = TopFieldDocs;

    fn new_collector(&self) -> Result<Self::C> {
        let mut queue = create(self.sort.get_sort(), self.num_hits)?;

        let collector = if self.after.is_none() {
            // Inform a comparator that sort is based on a single field,
            // to enable optimizations for skipping over non-competitive documents.
            // We can't set single sort when `after` is non-null as it's
            // an implicit sort over the document id.
            if queue.get_comparators().len() == 1 {
                let comparators = queue.get_comparators_mut();
                comparators[0].set_single_sort();
            }

            TopFieldCollectorEnum::Simple(SimpleFieldCollector::new(
                Rc::clone(&self.sort),
                queue,
                self.num_hits,
                self.total_hits_threshold,
                self.min_score_acc.clone(),
            )?)
        } else {
            let after = self.after.clone().ok_or_else(|| {
                LuceneError::illegal_argument(
                    "`after` must be set before creating a PagingFieldCollector",
                )
            })?;

            if after.fields.is_empty() {
                return Err(LuceneError::illegal_argument(
                    "`after.fields` wasn't set; you must pass fill_fields=true for the previous search",
                ));
            }

            if after.fields.len() != self.sort.get_sort().len() {
                return Err(LuceneError::illegal_argument(format!(
                    "`after.fields` has {} values but sort has {}",
                    after.fields.len(),
                    self.sort.get_sort().len()
                )));
            }

            TopFieldCollectorEnum::Paging(PagingFieldCollector::new(
                Rc::clone(&self.sort),
                queue,
                after,
                self.num_hits,
                self.total_hits_threshold,
                self.min_score_acc.clone(),
            )?)
        };

        Ok(collector)
    }

    fn reduce(&self, collectors: Vec<Self::C>) -> Result<Self::T> {
        let len = collectors.len();
        let mut top_docs_list = Vec::with_capacity(len);
        for mut collector in collectors {
            let mut v = collector.top_docs()?;
            // Here we discard TopFieldDocs#fields because it is not used in the original Java Lucene implementation
            top_docs_list.push(std::mem::take(&mut v.base));
        }
        top_docs_util::merge_top_field_docs_with_start(&self.sort, 0, self.num_hits, top_docs_list)
    }
}
