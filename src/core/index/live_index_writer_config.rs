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
use crate::core::codecs::{Codecs, codec};
use crate::core::index::flush_by_ram_or_counts_policy::FlushByRamOrCountsPolicy;
use crate::core::index::flush_policy::FlushPolicyEnum;
use crate::core::index::index_deletion_policy::IndexDeletionPolicyEnum;
use crate::core::index::index_writer::IndexReaderWarmerEnum;
use crate::core::index::index_writer_config::{
  DEFAULT_COMMIT_ON_CLOSE, DEFAULT_MAX_BUFFERED_DOCS, DEFAULT_MAX_FULL_FLUSH_MERGE_WAIT_MILLIS,
  DEFAULT_RAM_BUFFER_SIZE_MB, DEFAULT_RAM_PER_THREAD_HARD_LIMIT_MB, DEFAULT_READER_POOLING,
  DEFAULT_USE_COMPOUND_FILE_SYSTEM, OpenMode,
};
use crate::core::index::index_writer_event_listener::{
  IndexWriterEventListenerEnum, NoOpIndexWriterEventListener,
};
use crate::core::index::keep_only_last_commit_deletion_policy::KeepOnlyLastCommitDeletionPolicy;
use crate::core::index::merge_policy::MergePolicyEnum;
use crate::core::index::merge_scheduler::MergeSchedulerEnum;
use crate::core::index::tiered_merge_policy::TieredMergePolicy;
use crate::core::search::index_searcher::get_default_similarity;
use crate::core::search::similarities_impl::similarities::SimilarityEnum;
use crate::core::search::sort::Sort;
use crate::core::store::directory::Directory;
use crate::core::util::LATEST;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStreamEnum, InfoStreamMT, NoOutput};
use std::collections::HashSet;
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::util::comparator::Comparator;

/// Holds all configuration used by `IndexWriter`, with a small set of setters
/// for settings that can be changed on an `IndexWriter` instance live.
pub trait LiveIndexWriterConfig: Display {
  type Directory: Directory;

  /// Returns the default analyzer to use for indexing documents.
  fn get_analyzer(&self) -> &AnalyzerEnum;

  /// Expert: returns the [`SimilarityEnum`] implementation used by this
  /// `IndexWriter`.
  fn get_similarity(&self) -> &SimilarityEnum;

  /// Returns the [`MergeSchedulerEnum`] set on this configuration.
  fn get_merge_scheduler(&self) -> &MergeSchedulerEnum;

  /// Returns the current [`Codec`].
  fn get_codec(&self) -> &Codecs;

  /// Gets the index-time [`Sort`] order applied to all flushed and merged
  /// segments.
  fn get_index_sort(&self) -> Option<Arc<Sort>>;
  /// Returns the field names involved in the index sort.
  fn get_index_sort_fields(&self) -> &HashSet<String>;
  /// Returns `true` iff newly written segments are packed in a compound file.
  ///
  /// The default is `true`.
  fn get_use_compound_file(&self) -> bool;

  /// Returns the soft deletes field, or `None` if soft deletes are disabled.
  fn get_soft_deletes_field(&self) -> Option<&String>;

  /// Returns the [`InfoStreamMT`] used for debugging.
  fn get_info_stream(&self) -> InfoStreamMT;

  /// Returns the parent document field name if configured.
  fn get_parent_field(&self) -> Option<&String>;

  /// Returns the current [`MergePolicyEnum`] in use by this writer.
  fn get_merge_policy(&self) -> &MergePolicyEnum<Self::Directory>;
  /// Returns mutable access to the current [`MergePolicyEnum`].
  fn get_merge_policy_mut(&mut self) -> &mut MergePolicyEnum<Self::Directory>;

  /// Returns the [`FlushPolicyEnum`] used to control when segments are flushed.
  fn get_flush_policy(&self) -> &FlushPolicyEnum;

  /// Returns the RAM buffer size in MB if enabled.
  fn get_ram_buffer_size_mb(&self) -> f64;

  /// Returns the max amount of memory each documents writer thread can consume
  /// before it is forcefully flushed.
  fn get_ram_per_thread_hard_limit_mb(&self) -> i32;

  /// Returns the number of buffered added documents that will trigger a flush if
  /// document-count flushing is enabled.
  fn get_max_buffered_docs(&self) -> i32;

  /// Expert: returns whether indexing threads check for pending flushes on
  /// update in order to help flush indexing buffers to disk.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn get_check_pending_flush_on_update(&self) -> bool;

