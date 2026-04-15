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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerEnum};
use crate::core::codecs::Codec;
use crate::core::codecs::lucene101_codec::Lucene101Codec;
use crate::core::index::dummy::dummy_index_commit::DummyIndexCommit;
use crate::core::index::flush_by_ram_or_counts_policy::FlushByRamOrCountsPolicy;
use crate::core::index::flush_policy::FlushPolicy;
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_deletion_policy::IndexDeletionPolicy;
use crate::core::index::index_writer_config::{
  DEFAULT_COMMIT_ON_CLOSE, DEFAULT_MAX_BUFFERED_DOCS, DEFAULT_MAX_FULL_FLUSH_MERGE_WAIT_MILLIS,
  DEFAULT_RAM_BUFFER_SIZE_MB, DEFAULT_RAM_PER_THREAD_HARD_LIMIT_MB, DEFAULT_READER_POOLING,
  DEFAULT_USE_COMPOUND_FILE_SYSTEM, OpenMode,
};
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::merge_policy::{MergePolicy, MergePolicyEnum};
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSchedulerEnum};
use crate::core::index::tiered_merge_policy::TieredMergePolicy;
use crate::core::search::index_searcher::default_similarity;
use crate::core::search::similarities_impl::similarities::{Similarity, SimilarityEnum};
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::SortFiledBase;
use crate::core::store::dummy::dummy_directory::DummyDirectory;
use crate::core::util::LATEST;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStreamEnum, InfoStreamMT, NoOutput};
use std::collections::HashSet;
use std::fmt::Display;
use std::sync::Arc;

pub trait LiveIndexWriterConfig: Display {
  fn set_open_mode(&mut self, open_mode: OpenMode) -> &mut Self {
    let base = self.get_base_mut();
    base.open_mode = open_mode;
    self
  }

  type Analyzer: Analyzer;
  fn get_analyzer(&self) -> &Self::Analyzer;

  type Similarity: Similarity;
  fn get_similarity(&self) -> &Self::Similarity;

  type MergeScheduler: MergeScheduler;
  fn get_merge_scheduler(&self) -> &Self::MergeScheduler;

  type Codec: Codec;
  fn get_codec(&self) -> &Self::Codec;

  fn get_index_sort(&self) -> Option<Arc<Sort>>;
  fn get_index_sort_fields(&self) -> &HashSet<String>;

  fn get_use_compound_file(&self) -> bool;

  fn get_soft_deletes_field(&self) -> Option<&String>;

  fn get_info_stream(&self) -> InfoStreamMT;

  fn get_parent_field(&self) -> Option<&String>;

  type MergePolicy: MergePolicy;
  fn get_merge_policy(&self) -> &Self::MergePolicy;
  fn get_merge_policy_mut(&mut self) -> &mut Self::MergePolicy;

  type FlushPolicy: FlushPolicy;
  fn get_flush_policy(&self) -> &Self::FlushPolicy;

  fn get_ram_buffer_size_mb(&self) -> f64;

  fn get_ram_per_thread_hard_limit_mb(&self) -> i32;

  fn get_max_buffered_docs(&self) -> i32;

  fn get_check_pending_flush_on_update(&self) -> bool;

  type IndexDeletionPolicy: IndexDeletionPolicy;
  fn get_index_deletion_policy(&self) -> &Self::IndexDeletionPolicy;

  fn get_max_full_flush_merge_wait_millis(&self) -> i64;

  fn set_max_full_flush_merge_wait_millis(
    &mut self,
    max_full_flush_merge_wait_millis: i64,
  ) -> &mut Self {
    self.get_base_mut().max_full_flush_merge_wait_millis = max_full_flush_merge_wait_millis;
    self
  }

  fn get_commit_on_close(&self) -> bool;

  fn get_open_mode(&self) -> &OpenMode;

  type IndexCommit: IndexCommit;
  fn get_index_commit(&mut self) -> Option<Self::IndexCommit>;

  fn get_index_created_version_major(&self) -> i32;

  fn get_reader_pooling(&self) -> bool;
  fn get_base_mut(&mut self) -> &mut LiveIndexWriterConfigBase;

  fn set_ram_buffer_size_mb(&mut self, ram_buffer_size_mb: f64) -> &mut Self {
    self.get_base_mut().ram_buffer_size_mb = ram_buffer_size_mb;
    self
  }

  fn set_max_buffered_docs(&mut self, max_buffered_docs: i32) -> &mut Self {
    self.get_base_mut().max_buffered_docs = max_buffered_docs;
    self
  }

  fn set_use_compound_file(&mut self, use_compound_file: bool) -> &mut Self {
    self.get_base_mut().use_compound_file = use_compound_file;
    self
  }

