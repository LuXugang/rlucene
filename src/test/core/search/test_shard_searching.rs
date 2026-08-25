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

use crate::core::index::BytesRef;
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::index_reader::{IndexReader, IndexReaderContextType};
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::multi_terms;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::field_value_hit_queue::TopFieldScoreDoc;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{IntoQuery, Query};
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::search::shard_searching_test_base::{
  SearcherExpiredException, ShardIndexSearcher, ShardSearchingTestContext,
};
use crate::test_framework::core::util::DefaultCRReader;
use crate::test_framework::core::util::lucene_test_case::{at_least, is_night_mode, random};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::SliceRandom;
use rand::{Rng, RngExt};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Instant;

//   - other queries besides PrefixQuery & TermQuery (but:
//     FuzzyQ will be problematic... the top N terms it
//     takes means results will differ)
//   - NRQ/F
//   - BQ, negated clauses, negated prefix clauses
//   - test pulling docs in 2nd round trip...
//   - filter too

#[allow(dead_code)] // for quick search
struct TestShardSearching;

struct PreviousSearchState {
  search_time: Instant,
  versions: Vec<i64>,
  search_after_local: Option<ScoreDoc>,
  search_after_shard: Option<ScoreDoc>,
  sort: Option<Arc<Sort>>,
  query: Query,
  num_hits_paged: usize,
}

impl PreviousSearchState {
  fn new(
    query: Query,
    sort: Option<Arc<Sort>>,
    search_after_local: Option<ScoreDoc>,
    search_after_shard: Option<ScoreDoc>,
    versions: &[i64],
    num_hits_paged: usize,
  ) -> Self {
    Self {
      versions: versions.to_vec(),
      search_after_local,
      search_after_shard,
      sort,
      query,
      num_hits_paged,
      search_time: Instant::now(),
    }
  }
}

type MockReader = Arc<MultiReader<Arc<DefaultCRReader>>>;
type MockSearcher = IndexSearcher<IndexReaderContextType<MockReader>>;

