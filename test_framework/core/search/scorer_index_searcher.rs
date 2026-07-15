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
use crate::core::index::index_reader::{IndexReader, IndexReaderContextType};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::index_searcher::{IndexSearcher, IndexSearcherBase, IndexSearcherHook};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorer::Scorer;
use crate::core::search::weight::Weight;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::LuceneError;

#[allow(dead_code)] // for quick search
pub struct ScorerIndexSearcher<IR>
where
  IR: IndexReader + 'static,
{
  pub s: IndexSearcher<IndexReaderContextType<IR>>,
}
impl<IR> ScorerIndexSearcher<IR>
where
  IR: IndexReader + 'static,
{
  pub fn new(reader: IR) -> Self {
    let s = IndexSearcher::new(reader.get_context().unwrap())
      .unwrap()
      .with_hook(IndexSearcherHook::Scorer(ScorerIndexSearcherHook));
    Self { s }
  }
}

#[derive(Default)]
pub(crate) struct ScorerIndexSearcherHook;

impl<IRC> IndexSearcherBase<IRC> for ScorerIndexSearcherHook
where
  IRC: IndexReaderContext,
{
  fn search_leaf<W, C>(
    &self,
    searcher: &IndexSearcher<IRC>,
    ctx_ord: usize,
    min_doc_id: i32,
    max_doc_id: i32,
    weight: &W,
    collector: &mut C,
  ) -> crate::core::util::error::lucene_error::Result<()>
  where
    C: Collector,
    W: Weight<IRC> + ?Sized,
  {
    // the default slices method does not create segment partitions, and we don't provide an
    // executor to this searcher in our codebase, so we should not run into this problem. This type
    // can though be used externally, hence it is better to provide a clear and hard error.
    if min_doc_id != 0 || max_doc_id != NO_MORE_DOCS {
      return Err(LuceneError::illegal_state(
        "intra-segment concurrency is not supported by this searcher",
      ));
    }
    let ctx = &searcher.get_leaf_contexts()?[ctx_ord];
    // we force the use of Scorer (not BulkScorer) to make sure
    // that the scorer passed to LeafCollector.setScorer supports
    // Scorer.getChildren
    let Some(mut scorer) = weight.scorer(ctx, searcher)? else {
      return Ok(());
    };

    let mut leaf_collector = collector.get_leaf_collector(ctx, Some(weight), searcher)?;
    leaf_collector.set_scorer(&mut scorer)?;

    let live_docs = ctx.reader().get_live_docs()?;

    let mut doc = scorer.iterator_mut().next_doc()?;
    while doc != NO_MORE_DOCS {
      let accepted = match live_docs.as_ref() {
        None => true,
        Some(bits) => bits.get(doc as usize)?,
      };

      if accepted {
        leaf_collector.collect(doc, &mut scorer)?;
      }

      doc = scorer.iterator_mut().next_doc()?;
    }

    Ok(())
  }
}
