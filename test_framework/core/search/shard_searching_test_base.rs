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
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::term::Term;
use crate::core::index::term_states;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::index_searcher::{
  IndexSearcher, IndexSearcherBase, IndexSearcherDefaults, IndexSearcherHook,
};
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::query_visitor::term_collector;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::searcher_lifetime_manager::{PruneByAge, SearcherLifetimeManager};
use crate::core::search::searcher_manager::SearcherManager;
use crate::core::search::sort::Sort;
use crate::core::search::term_statistics::TermStatistics;
use crate::core::search::top_docs::{TopDocs, merge_top_docs, merge_top_field_docs};
use crate::core::search::top_field_docs::TopFieldDocs;
use crate::core::store::directory::DirEnum;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::DefaultCRReader;
use crate::test_framework::core::util::line_file_docs::LineFileDocs;
use crate::test_framework::core::util::lucene_test_case::{
  create_temp_dir_with_prefix, new_fs_directory, random_from_seed,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::{Mutex, RwLock};
use rand::{Rng, RngExt};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type ManagedSearcher = IndexSearcher<IndexReaderContextType<Arc<DefaultCRReader>>>;

/// Base test struct for simulating distributed search across multiple shards.
#[allow(dead_code)] // for quick search
struct ShardSearchingTestBase;

/// Thrown when the lease for a searcher has expired.
pub struct SearcherExpiredException {
  message: String,
}

const SEARCHER_EXPIRED_PREFIX: &str = "searcher expired: ";

impl SearcherExpiredException {
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }

  pub fn is_instance(error: &LuceneError) -> bool {
    matches!(error, LuceneError::IllegalState(_))
      && error.to_string().starts_with(SEARCHER_EXPIRED_PREFIX)
  }
}

impl From<SearcherExpiredException> for LuceneError {
  fn from(exception: SearcherExpiredException) -> Self {
    LuceneError::illegal_state(format!("{SEARCHER_EXPIRED_PREFIX}{}", exception.message))
  }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FieldAndShardVersion {
  node_id: usize,
  version: i64,
  field: String,
}

impl Display for FieldAndShardVersion {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      formatter,
      "FieldAndShardVersion(field={} nodeID={} version={})",
      self.field, self.node_id, self.version
    )
  }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TermAndShardVersion {
  node_id: usize,
  version: i64,
  term: Term,
}

// We share collection stats for these fields on each node reopen:
const FIELDS_TO_SHARE: [&str; 2] = ["body", "title"];

// Java returns `TopDocs` for both branches; Rust represents the two concrete result types
// explicitly because `TopFieldDocs` does not inherit from `TopDocs`.
enum SearchNodeResult {
  Score(TopDocs<ScoreDoc>),
  Field(TopFieldDocs),
}

struct ShardSearchingState {
  nodes: RwLock<Vec<Arc<NodeState>>>,
  max_searcher_age_seconds: RwLock<f64>,
  end_time: RwLock<Option<Instant>>,
  change_indices_thread: Mutex<Option<JoinHandle<Result<()>>>>,
}

impl ShardSearchingState {
  fn new() -> Self {
    Self {
      nodes: RwLock::new(Vec::new()),
      max_searcher_age_seconds: RwLock::new(0.0),
      end_time: RwLock::new(None),
      change_indices_thread: Mutex::new(None),
    }
  }

  fn nodes(&self) -> Vec<Arc<NodeState>> {
    self.nodes.read().clone()
  }

  // Called by one node once it has reopened, to notify all
  // other nodes. This is just a mock (since it goes and
  // directly updates all other nodes, in RAM)... in a real
  // env this would hit the wire, sending version &
  // collection stats to all other nodes:
  fn broadcast_node_reopen(
    &self,
    node_id: usize,
    version: i64,
    new_searcher: &ManagedSearcher,
  ) -> Result<()> {
    let nodes = self.nodes();

    // Broadcast new collection stats for this node to all other nodes:
    for field in FIELDS_TO_SHARE {
      if let Some(stats) = new_searcher.collection_statistics(field)? {
        let stats = Arc::new(stats);
        for node in &nodes {
          // Don't put my own collection stats into the cache; we pull locally.
          if node.my_node_id != node_id {
            node.collection_stats_cache.lock().insert(
              FieldAndShardVersion {
                node_id,
                version,
                field: field.to_string(),
              },
              stats.clone(),
            );
          }
        }
      }
    }
    for node in nodes {
      node.update_node_version(node_id, version)?;
    }
    Ok(())
  }