  fn set_index_sort<T>(&mut self, sort: T) -> Result<&mut Self>
  where
    T: Into<Arc<Sort>>,
  {
    let sort = sort.into();
    let base = self.get_base_mut();
    for sort_field in sort.get_sort() {
      if sort_field.get_index_sorter()?.is_none() {
        return Err(LuceneError::illegal_argument(format!(
          "Cannot sort index with sort field {}",
          sort_field
        )));
      }
    }
    let index_sort_fields: HashSet<String> = sort
      .get_sort()
      .iter()
      .filter_map(|f| f.get_field())
      .map(str::to_string)
      .collect();
    base.index_sort_fields = index_sort_fields;
    base.index_sort = Some(sort);
    Ok(self)
  }

  fn set_merge_policy<T>(&mut self, merge_policy: T) -> &mut Self
  where
    T: Into<MergePolicyEnum>,
  {
    let v = merge_policy.into();
    self.get_base_mut().merge_policy = v;
    self
  }
  fn set_merge_scheduler<T>(&mut self, merge_scheduler: T) -> &mut Self
  where
    T: Into<MergeSchedulerEnum>,
  {
    let v = merge_scheduler.into();
    self.get_base_mut().merge_scheduler = v;
    self
  }

  fn set_parent_field<T>(&mut self, parent_field: T) -> &mut Self
  where
    T: Into<String>,
  {
    let v = parent_field.into();
    self.get_base_mut().parent_field = Some(v);
    self
  }
}

pub struct LiveIndexWriterConfigBase {
  pub analyzer: AnalyzerEnum,
  pub ram_buffer_size_mb: f64,
  pub max_buffered_docs: i32,
  pub index_deletion_policy: KeepOnlyLastCommitDeletionPolicy,
  pub index_commit: Option<DummyIndexCommit<DummyDirectory>>,
  pub use_compound_file: bool,
  pub open_mode: OpenMode,
  pub similarity: Arc<SimilarityEnum>,
  pub codec: Lucene101Codec,
  pub info_stream: InfoStreamMT,
  pub merge_policy: MergePolicyEnum,
  pub flush_policy: Arc<FlushByRamOrCountsPolicy>,
  pub reader_pooling: bool,
  pub per_thread_hard_limit_mb: i32,
  pub created_version_major: i32,
  pub soft_deletes_field: Option<String>,
  pub max_full_flush_merge_wait_millis: i64,
  pub commit_on_close: bool,
  pub check_pending_flush_on_update: bool,
  pub parent_field: Option<String>,
  pub index_sort: Option<Arc<Sort>>,
  pub index_sort_fields: HashSet<String>,
  pub merge_scheduler: MergeSchedulerEnum,
}
impl Default for LiveIndexWriterConfigBase {
  fn default() -> Self {
    Self::new()
  }
}

impl LiveIndexWriterConfigBase {
  pub fn with_analyzer<T>(analyzer: T) -> Self
  where
    T: Into<AnalyzerEnum>,
  {
    let mut v = Self::new();
    v.analyzer = analyzer.into();
    v
  }
  pub fn new() -> Self {
    Self {
      analyzer: AnalyzerEnum::default(),
      ram_buffer_size_mb: DEFAULT_RAM_BUFFER_SIZE_MB,
      max_buffered_docs: DEFAULT_MAX_BUFFERED_DOCS,
      index_deletion_policy: KeepOnlyLastCommitDeletionPolicy,
      index_commit: None,
      use_compound_file: DEFAULT_USE_COMPOUND_FILE_SYSTEM,
      open_mode: OpenMode::CreateOrAppend,
      similarity: Arc::new(default_similarity()),
      codec: Lucene101Codec,
      info_stream: Arc::new(InfoStreamEnum::NoOutput(NoOutput)),
      merge_policy: MergePolicyEnum::Tiered(TieredMergePolicy::default()),
      flush_policy: Arc::new(FlushByRamOrCountsPolicy::new()),
      reader_pooling: DEFAULT_READER_POOLING,
      per_thread_hard_limit_mb: DEFAULT_RAM_PER_THREAD_HARD_LIMIT_MB,
      created_version_major: LATEST.major,
      soft_deletes_field: None,
      max_full_flush_merge_wait_millis: DEFAULT_MAX_FULL_FLUSH_MERGE_WAIT_MILLIS,
      commit_on_close: DEFAULT_COMMIT_ON_CLOSE,
      check_pending_flush_on_update: true,
      parent_field: None,
      index_sort: None,
      index_sort_fields: HashSet::new(),
      // TODO IMPORTANT 这里的默认不对
      merge_scheduler: MergeSchedulerEnum::default(),
    }
  }
}