  /// Expert: sets whether indexing threads check for pending flushes on update
  /// in order to help flush indexing buffers to disk.
  ///
  /// As a consequence, threads that open a changed reader or flush the writer
  /// will be the only threads writing segments to disk unless flushes are
  /// falling behind. If indexing is stalled due to too many pending flushes,
  /// indexing threads will help write pending segment flushes to disk.
  ///
  /// Experimental: this API follows the original Lucene experimental status.
  fn set_check_pending_flush_update(&self, check_pending_flush_on_update: bool) -> &Self;

  /// Returns the [`IndexDeletionPolicyEnum`] specified on this configuration, or
  /// the default keep-only-last-commit deletion policy.
  fn get_index_deletion_policy(&self) -> &IndexDeletionPolicyEnum<Self::Directory>;

  /// Expert: returns the amount of time to wait for merges returned by
  /// `MergePolicy::find_full_flush_merges`.
  ///
  /// If this time is reached, commit proceeds based on segments merged up to
  /// that point. The merges are not cancelled and may still run to completion
  /// independent of the commit.
  fn get_max_full_flush_merge_wait_millis(&self) -> i64;

  /// Returns the [`IndexWriterEventListenerEnum`] callback that tracks the key
  /// `IndexWriter` operations.
  fn get_index_writer_event_listener(&self) -> &IndexWriterEventListenerEnum;

  /// Returns the current merged segment warmer.
  fn get_merged_segment_warmer(&self) -> Option<&IndexReaderWarmerEnum<Self::Directory>>;

  /// Returns `true` if `IndexWriter::close` should first commit before closing.
  fn get_commit_on_close(&self) -> bool;

  /// Returns the [`OpenMode`] that `IndexWriter` is opened with.
  fn get_open_mode(&self) -> &OpenMode;

  /// Returns the compatibility version to use for this index.
  fn get_index_created_version_major(&self) -> i32;

  /// Returns `true` if `IndexWriter` should pool readers even if opening a
  /// reader from the writer has not been called.
  fn get_reader_pooling(&self) -> bool;

  /// Returns mutable access to the base live configuration storage.
  fn get_base_mut(&mut self) -> &mut LiveIndexWriterConfigBase<Self::Directory>;

  /// Determines the amount of RAM that may be used for buffering added
  /// documents and deletions before they are flushed to the directory.
  ///
  /// Generally, for faster indexing performance it is best to flush by RAM
  /// usage instead of document count and use as large a RAM buffer as possible.
  ///
  /// When this is set, the writer will flush whenever buffered documents and
  /// deletions use this much RAM. Pass `DISABLE_AUTO_FLUSH` to prevent
  /// triggering a flush due to RAM usage. If flushing by document count is also
  /// enabled, the flush is triggered by whichever limit comes first.
  ///
  /// The maximum RAM limit is inherently determined by available memory. An
  /// `IndexWriter` session can consume significantly more memory than this
  /// limit, since the limit only indicates when to flush memory-resident
  /// documents. Flushes may happen concurrently while other threads add
  /// documents, so available memory should be significantly larger than the RAM
  /// buffer used for indexing.
  ///
  /// NOTE: RAM accounting for pending deletions is approximate. Deletes by query
  /// cannot measure individual query RAM usage, so accounting may
  /// underestimate; applications should compensate by committing or refreshing
  /// periodically if needed.
  ///
  /// NOTE: It is not guaranteed that all memory-resident documents are flushed
  /// once this limit is exceeded. Depending on the configured flush policy, only
  /// a subset of buffered documents may be flushed and only part of the RAM
  /// buffer released.
  ///
  /// The default value is `DEFAULT_RAM_BUFFER_SIZE_MB`.
  ///
  /// Takes effect immediately, but only the next time a document is added,
  /// updated, or deleted.
  fn set_ram_buffer_size_mb(&mut self, ram_buffer_size_mb: f64) -> &mut Self {
    self.get_base_mut().ram_buffer_size_mb = ram_buffer_size_mb;
    self
  }

  /// Determines the minimal number of documents required before buffered
  /// in-memory documents are flushed as a new segment.
  ///
  /// Large values generally give faster indexing. When this is set, the writer
  /// flushes every `max_buffered_docs` added documents. Pass
  /// `DISABLE_AUTO_FLUSH` to prevent triggering a flush due to the number of
  /// buffered documents. If flushing by RAM usage is also enabled, the flush is
  /// triggered by whichever limit comes first.
  ///
  /// Disabled by default because the writer flushes by RAM usage.
  ///
  /// Takes effect immediately, but only the next time a document is added,
  /// updated, or deleted.
  fn set_max_buffered_docs(&mut self, max_buffered_docs: i32) -> &mut Self {
    self.get_base_mut().max_buffered_docs = max_buffered_docs;
    self
  }