  // MOCK: in a real env you have to hit the wire
  // (send this query to all remote nodes concurrently):
  fn search_node(
    &self,
    node_id: usize,
    node_versions: &[i64],
    query: Query,
    sort: Option<Arc<Sort>>,
    num_hits: usize,
    search_after: Option<ScoreDoc>,
  ) -> Result<SearchNodeResult> {
    let nodes = self.nodes();
    let searcher = nodes[node_id].acquire_versions(node_versions)?;
    let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<_> {
      if let Some(sort) = sort {
        debug_assert!(search_after.is_none()); // not supported yet
        searcher
          .local_search_with_sort(query, num_hits, sort)
          .map(SearchNodeResult::Field)
      } else if search_after.is_some() {
        searcher
          .local_search_after(search_after, query, num_hits)
          .map(SearchNodeResult::Score)
      } else {
        searcher
          .local_search(query, num_hits)
          .map(SearchNodeResult::Score)
      }
    }));
    let close_result = catch_unwind(AssertUnwindSafe(|| nodes[node_id].release(searcher)));
    IOUtils::finally_caught_result(body_result, close_result)
  }

  // Mock: in a real env, this would hit the wire and get
  // term stats from remote node
  fn get_node_term_stats(
    &self,
    terms: &HashSet<Term>,
    node_id: usize,
    version: i64,
  ) -> Result<HashMap<Term, Arc<TermStatistics>>> {
    let nodes = self.nodes();
    let node = &nodes[node_id];
    let searcher = node
      .searchers
      .acquire(version)?
      .ok_or_else(|| SearcherExpiredException::new(format!("node={node_id} version={version}")))?;
    let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<_> {
      let mut stats = HashMap::new();
      for term in terms {
        let term_states = term_states::build(searcher.as_ref(), term.clone(), true)?;
        let doc_freq = term_states.doc_freq()?;
        if doc_freq > 0 {
          stats.insert(
            term.clone(),
            Arc::new(searcher.term_statistics(
              term.clone(),
              doc_freq,
              term_states.total_term_freq()?,
            )?),
          );
        }
      }
      Ok(stats)
    }));
    let close_result = catch_unwind(AssertUnwindSafe(|| node.searchers.release(searcher)));
    IOUtils::finally_caught_result(body_result, close_result)
  }
}

/// Simulated shard node under test.
pub struct NodeState {
  pub dir: Arc<DirEnum>,
  pub writer: Arc<IndexWriter<DirEnum>>,
  pub searchers: Arc<SearcherLifetimeManager<StandardDirectoryReader<DirEnum>>>,
  pub mgr: Arc<SearcherManager<StandardDirectoryReader<DirEnum>>>,
  pub my_node_id: usize,
  pub current_node_versions: RwLock<Vec<i64>>,

  collection_stats_cache: Mutex<HashMap<FieldAndShardVersion, Arc<CollectionStatistics>>>,
  term_stats_cache: Mutex<HashMap<TermAndShardVersion, Arc<TermStatistics>>>,
  current_shard_searcher: RwLock<Option<Arc<ShardIndexSearcher>>>,
  state: Weak<ShardSearchingState>,
}

/// Matches docs in the local shard but scores based on aggregated stats ("mock distributed
/// scoring") from all nodes.
pub struct ShardIndexSearcher {
  searcher: IndexSearcher<IndexReaderContextType<Arc<DefaultCRReader>>>,
  pub node_versions: Vec<i64>,
  pub my_node_id: usize,
  state: Weak<ShardSearchingState>,
}

