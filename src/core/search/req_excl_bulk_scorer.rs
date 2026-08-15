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
use crate::core::search::bulk_scorer::BulkScorer;
#[cfg(test)]
use crate::core::search::bulk_scorer::BulkScorerKind;
#[cfg(test)]
use crate::core::search::bulk_scorer::BulkScorerKind::ReqExcl;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorer::Scorer;
use crate::core::search::two_phase_iterator::TwoPhaseIterator;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;

pub struct ReqExclBulkScorer<BS, S> {
  req: BS,
  excl: S,
}
impl<BS, S> ReqExclBulkScorer<BS, S> {
  pub(crate) fn new(req: BS, excl: S) -> Self {
    Self { req, excl }
  }
}
impl<BS, S> BulkScorer for ReqExclBulkScorer<BS, S>
where
  BS: BulkScorer,
  S: Scorer,
{
  fn score(
    &mut self,
    collector: &mut dyn LeafCollector,
    accept_docs: Option<&dyn Bits>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    let mut upto = min;

    let mut excl_doc = self.excl.approximation().doc_id();

    while upto < max {
      if excl_doc < upto {
        excl_doc = self.excl.approximation_mut().advance(upto)?;
      }
      if excl_doc == upto {
        let excluded = match self.excl.two_phase_iterator_mut() {
          None => true,
          Some(ref mut tpi) => tpi.matches()?,
        };

        if excluded {
          upto += 1;
        }
        excl_doc = self.excl.approximation_mut().next_doc()?;
      } else {
        let limit = excl_doc.min(max);
        upto = self.req.score(collector, accept_docs, upto, limit)?;
      }
    }
    if upto == max {
      upto = self.req.score(collector, accept_docs, upto, upto)?;
    }

    Ok(upto)
  }

  fn cost(&mut self) -> Result<i64> {
    self.req.cost()
  }
  #[cfg(test)]
  fn kind(&self) -> BulkScorerKind {
    ReqExcl
  }
}