  /// Sets whether the `IndexWriter` should pack newly written segments in a
  /// compound file.
  ///
  /// The default is `true`. Use `false` for batch indexing with very large RAM
  /// buffer settings.
  ///
  /// NOTE: To control compound file usage during segment merges, use the
  /// corresponding merge policy settings. This setting only applies to newly
  /// created segments.
  fn set_use_compound_file(&mut self, use_compound_file: bool) -> &mut Self {
    self.get_base_mut().use_compound_file = use_compound_file;
    self
  }

  /// Expert: sets the [`MergePolicyEnum`] used whenever there are changes to
  /// the segments in the index.
  ///
  /// The merge policy selects which merges to do, if any, and also selects
  /// merges for force merge.
  ///
  /// Takes effect on subsequent merge selections. Any merges in flight or
  /// already registered by the previous merge policy are not affected.
  fn set_merge_policy<T>(&mut self, merge_policy: T) -> &mut Self
  where
    T: Into<MergePolicyEnum<Self::Directory>>,
  {
    let v = merge_policy.into();
    self.get_base_mut().merge_policy = v;
    self
  }

  /// Sets the [`FlushPolicyEnum`] used to control when segments are flushed.
  fn set_flush_policy<T>(&mut self, flush_policy: T) -> &mut Self
  where
    T: Into<FlushPolicyEnum>,
  {
    let v = flush_policy.into();
    self.get_base_mut().flush_policy = Arc::new(v);
    self
  }

  /// Sets the [`InfoStreamMT`] used for debugging.
  fn set_info_stream<T>(&mut self, info_stream: T) -> &mut Self
  where
    T: Into<InfoStreamMT>,
  {
    self.get_base_mut().info_stream = info_stream.into();
    self
  }

  /// Sets the merged segment warmer.
  ///
  /// Takes effect on the next merge.
  fn set_merged_segment_warmer(
    &mut self,
    merge_segment_warmer: Option<IndexReaderWarmerEnum<Self::Directory>>,
  ) -> &mut Self {
    self.get_base_mut().merged_segment_warmer = merge_segment_warmer;
    self
  }
}

/// Leaf sorter for sorting leaf readers, equivalent to Java's
/// `Comparator<LeafReader>`.
///
/// Implements [`Comparator<DefaultLeafReader<D>>`] via a closure held in
/// the `Custom` variant.
pub enum LeafSorter<D>
where
  D: Directory,
{
  /// A custom comparator provided by the user.
  ///
  /// Uses `Arc<dyn Fn>` because [`Comparator`] is not dyn-compatible
  /// (it has an associated constant `TYPE`).
  Custom(Arc<LeafReaderComparator<D>>),
}
impl<D> Comparator<DefaultLeafReader<D>> for LeafSorter<D>
where
  D: Directory,
{
  const TYPE: &'static str = "LeafSorter";

  fn compare(&self, a: &DefaultLeafReader<D>, b: &DefaultLeafReader<D>) -> Result<i32> {
    match self {
      LeafSorter::Custom(c) => c(a, b),
    }
  }
}

impl<D> Clone for LeafSorter<D>
where
  D: Directory,
{
  fn clone(&self) -> Self {
    match self {
      LeafSorter::Custom(c) => LeafSorter::Custom(Arc::clone(c)),
    }
  }
}

impl<D> LeafSorter<D>
where
  D: Directory,
{
  /// Creates a new `LeafSorter` with a custom comparator.
  pub fn custom(comparator: Arc<LeafReaderComparator<D>>) -> Self {
    LeafSorter::Custom(comparator)
  }
}

type LeafReaderComparator<D> =
  dyn Fn(&DefaultLeafReader<D>, &DefaultLeafReader<D>) -> Result<i32> + Send + Sync;