impl ShardIndexSearcher {
  fn new(
    node_versions: Vec<i64>,
    local_reader: Arc<DefaultCRReader>,
    my_node_id: usize,
    state: Weak<ShardSearchingState>,
  ) -> Result<Self> {
    let searcher = IndexSearcher::new(local_reader.clone().get_context()?)?.with_hook(
      IndexSearcherHook::Shard(Box::new(ShardIndexSearcherHook {
        local_reader,
        node_versions: node_versions.clone(),
        my_node_id,
        state: state.clone(),
      })),
    );
    Ok(Self {
      searcher,
      node_versions,
      my_node_id,
      state,
    })
  }

  pub fn get_index_reader(&self) -> &Arc<DefaultCRReader> {
    self.searcher.get_index_reader()
  }
}

pub(crate) struct ShardIndexSearcherHook {
  local_reader: Arc<DefaultCRReader>,
  node_versions: Vec<i64>,
  my_node_id: usize,
  state: Weak<ShardSearchingState>,
}

impl<IRC> IndexSearcherBase<IRC> for ShardIndexSearcherHook
where
  IRC: IndexReaderContext,
{
  fn rewrite(&self, _searcher: &IndexSearcher<IRC>, original: Query) -> Result<Query> {
    let local_searcher = IndexSearcher::new(self.local_reader.clone().get_context()?)?;
    let original = local_searcher.rewrite(original)?;
    let mut terms = HashSet::new();
    original.visit(&mut term_collector(&mut terms))?;

    let state = self
      .state
      .upgrade()
      .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?;
    let nodes = state.nodes();
    let local_node = &nodes[self.my_node_id];
    for node_id in 0..self.node_versions.len() {
      if node_id == self.my_node_id {
        continue;
      }
      let missing = terms
        .iter()
        .filter(|term| {
          !local_node
            .term_stats_cache
            .lock()
            .contains_key(&TermAndShardVersion {
              node_id,
              version: self.node_versions[node_id],
              term: (*term).clone(),
            })
        })
        .cloned()
        .collect::<HashSet<_>>();
      if !missing.is_empty() {
        for (term, stats) in
          state.get_node_term_stats(&missing, node_id, self.node_versions[node_id])?
        {
          local_node.term_stats_cache.lock().insert(
            TermAndShardVersion {
              node_id,
              version: self.node_versions[node_id],
              term,
            },
            stats,
          );
        }
      }
    }
    Ok(original)
  }

  fn term_statistics(
    &self,
    _searcher: &IndexSearcher<IRC>,
    term: Arc<Term>,
    doc_freq: i32,
    total_term_freq: i64,
  ) -> Result<TermStatistics> {
    let state = self
      .state
      .upgrade()
      .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?;
    let nodes = state.nodes();
    let mut distributed_doc_freq = 0;
    let mut distributed_total_term_freq = 0;
    for node_id in 0..self.node_versions.len() {
      let stats = if node_id == self.my_node_id {
        Some(Arc::new(IndexSearcherDefaults::term_statistics(
          term.clone(),
          doc_freq,
          total_term_freq,
        )?))
      } else {
        nodes[self.my_node_id]
          .term_stats_cache
          .lock()
          .get(&TermAndShardVersion {
            node_id,
            version: self.node_versions[node_id],
            term: term.as_ref().clone(),
          })
          .cloned()
      };
      let Some(stats) = stats else {
        continue; // term not found
      };

      let node_doc_freq = stats.get_doc_freq();
      distributed_doc_freq += node_doc_freq;

      let node_total_term_freq = stats.get_total_term_freq();
      distributed_total_term_freq += node_total_term_freq;
    }
    debug_assert!(distributed_doc_freq > 0);
    TermStatistics::new(term, distributed_doc_freq, distributed_total_term_freq)
  }

  fn collection_statistics(
    &self,
    searcher: &IndexSearcher<IRC>,
    field: &str,
  ) -> Result<Option<CollectionStatistics>> {
    let state = self
      .state
      .upgrade()
      .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?;
    let nodes = state.nodes();
    let mut doc_count = 0;
    let mut sum_total_term_freq = 0;
    let mut sum_doc_freq = 0;
    let mut max_doc = 0;

    for node_id in 0..self.node_versions.len() {
      let node_stats = if node_id == self.my_node_id {
        IndexSearcherDefaults::collection_statistics(searcher, field)?.map(Arc::new)
      } else {
        nodes[self.my_node_id]
          .collection_stats_cache
          .lock()
          .get(&FieldAndShardVersion {
            node_id,
            version: self.node_versions[node_id],
            field: field.to_string(),
          })
          .cloned()
      };
      let Some(node_stats) = node_stats else {
        continue;
      };
      doc_count += node_stats.get_doc_count();
      sum_total_term_freq += node_stats.get_sum_total_term_freq();
      sum_doc_freq += node_stats.get_sum_doc_freq();
      debug_assert!(node_stats.get_max_doc() >= 0);
      max_doc += node_stats.get_max_doc();
    }

    if max_doc == 0 {
      Ok(None)
    } else {
      CollectionStatistics::new(field, max_doc, doc_count, sum_total_term_freq, sum_doc_freq)
        .map(Some)
    }
  }
}

