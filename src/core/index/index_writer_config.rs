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
use crate::core::analysis::analyzer::AnalyzerEnum;
use crate::core::codecs::lucene101_codec::Lucene101Codec;
use crate::core::index::flush_policy::FlushPolicyEnum;
use crate::core::index::index_deletion_policy::IndexDeletionPolicyEnum;
use crate::core::index::live_index_writer_config::{
  LiveIndexWriterConfig, LiveIndexWriterConfigBase,
};
use crate::core::index::merge_policy::MergePolicyEnum;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::search::similarities_impl::similarities::SimilarityEnum;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::SortFiledBase;
use crate::core::util::LATEST;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::InfoStreamMT;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct IndexWriterConfig {
  pub(crate) base: LiveIndexWriterConfigBase,
}
impl Default for IndexWriterConfig {
  fn default() -> Self {
    Self::new()
  }
}

impl IndexWriterConfig {
  pub fn new() -> Self {
    Self {
      base: LiveIndexWriterConfigBase::new(),
    }
  }
  pub fn with_analyzer<T>(analyzer: T) -> Self
  where
    T: Into<AnalyzerEnum>,
  {
    Self {
      base: LiveIndexWriterConfigBase::with_analyzer(analyzer),
    }
  }
  pub fn set_commit_on_close(&mut self, commit_on_close: bool) -> &mut Self {
    self.base.commit_on_close = commit_on_close;
    self
  }
  pub fn set_max_full_flush_merge_wait_millis(
    &mut self,
    max_full_flush_merge_wait_millis: i64,
  ) -> &mut Self {
    self.base.max_full_flush_merge_wait_millis = max_full_flush_merge_wait_millis;
    self
  }
  pub fn set_open_mode(&mut self, open_mode: OpenMode) -> &mut Self {
    let base = self.get_base_mut();
    base.open_mode = open_mode;
    self
  }

  pub fn set_similarity<T>(&mut self, similarity: T)
  where
    T: Into<SimilarityEnum>,
  {
    self.base.similarity = Arc::new(similarity.into());
  }
  pub fn set_index_created_version_major(
    &mut self,
    index_created_version_major: i32,
  ) -> Result<&mut Self> {
    if index_created_version_major > LATEST.major {
      return Err(LuceneError::illegal_argument(format!(
        "indexCreatedVersionMajor may not be in the future: current major version is {}, but got: {}",
        LATEST.major, index_created_version_major
      )));
    }

    if index_created_version_major < LATEST.major - 1 {
      return Err(LuceneError::illegal_argument(format!(
        "indexCreatedVersionMajor may not be less than the minimum supported version: {}, but got: {}",
        LATEST.major - 1,
        index_created_version_major
      )));
    }

    self.base.created_version_major = index_created_version_major;
    Ok(self)
  }

  pub fn set_index_deletion_policy<T>(&mut self, deletion_policy: T) -> &mut Self
  where
    T: Into<IndexDeletionPolicyEnum>,
  {
    self.base.index_deletion_policy = deletion_policy.into();
    self
  }

  pub fn set_index_sort<T>(&mut self, sort: T) -> Result<&mut Self>
  where
    T: Into<Arc<Sort>>,
  {
    let sort = sort.into();
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
    self.base.index_sort_fields = index_sort_fields;
    self.base.index_sort = Some(sort);
    Ok(self)
  }

  pub fn set_merge_scheduler<T>(&mut self, merge_scheduler: T) -> &mut Self
  where
    T: Into<MergeSchedulerEnum>,
  {
    let v = merge_scheduler.into();
    self.base.merge_scheduler = v;
    self
  }
  pub fn set_soft_deletes_field<T>(&mut self, soft_deletes_field: T) -> &mut Self
  where
    T: Into<String>,
  {
    let v = soft_deletes_field.into();
    self.base.soft_deletes_field = Some(v);
    self
  }

  pub fn set_parent_field<T>(&mut self, parent_field: T) -> &mut Self
  where
    T: Into<String>,
  {
    let v = parent_field.into();
    self.base.parent_field = Some(v);
    self
  }
}

impl Display for IndexWriterConfig {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl LiveIndexWriterConfig for IndexWriterConfig {
  fn get_analyzer(&self) -> &AnalyzerEnum {
    &self.base.analyzer
  }

  fn get_similarity(&self) -> &SimilarityEnum {
    self.base.similarity.as_ref()
  }

  fn get_merge_scheduler(&self) -> &MergeSchedulerEnum {
    &self.base.merge_scheduler
  }

  type Codec = Lucene101Codec;

  fn get_codec(&self) -> &Self::Codec {
    &self.base.codec
  }