/// Storage for live index writer configuration values.
///
/// These fields mirror the live configuration state that an `IndexWriter` reads
/// while indexing and merging.
pub struct LiveIndexWriterConfigBase<D>
where
  D: Directory,
{
  /// Directory type marker for the `IndexWriter` using this configuration.
  _mark: PhantomData<D>,
  /// Default analyzer to use for indexing documents.
  pub analyzer: AnalyzerEnum,
  /// RAM buffer size in MB for added documents and deletions before flushing.
  pub ram_buffer_size_mb: f64,
  /// Number of buffered added documents that triggers a flush.
  pub max_buffered_docs: i32,
  /// [`IndexDeletionPolicyEnum`] controlling when commit points are deleted.
  pub index_deletion_policy: IndexDeletionPolicyEnum<D>,
  /// True if newly written segment flushes should use compound file format.
  pub use_compound_file: bool,
  /// [`OpenMode`] that `IndexWriter` is opened with.
  pub open_mode: OpenMode,
  /// [`SimilarityEnum`] to use when encoding norms.
  pub similarity: Arc<SimilarityEnum>,
  /// [`Codec`] used to write new segments.
  pub codec: Codecs,
  /// [`InfoStreamMT`] for debugging messages.
  pub info_stream: InfoStreamMT,
  /// [`MergePolicyEnum`] for selecting merges.
  pub merge_policy: MergePolicyEnum<D>,
  /// [`FlushPolicyEnum`] to control when segments are flushed.
  pub flush_policy: Arc<FlushPolicyEnum>,
  /// True if readers should be pooled.
  pub reader_pooling: bool,
  /// Hard upper bound on RAM usage for a single thread, after which the segment
  /// is forced to flush.
  pub per_thread_hard_limit_mb: i32,
  /// Compatibility version to use for this index.
  pub created_version_major: i32,
  /// Soft deletes field, or `None` if soft deletes are disabled.
  pub soft_deletes_field: Option<String>,
  /// Amount of time to wait for merges returned by full-flush merge selection.
  pub max_full_flush_merge_wait_millis: i64,
  /// [`IndexWriterEventListenerEnum`] for recording key `IndexWriter` events.
  pub event_listener: IndexWriterEventListenerEnum,
  /// Warmer called for newly merged segments before they are committed.
  pub merged_segment_warmer: Option<IndexReaderWarmerEnum<D>>,
  /// True if calls to `IndexWriter::close` should first do a commit.
  pub commit_on_close: bool,
  /// True if an indexing thread should check for pending flushes on update in
  /// order to help with a full flush.
  pub check_pending_flush_on_update: AtomicBool,
  /// Parent document field name.
  pub parent_field: Option<String>,
  /// Sort order to use to write merged segments.
  pub index_sort: Option<Arc<Sort>>,
  /// Field names involved in the index sort.
  pub index_sort_fields: HashSet<String>,
  /// [`MergeSchedulerEnum`] to use for running merges.
  pub merge_scheduler: MergeSchedulerEnum,
  /// Comparator for sorting leaf readers.
  pub leaf_sorter: Option<LeafSorter<D>>,
}
impl<D> LiveIndexWriterConfigBase<D>
where
  D: Directory,
{
  pub fn with_analyzer<T>(analyzer: T) -> Result<Self>
  where
    T: Into<AnalyzerEnum>,
  {
    let mut v = Self::new()?;
    v.analyzer = analyzer.into();
    Ok(v)
  }
  pub fn new() -> Result<Self> {
    Ok(Self {
      _mark: PhantomData,
      analyzer: AnalyzerEnum::default(),
      ram_buffer_size_mb: DEFAULT_RAM_BUFFER_SIZE_MB,
      max_buffered_docs: DEFAULT_MAX_BUFFERED_DOCS,
      index_deletion_policy: KeepOnlyLastCommitDeletionPolicy.into(),
      use_compound_file: DEFAULT_USE_COMPOUND_FILE_SYSTEM,
      open_mode: OpenMode::CreateOrAppend,
      similarity: Arc::new(get_default_similarity()?),
      codec: codec::get_default(),
      info_stream: Arc::new(InfoStreamEnum::NoOutput(NoOutput)),
      merge_policy: MergePolicyEnum::Tiered(TieredMergePolicy::default()),
      flush_policy: Arc::new(FlushByRamOrCountsPolicy::new().into()),
      reader_pooling: DEFAULT_READER_POOLING,
      per_thread_hard_limit_mb: DEFAULT_RAM_PER_THREAD_HARD_LIMIT_MB,
      created_version_major: LATEST.major,
      soft_deletes_field: None,
      max_full_flush_merge_wait_millis: DEFAULT_MAX_FULL_FLUSH_MERGE_WAIT_MILLIS,
      event_listener: NoOpIndexWriterEventListener.into(),
      merged_segment_warmer: None,
      commit_on_close: DEFAULT_COMMIT_ON_CLOSE,
      check_pending_flush_on_update: AtomicBool::new(true),
      parent_field: None,
      index_sort: None,
      index_sort_fields: HashSet::new(),
      merge_scheduler: MergeSchedulerEnum::default(),
      leaf_sorter: None,
    })
  }

  pub fn get_flush_policy(&self) -> &FlushPolicyEnum {
    &self.flush_policy
  }
}