impl ShardIndexSearcher {
  pub fn search(&self, query: Query, num_hits: usize) -> Result<TopDocs<ScoreDoc>> {
    let state = self
      .state
      .upgrade()
      .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?;
    let mut shard_hits = Vec::with_capacity(self.node_versions.len());
    for node_id in 0..self.node_versions.len() {
      let mut hits = if node_id == self.my_node_id {
        // My node; run using local shard searcher we already acquired:
        self.local_search(query.clone(), num_hits)?
      } else {
        let SearchNodeResult::Score(hits) = state.search_node(
          node_id,
          &self.node_versions,
          query.clone(),
          None,
          num_hits,
          None,
        )?
        else {
          unreachable!()
        };
        hits
      };
      for score_doc in &mut hits.score_docs {
        score_doc.set_shard_index(node_id as i32);
      }
      shard_hits.push(hits);
    }
    merge_top_docs(num_hits, shard_hits)
  }

  pub fn local_search(&self, query: Query, num_hits: usize) -> Result<TopDocs<ScoreDoc>> {
    IndexSearcherDefaults::search(&self.searcher, query, num_hits)
  }

  pub fn search_after(
    &self,
    after: Option<ScoreDoc>,
    query: Query,
    num_hits: usize,
  ) -> Result<TopDocs<ScoreDoc>> {
    let Some(after) = after else {
      return self.local_search_after(None, query, num_hits);
    };
    let state = self
      .state
      .upgrade()
      .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?;
    let nodes = state.nodes();
    let mut shard_hits = Vec::with_capacity(self.node_versions.len());
    // Results are merged in that order: score, shardIndex, doc. Therefore we set
    // after to after.score and depending on the nodeID we set doc to either:
    // - not collect any more documents with that score (only with worse score)
    // - collect more documents with that score (and worse) following the last collected document
    // - collect all documents with that score (and worse)
    let mut shard_after = ScoreDoc::new(after.doc, after.score);
    for (node_id, node) in nodes.iter().enumerate().take(self.node_versions.len()) {
      if node_id < after.shard_index as usize {
        // All documents with after.score were already collected, so collect only documents with
        // worse scores.
        let searcher = node.acquire_versions(&self.node_versions)?;
        let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<i32> {
          // Setting after.doc to reader.maxDoc-1 tells TopScoreDocCollector that no more docs with
          // that score should be collected. In practice the sending shard won't have maxDoc at
          // hand, so it sends an arbitrary value that is fixed on the other end.
          Ok(searcher.get_index_reader().max_doc()? - 1)
        }));
        let close_result = catch_unwind(AssertUnwindSafe(|| node.release(searcher)));
        shard_after.doc = IOUtils::finally_caught_result(body_result, close_result)?;
      } else if node_id == after.shard_index as usize {
        // Collect documents following the last collected doc with after.score, plus worse scores.
        shard_after.doc = after.doc;
      } else {
        // All documents with after.score (and worse) should be collected because they did not make
        // it to top-N in the previous round.
        shard_after.doc = -1;
      }
      let mut hits = if node_id == self.my_node_id {
        // My node; run using local shard searcher we already acquired:
        self.local_search_after(Some(shard_after.clone()), query.clone(), num_hits)?
      } else {
        let SearchNodeResult::Score(hits) = state.search_node(
          node_id,
          &self.node_versions,
          query.clone(),
          None,
          num_hits,
          Some(shard_after.clone()),
        )?
        else {
          unreachable!()
        };
        hits
      };
      for score_doc in &mut hits.score_docs {
        score_doc.set_shard_index(node_id as i32);
      }
      shard_hits.push(hits);
    }
    merge_top_docs(num_hits, shard_hits)
  }

  pub fn local_search_after(
    &self,
    after: Option<ScoreDoc>,
    query: Query,
    num_hits: usize,
  ) -> Result<TopDocs<ScoreDoc>> {
    IndexSearcherDefaults::search_after_score(&self.searcher, after, query, num_hits)
  }

  pub fn search_with_sort(
    &self,
    query: Query,
    num_hits: usize,
    sort: Arc<Sort>,
  ) -> Result<TopFieldDocs> {
    let state = self
      .state
      .upgrade()
      .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?;
    let mut shard_hits = Vec::with_capacity(self.node_versions.len());
    for node_id in 0..self.node_versions.len() {
      let mut hits = if node_id == self.my_node_id {
        // My node; run using local shard searcher we already acquired:
        self.local_search_with_sort(query.clone(), num_hits, sort.clone())?
      } else {
        let SearchNodeResult::Field(hits) = state.search_node(
          node_id,
          &self.node_versions,
          query.clone(),
          Some(sort.clone()),
          num_hits,
          None,
        )?
        else {
          unreachable!()
        };
        hits
      };
      for score_doc in &mut hits.base.score_docs {
        score_doc.set_shard_index(node_id as i32);
      }
      shard_hits.push(hits.base);
    }
    merge_top_field_docs(sort.as_ref(), num_hits, shard_hits)
  }

  pub fn local_search_with_sort(
    &self,
    query: Query,
    num_hits: usize,
    sort: Arc<Sort>,
  ) -> Result<TopFieldDocs> {
    IndexSearcherDefaults::search_with_sort(&self.searcher, query, num_hits, sort)
  }
}