#[test]
fn test_simple() -> Result<()> {
  let mut random = random();
  let num_nodes = TestUtil::next_int(&mut random, 1, 10) as usize;

  let run_time_sec = if is_night_mode() {
    at_least(&mut random, 5)
  } else {
    at_least(&mut random, 1)
  } as f64;

  let min_docs_to_make_terms = TestUtil::next_int(&mut random, 5, 20);

  let max_searcher_age_seconds = TestUtil::next_int(&mut random, 1, 3);

  let context = ShardSearchingTestContext::new();
  context.start(
    &mut random,
    num_nodes,
    run_time_sec,
    max_searcher_age_seconds,
  )?;

  let mut prior_searches = Vec::new();
  let mut terms: Option<Vec<BytesRef<Vec<u8>>>> = None;
  while Instant::now() < context.end_time() {
    let do_followon = !prior_searches.is_empty() && random.random_range(0..7) == 1;

    // Pick a random node; we will run the query on this node:
    let my_node_id = random.random_range(0..num_nodes);
    let nodes = context.nodes();

    let (local_shard_searcher, prev_search_index) = if do_followon {
      // Pretend user issued a followon query:
      let prev_search_index = random.random_range(0..prior_searches.len());
      let prev_search_state: &PreviousSearchState = &prior_searches[prev_search_index];
      let _search_age = prev_search_state.search_time.elapsed();
      match nodes[my_node_id].acquire_versions(&prev_search_state.versions) {
        Ok(searcher) => (searcher, Some(prev_search_index)),
        Err(error) if SearcherExpiredException::is_instance(&error) => {
          // Expected, sometimes; in a "real" app we would
          // either forward this error to the user ("too
          // much time has passed; please re-run your
          // search") or sneakily just switch to newest
          // searcher w/o telling them...
          prior_searches.remove(prev_search_index);
          continue;
        },
        Err(error) => return Err(error),
      }
    } else {
      // Do fresh query:
      (nodes[my_node_id].acquire()?, None)
    };

    let mut subs = Vec::with_capacity(num_nodes);
    let body_result = catch_unwind(AssertUnwindSafe(
      || -> Result<Option<PreviousSearchState>> {
        // Mock: now make a single reader (MultiReader) from all node
        // searchers.  In a real shard env you can't do this... we
        // do it to confirm results from the shard searcher
        // are correct:
        let mock_reader_result = (|| -> Result<(i32, MockReader, MockSearcher)> {
          let mut doc_count = 0;
          for (node_id, node) in nodes.iter().enumerate().take(num_nodes) {
            let sub_version = local_shard_searcher.node_versions[node_id];
            let Some(sub) = node.searchers.acquire(sub_version)? else {
              return Err(
                SearcherExpiredException::new(format!("nodeID=-1 version={sub_version}")).into(),
              );
            };
            let reader = sub.get_index_reader().clone();
            doc_count += reader.max_doc()?;
            subs.push(reader);
          }

          let mock_reader = Arc::new(MultiReader::new(subs.clone())?);
          let mock_searcher = IndexSearcher::new(mock_reader.clone().get_context()?)?;
          Ok((doc_count, mock_reader, mock_searcher))
        })();
        let (doc_count, mock_reader, mock_searcher) = match mock_reader_result {
          Ok(value) => value,
          Err(error) if SearcherExpiredException::is_instance(&error) => {
            // Expected
            return Ok(None);
          },
          Err(error) => return Err(error),
        };

        let (query, sort) = if let Some(prev_search_index) = prev_search_index {
          let prev_search_state: &PreviousSearchState = &prior_searches[prev_search_index];
          (
            prev_search_state.query.clone(),
            prev_search_state.sort.clone(),
          )
        } else {
          if terms.is_none()
            && doc_count > min_docs_to_make_terms
            && let Some(body_terms) = multi_terms::get_terms(mock_reader.clone(), "body")?
          {
            let mut terms_enum = body_terms.iterator()?;
            let mut new_terms = Vec::new();
            while let Some(term) = terms_enum.next()? {
              new_terms.push(BytesRef::deep_copy_of(term.as_ref()));
            }
            if !new_terms.is_empty() {
              terms = Some(new_terms);
            }
          }

          let Some(terms) = terms.as_ref() else {
            return Ok(None);
          };
          let query = if random.random_bool(0.5) {
            TermQuery::new(Term::new(
              "body",
              terms[random.random_range(0..terms.len())].clone(),
            ))
            .into_query()
          } else {
            let term = terms[random.random_range(0..terms.len())].utf8_to_string()?;
            let prefix = if term.chars().count() <= 1 {
              term
            } else {
              term
                .chars()
                .take(TestUtil::next_int(&mut random, 1, 2) as usize)
                .collect()
            };
            PrefixQuery::new(Term::from_text("body", prefix))?.into_query()
          };

          let sort = if random.random_bool(0.5) {
            None
          } else {
            match random.random_range(0..3) {
              0 => Some(Arc::new(Sort::new()?)),
              1 => None,
              2 => Some(Arc::new(Sort::with_fields(vec![SortField::with_reverse(
                Some("docid_intDV"),
                SortFieldType::Int,
                random.random_bool(0.5),
              )?])?)),
              _ => Some(Arc::new(Sort::with_fields(vec![SortField::with_reverse(
                Some("titleDV"),
                SortFieldType::String,
                random.random_bool(0.5),
              )?])?)),
            }
          };
          (query, sort)
        };

        let previous = prev_search_index.map(|index| &prior_searches[index]);
        match assert_same(
          &mut random,
          &mock_searcher,
          local_shard_searcher.as_ref(),
          query,
          sort,
          previous,
        ) {
          Ok(search_state) => Ok(search_state),
          Err(error) if SearcherExpiredException::is_instance(&error) => {
            // Expected; in a "real" app we would either
            // forward this error to the user or switch to the newest searcher.
            if let Some(prev_search_index) = prev_search_index {
              prior_searches.remove(prev_search_index);
            }
            Ok(None)
          },
          Err(error) => Err(error),
        }
      },
    ));
    let close_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      nodes[my_node_id].release(local_shard_searcher)?;
      for sub in &subs {
        sub.dec_ref()?;
      }
      Ok(())
    }));
    let search_state = IOUtils::finally_caught_result(body_result, close_result)?;

    if let Some(search_state) = search_state
      && search_state.search_after_local.is_some()
      && random.random_range(0..5) == 3
    {
      prior_searches.push(search_state);
      if prior_searches.len() > 200 {
        prior_searches.shuffle(&mut random);
        prior_searches.truncate(100);
      }
    }
  }

  context.finish()
}

