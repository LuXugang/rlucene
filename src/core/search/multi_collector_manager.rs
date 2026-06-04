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
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::multi_collector::{OneOrMultiCollector, wrap};
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// A [`CollectorManager`] implementation which wraps a set of [`CollectorManager`]s as
/// [`MultiCollector`](crate::core::search::multi_collector::MultiCollector) acts for
/// [`Collector`](crate::core::search::collector::Collector).
pub struct MultiCollectorManager<M> {
  collector_managers: Vec<M>,
}

impl<M> MultiCollectorManager<M> {
  pub fn new(collector_managers: Vec<M>) -> Result<Self> {
    if collector_managers.is_empty() {
      return Err(LuceneError::illegal_argument(
        "There must be at least one collector manager",
      ));
    }

    Ok(Self { collector_managers })
  }
}

impl<M> CollectorManager for MultiCollectorManager<M>
where
  M: CollectorManager,
{
  type C = OneOrMultiCollector<M::C>;
  type T = Vec<M::T>;

  fn new_collector(&self) -> Result<Self::C> {
    let mut collectors = Vec::with_capacity(self.collector_managers.len());
    for collector_manager in &self.collector_managers {
      collectors.push(Some(collector_manager.new_collector()?));
    }
    wrap(collectors)
  }

  fn reduce(&self, reducable_collectors: Vec<Self::C>) -> Result<Self::T> {
    let size = reducable_collectors.len();
    let mut reducable_collectors_by_manager: Vec<Vec<M::C>> = (0..self.collector_managers.len())
      .map(|_| Vec::with_capacity(size))
      .collect();

    for collector in reducable_collectors {
      let collectors = match collector {
        OneOrMultiCollector::One(collector) => vec![collector],
        OneOrMultiCollector::Multi(collector) => collector.into_collectors(),
      };

      if collectors.len() != self.collector_managers.len() {
        return Err(LuceneError::illegal_state(format!(
          "expected {} collectors, got {}",
          self.collector_managers.len(),
          collectors.len()
        )));
      }

      for (idx, collector) in collectors.into_iter().enumerate() {
        reducable_collectors_by_manager[idx].push(collector);
      }
    }

    let mut results = Vec::with_capacity(self.collector_managers.len());
    for (collector_manager, reducable_collectors) in self
      .collector_managers
      .iter()
      .zip(reducable_collectors_by_manager)
    {
      results.push(collector_manager.reduce(reducable_collectors)?);
    }
    Ok(results)
  }
}