impl NodeState {
  fn new<R>(
    random: &mut R,
    state: Weak<ShardSearchingState>,
    node_id: usize,
    num_nodes: usize,
  ) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let dir = new_fs_directory(
      random,
      create_temp_dir_with_prefix("ShardSearchingTestBase")?,
    )?;
    let mut analyzer = MockAnalyzer::new(random);
    analyzer.set_max_token_length(TestUtil::next_int(
      random,
      1,
      crate::core::index::index_writer::MAX_TERM_LENGTH,
    ));
    let mut config = IndexWriterConfig::with_analyzer(analyzer)?;
    config.set_open_mode(OpenMode::Create);
    let writer = IndexWriter::new(dir.clone(), config)?;
    let mgr = Arc::new(SearcherManager::from_writer(&writer, None)?);
    Ok(Self {
      dir,
      writer,
      searchers: Arc::new(SearcherLifetimeManager::new()),
      mgr,
      my_node_id: node_id,
      // Init w/ 0s... caller above will do initial
      // "broadcast" by calling initSearcher:
      current_node_versions: RwLock::new(vec![0; num_nodes]),
      collection_stats_cache: Mutex::new(HashMap::new()),
      term_stats_cache: Mutex::new(HashMap::new()),
      current_shard_searcher: RwLock::new(None),
      state,
    })
  }

  #[allow(unused)]
  pub fn init_searcher(&self, node_versions: &[i64]) -> Result<()> {
    let mut current_shard_searcher = self.current_shard_searcher.write();
    assert!(current_shard_searcher.is_none());
    self
      .current_node_versions
      .write()
      .copy_from_slice(node_versions);
    let source = self.mgr.acquire()?;
    *current_shard_searcher = Some(Arc::new(ShardIndexSearcher::new(
      self.current_node_versions.read().clone(),
      source.get_index_reader().clone(),
      self.my_node_id,
      self.state.clone(),
    )?));
    Ok(())
  }

  pub fn update_node_version(&self, node_id: usize, version: i64) -> Result<()> {
    self.current_node_versions.write()[node_id] = version;
    let mut current_shard_searcher = self.current_shard_searcher.write();
    if let Some(old_searcher) = current_shard_searcher.as_ref() {
      old_searcher.get_index_reader().dec_ref()?;
    }
    let source = self.mgr.acquire()?;
    *current_shard_searcher = Some(Arc::new(ShardIndexSearcher::new(
      self.current_node_versions.read().clone(),
      source.get_index_reader().clone(),
      self.my_node_id,
      self.state.clone(),
    )?));
    Ok(())
  }

  // Get the current (fresh) searcher for this node
  pub fn acquire(&self) -> Result<Arc<ShardIndexSearcher>> {
    loop {
      let searcher = self
        .current_shard_searcher
        .read()
        .clone()
        .ok_or_else(|| LuceneError::illegal_state("shard searcher is not initialized"))?;
      // In theory the reader could get decRef'd to 0
      // before we have a chance to incRef, ie if a reopen
      // happens right after the above line, this thread
      // gets stalled, and the old IR is closed. So we
      // must try/retry until incRef succeeds:
      if searcher.get_index_reader().try_inc_ref() {
        return Ok(searcher);
      }
    }
  }

  pub fn release(&self, searcher: Arc<ShardIndexSearcher>) -> Result<()> {
    searcher.get_index_reader().dec_ref()
  }

  // Get an old searcher matching the specified versions:
  pub fn acquire_versions(&self, node_versions: &[i64]) -> Result<Arc<ShardIndexSearcher>> {
    let source = self
      .searchers
      .acquire(node_versions[self.my_node_id])?
      .ok_or_else(|| {
        SearcherExpiredException::new(format!(
          "nodeID={} version={}",
          self.my_node_id, node_versions[self.my_node_id]
        ))
      })?;
    Ok(Arc::new(ShardIndexSearcher::new(
      node_versions.to_vec(),
      source.get_index_reader().clone(),
      self.my_node_id,
      self.state.clone(),
    )?))
  }

  // Reopen local reader
  pub fn reopen(&self) -> Result<()> {
    let before = self.mgr.acquire()?;
    self.mgr.release(before.clone())?;

    self.mgr.maybe_refresh()?;
    let after = self.mgr.acquire()?;
    let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      if !Arc::ptr_eq(&after, &before) {
        // New searcher was opened
        let version = self.searchers.record(&after)?;
        self.searchers.prune(&PruneByAge::new(
          *self
            .state
            .upgrade()
            .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?
            .max_searcher_age_seconds
            .read(),
        )?)?;
        self
          .state
          .upgrade()
          .ok_or_else(|| LuceneError::illegal_state("shard searching state was dropped"))?
          .broadcast_node_reopen(self.my_node_id, version, after.as_ref())?;
      }
      Ok(())
    }));
    let close_result = catch_unwind(AssertUnwindSafe(|| self.mgr.release(after)));
    IOUtils::finally_caught_result(body_result, close_result)
  }

  pub fn close(&self) -> Result<()> {
    if let Some(current) = self.current_shard_searcher.read().as_ref() {
      current.get_index_reader().dec_ref()?;
    }
    self.searchers.close()?;
    self.mgr.close()?;
    self.writer.close()?;
    self.dir.close()
  }
}