  fn get_index_sort(&self) -> Option<Arc<Sort>> {
    self.base.index_sort.clone()
  }

  fn get_index_sort_fields(&self) -> &HashSet<String> {
    &self.base.index_sort_fields
  }

  fn get_use_compound_file(&self) -> bool {
    self.base.use_compound_file
  }

  fn get_soft_deletes_field(&self) -> Option<&String> {
    self.base.soft_deletes_field.as_ref()
  }

  fn get_info_stream(&self) -> InfoStreamMT {
    self.base.info_stream.clone()
  }

  fn get_parent_field(&self) -> Option<&String> {
    self.base.parent_field.as_ref()
  }

  fn get_merge_policy(&self) -> &MergePolicyEnum {
    &self.base.merge_policy
  }
  fn get_merge_policy_mut(&mut self) -> &mut MergePolicyEnum {
    &mut self.base.merge_policy
  }
  fn get_flush_policy(&self) -> &FlushPolicyEnum {
    self.base.get_flush_policy()
  }

  fn get_ram_buffer_size_mb(&self) -> f64 {
    self.base.ram_buffer_size_mb
  }

  fn get_ram_per_thread_hard_limit_mb(&self) -> i32 {
    self.base.per_thread_hard_limit_mb
  }

  fn get_max_buffered_docs(&self) -> i32 {
    self.base.max_buffered_docs
  }

  fn get_check_pending_flush_on_update(&self) -> bool {
    self.base.check_pending_flush_on_update
  }

  fn get_index_deletion_policy(&self) -> &IndexDeletionPolicyEnum {
    &self.base.index_deletion_policy
  }

  fn get_max_full_flush_merge_wait_millis(&self) -> i64 {
    self.base.max_full_flush_merge_wait_millis
  }

  fn get_commit_on_close(&self) -> bool {
    self.base.commit_on_close
  }

  fn get_open_mode(&self) -> &OpenMode {
    &self.base.open_mode
  }

  fn get_index_created_version_major(&self) -> i32 {
    self.base.created_version_major
  }

  fn get_reader_pooling(&self) -> bool {
    self.base.reader_pooling
  }

  fn get_base_mut(&mut self) -> &mut LiveIndexWriterConfigBase {
    &mut self.base
  }
}

/// Specifies the open mode for [`IndexWriter`](crate::core::index::index_writer::IndexWriter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
  /// Creates a new index or overwrites an existing one.
  Create,

  /// Opens an existing index.
  Append,

  /// Creates a new index if one does not exist, otherwise it opens the index
  /// and documents will be appended.
  CreateOrAppend,
}

/// Denotes a flush trigger is disabled.
pub const DISABLE_AUTO_FLUSH: i32 = -1;

/// Disabled by default (because IndexWriter flushes by RAM usage by default).
pub const DEFAULT_MAX_BUFFERED_DELETE_TERMS: i32 = DISABLE_AUTO_FLUSH;

/// Disabled by default (because IndexWriter flushes by RAM usage by default).
pub const DEFAULT_MAX_BUFFERED_DOCS: i32 = DISABLE_AUTO_FLUSH;

/// Default value is 16 MB (which means flush when buffered docs consume approximately 16 MB RAM).
pub const DEFAULT_RAM_BUFFER_SIZE_MB: f64 = 16.0;

/// Default setting (true) for `set_reader_pooling`.
///
/// We changed this default to true with concurrent deletes/updates (LUCENE-7868),
/// because we will otherwise need to open and close segment readers more frequently.
/// False is still supported, but will have worse performance since readers will
/// be forced to aggressively move all state to disk.
pub const DEFAULT_READER_POOLING: bool = true;

/// Default value is 1945. Change using `set_ram_per_thread_hard_limit_mb`.
pub const DEFAULT_RAM_PER_THREAD_HARD_LIMIT_MB: i32 = 1945;

/// Default value for compound file system for newly written segments (set to `true`).
/// For batch indexing with very large ram buffers use `false`.
pub const DEFAULT_USE_COMPOUND_FILE_SYSTEM: bool = true;

/// Default value for whether calls to `IndexWriter::close` include a commit.
pub const DEFAULT_COMMIT_ON_CLOSE: bool = true;

/// Default value for time to wait for merges on commit or getReader (when using a
/// [`MergePolicy`](crate::core::index::merge_policy::MergePolicy) that implements [`MergePolicy::find_full_flush_merges`](crate::core::index::merge_policy::MergePolicy::find_full_flush_merges)).
pub const DEFAULT_MAX_FULL_FLUSH_MERGE_WAIT_MILLIS: i64 = 500;