fn assert_same<R>(
  random: &mut R,
  mock_searcher: &MockSearcher,
  shard_searcher: &ShardIndexSearcher,
  query: Query,
  sort: Option<Arc<Sort>>,
  state: Option<&PreviousSearchState>,
) -> Result<Option<PreviousSearchState>>
where
  R: Rng + ?Sized,
{
  let mut num_hits = TestUtil::next_int(random, 1, 100) as usize;
  if let Some(state) = state
    && state.search_after_local.is_none()
  {
    // In addition to what we last searched:
    num_hits += state.num_hits_paged;
  }

  let (hits, mut shard_hits) = if let Some(sort) = sort.as_ref() {
    // Single (mock local) searcher:
    let hits = mock_searcher.search_with_sort(query.clone(), num_hits, sort.clone())?;

    // Shard searcher:
    let shard_hits = shard_searcher.search_with_sort(query.clone(), num_hits, sort.clone())?;
    (hits.base, shard_hits.base)
  } else {
    // Single (mock local) searcher:
    let hits = if let Some(after) = state.and_then(|state| state.search_after_local.clone()) {
      mock_searcher.search_after_score(Some(after), query.clone(), num_hits)?
    } else {
      mock_searcher.search(query.clone(), num_hits)?
    };

    // Shard searcher:
    let shard_hits = if let Some(after) = state.and_then(|state| state.search_after_shard.clone()) {
      shard_searcher.search_after(Some(after), query.clone(), num_hits)?
    } else {
      shard_searcher.search(query.clone(), num_hits)?
    };

    (
      TopDocs::new(
        hits.total_hits,
        hits
          .score_docs
          .into_iter()
          .map(TopFieldScoreDoc::from)
          .collect(),
      ),
      TopDocs::new(
        shard_hits.total_hits,
        shard_hits
          .score_docs
          .into_iter()
          .map(TopFieldScoreDoc::from)
          .collect(),
      ),
    )
  };

  let num_nodes = shard_searcher.node_versions.len();
  let mut base = Vec::with_capacity(num_nodes);
  let mut doc_base = 0;
  for sub in mock_searcher
    .get_index_reader()
    .get_sequential_sub_readers()
  {
    base.push(doc_base);
    doc_base += sub.max_doc()?;
  }
  assert_eq!(num_nodes, base.len());

  let mut num_hits_paged = hits.score_docs.len();
  if let Some(state) = state
    && state.search_after_local.is_some()
  {
    num_hits_paged += state.num_hits_paged;
  }

  let (more_hits, bottom_hit, bottom_hit_shards) = if num_hits_paged < hits.total_hits.value() {
    // More hits to page through
    if sort.is_none() {
      let bottom_hit = hits.score_docs.last().unwrap().as_score().unwrap().clone();
      let score_doc = shard_hits.score_docs.last().unwrap().as_score().unwrap();
      // Must copy because below we rebase:
      let bottom_hit_shards =
        ScoreDoc::with_shard_index(score_doc.doc, score_doc.score, score_doc.shard_index);
      (true, Some(bottom_hit), Some(bottom_hit_shards))
    } else {
      (true, None, None)
    }
  } else {
    assert_eq!(hits.total_hits.value(), num_hits_paged);
    (false, None, None)
  };

  // Must rebase so the equality assertion passes:
  for score_doc in &mut shard_hits.score_docs {
    let shard_index = score_doc.shard_index() as usize;
    score_doc.score_doc_mut().doc += base[shard_index];
  }

  TestUtil::assert_consistent(&hits, &shard_hits);

  if more_hits {
    // Return a continuation:
    Ok(Some(PreviousSearchState::new(
      query,
      sort,
      bottom_hit,
      bottom_hit_shards,
      &shard_searcher.node_versions,
      num_hits_paged,
    )))
  } else {
    Ok(None)
  }
}