struct ChangeIndices {
  state: Arc<ShardSearchingState>,
  end_time: Instant,
  seed: u64,
}

impl ChangeIndices {
  fn run(self) -> Result<()> {
    let mut random = random_from_seed(self.seed);
    let mut docs = LineFileDocs::new(&mut random)?;
    let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      let mut num_docs = 0;
      while Instant::now() < self.end_time {
        let nodes = self.state.nodes();
        let what = random.random_range(0..3);
        let node = nodes[random.random_range(0..nodes.len())].clone();
        if num_docs == 0 || what == 0 {
          node.writer.add_document(docs.next_doc()?)?;
          num_docs += 1;
        } else if what == 1 {
          node.writer.update_document_with_term(
            Term::from_text("docid", random.random_range(0..num_docs).to_string()),
            docs.next_doc()?,
          )?;
          num_docs += 1;
        } else {
          node
            .writer
            .delete_documents_with_terms(vec![Term::from_text(
              "docid",
              random.random_range(0..num_docs).to_string(),
            )])?;
        }

        if random.random_range(0..17) == 12 {
          node.writer.commit()?;
        }

        if random.random_range(0..17) == 12 {
          nodes[random.random_range(0..nodes.len())].reopen()?;
        }
      }
      Ok(())
    }));
    let close_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
      docs.close();
      Ok(())
    }));
    IOUtils::use_or_suppress_caught_result(body_result, close_result)
  }
}

