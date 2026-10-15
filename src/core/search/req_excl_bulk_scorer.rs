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
use crate::core::search::scorer::Scorer;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;

pub struct ReqExclBulkScorer<BS, S> {
  pub(crate) req: BS,
  pub(crate) excl: S,
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
  fn score<CLC: crate::core::search::leaf_collector::LeafCollector, AB: Bits + ?Sized>(
    &mut self,
    collector: &mut CLC,
    accept_docs: Option<&AB>,
    min: i32,
    max: i32,
  ) -> Result<i32> {
    let mut upto = min;

    let mut excl_doc = self.excl.approx_doc_id();

    while upto < max {
      if excl_doc < upto {
        excl_doc = self.excl.approx_advance(upto)?;
      }
      if excl_doc == upto {
        let excluded = self.excl.matches()?;

        if excluded {
          upto += 1;
        }
        excl_doc = self.excl.approx_next_doc()?;
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