pub struct ShardSearchingTestContext {
  state: Arc<ShardSearchingState>,
}

impl Default for ShardSearchingTestContext {
  fn default() -> Self {
    Self::new()
  }
}

impl ShardSearchingTestContext {
  pub fn new() -> Self {
    Self {
      state: Arc::new(ShardSearchingState::new()),
    }
  }

  pub fn nodes(&self) -> Vec<Arc<NodeState>> {
    self.state.nodes()
  }

  pub fn end_time(&self) -> Instant {
    self
      .state
      .end_time
      .read()
      .expect("ShardSearchingTestContext has not been started")
  }

  pub fn start<R>(
    &self,
    random: &mut R,
    num_nodes: usize,
    run_time_sec: f64,
    max_searcher_age_seconds: i32,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let end_time = Instant::now() + Duration::from_secs_f64(run_time_sec);
    *self.state.end_time.write() = Some(end_time);
    *self.state.max_searcher_age_seconds.write() = max_searcher_age_seconds as f64;

    let mut nodes = Vec::with_capacity(num_nodes);
    for node_id in 0..num_nodes {
      nodes.push(Arc::new(NodeState::new(
        random,
        Arc::downgrade(&self.state),
        node_id,
        num_nodes,
      )?));
    }
    *self.state.nodes.write() = nodes.clone();

    let mut node_versions = vec![0; nodes.len()];
    for node_id in 0..nodes.len() {
      let searcher = nodes[node_id].mgr.acquire()?;
      let body_result = catch_unwind(AssertUnwindSafe(|| {
        nodes[node_id].searchers.record(&searcher)
      }));
      let close_result = catch_unwind(AssertUnwindSafe(|| nodes[node_id].mgr.release(searcher)));
      node_versions[node_id] = IOUtils::finally_caught_result(body_result, close_result)?;
    }

    for node_id in 0..nodes.len() {
      let searcher = nodes[node_id].mgr.acquire()?;
      assert_eq!(
        node_versions[node_id],
        nodes[node_id].searchers.record(&searcher)?
      );
      let body_result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        self
          .state
          .broadcast_node_reopen(node_id, node_versions[node_id], searcher.as_ref())
      }));
      let close_result = catch_unwind(AssertUnwindSafe(|| nodes[node_id].mgr.release(searcher)));
      IOUtils::finally_caught_result(body_result, close_result)?;
    }

    let state = self.state.clone();
    let seed = random.random();
    *self.state.change_indices_thread.lock() = Some(thread::spawn(move || {
      ChangeIndices {
        state,
        end_time,
        seed,
      }
      .run()
    }));
    Ok(())
  }

  pub fn finish(&self) -> Result<()> {
    let change_indices_thread = self
      .state
      .change_indices_thread
      .lock()
      .take()
      .expect("ShardSearchingTestContext has not been started");
    let thread_result = match change_indices_thread.join() {
      Ok(result) => result,
      Err(payload) => Err(LuceneError::tragedy_from_panic(
        "change indices thread panicked",
        payload.as_ref(),
      )),
    };
    let close_result = (|| -> Result<()> {
      for node in self.state.nodes() {
        node.close()?;
      }
      Ok(())
    })();
    close_result?;
    thread_result
  }
}

/// An [`IndexSearcher`] and associated version (lease).
#[allow(dead_code)]
pub struct SearcherAndVersion {
  pub searcher: Arc<ManagedSearcher>,
  pub version: i64,
}
