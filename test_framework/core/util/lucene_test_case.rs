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
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::fmt;
use std::io::ErrorKind;
use std::sync::Arc;

use crate::core::analysis::analyzer::AnalyzerEnum;
use crate::core::document::field::{Field, FieldDataEnum, Store};
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::{CompositeReader, get_context};
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::{LogMergePolicy, LogMergePolicyBase};
use crate::core::index::merge_policy::{MergePolicy, MergePolicyEnum};
use crate::core::index::no_deletion_policy::NoDeletionPolicy;
use crate::core::index::snapshot_deletion_policy::SnapshotDeletionPolicy;
use crate::core::index::tiered_merge_policy::TieredMergePolicy;
use crate::core::search::index_searcher::{DefaultIndexSearcher, IndexSearcher};
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::store::flush_info::FlushInfo;
use crate::core::store::fs_directory_base::FSDirectoryBaseEnum;
use crate::core::store::lock_factory::{LockFactory, LockFactoryEnum};
use crate::core::store::merge_info::MergeInfo;
use crate::core::store::nio_fs_directory::NIOFSDirectory;
use crate::core::store::single_instance_lock_factory::SingleInstanceLockFactory;
use crate::core::store::{
  ByteBuffersDirectory, FSDirectory, IO_CONTEXT_DEFAULT, IO_CONTEXT_READ_ONCE, IOContext,
};
use crate::core::util::SliceCopyOps;
use crate::core::util::access::SharedAccessVec;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::store::mock_directory_wrapper::MockDirectoryWrapper;
use crate::test_framework::core::util::lucene_test_case::EnvConfig::{
  Multiplier, NightMode, TestSeed,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use tempfile::TempDir;

#[allow(dead_code)] // for quick search
pub struct LuceneTestCase;

pub(crate) fn maybe_change_live_index_writer_config<R, C>(
  _random: &mut R,
  _config: &mut C,
) -> Result<()>
where
  R: Rng + ?Sized,
  C: LiveIndexWriterConfig + ?Sized,
{
  Ok(())
}
/// Describes the currently supported environment variables used to control
/// Lucene tests.
///
/// Each variant corresponds to an environment variable that configures specific
/// behaviors of the tests. For example, environment variables can be used to
/// control the test mode, random number generator seed, etc.
#[derive(Debug, Clone, Copy)]
pub enum EnvConfig {
  NightMode,
  Multiplier,
  TestSeed,
}

impl fmt::Display for EnvConfig {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let key = match self {
      NightMode => "tests.nightly",
      Multiplier => "tests.multiplier",
      TestSeed => "tests.seed",
    };
    write!(f, "{}", key)
  }
}

pub type FSDirectories = FSDirectory<LockFactoryEnum, FSDirectoryBaseEnum>;

pub const DEFAULT_LINE_DOCS_FILE: &str = "europarl.lines.txt.gz";

pub(crate) fn random_multiplier() -> i32 {
  let multiplier = std::env::var(Multiplier.to_string()).ok();

  multiplier
    .and_then(|v| v.parse::<i32>().ok())
    .unwrap_or(default_random_multiplier())
}

fn default_random_multiplier() -> i32 {
  if is_night_mode() { 2 } else { 1 }
}

pub fn get_only_leaf_reader<CR>(reader: CR) -> Result<<CR as CompositeReader>::LeafReader>
where
  CR: CompositeReader,
{
  let irc = get_context(reader)?;
  let sub_readers = irc.leaves()?;
  if sub_readers.len() != 1 {
    return Err(LuceneError::illegal_argument(format!(
      "{} has {} segments instead of exactly one",
      irc.reader(),
      sub_readers.len()
    )));
  }
  Ok(sub_readers[0].reader().clone())
}
pub(crate) fn at_least_usize<R>(random: &mut R, i: usize) -> usize
where
  R: Rng + ?Sized,
{
  debug_assert!(i <= i32::MAX as usize);
  at_least(random, i as i32) as usize
}
/// Returns a number of at least `i`
///
/// The actual number returned will be influenced by whether `TEST_NIGHTLY` is
/// active and `RANDOM_MULTIPLIER`, but also with some random fudge.
pub(crate) fn at_least<R>(random: &mut R, i: i32) -> i32
where
  R: Rng + ?Sized,
{
  let min = i * random_multiplier();
  let max = min + (min / 2);
  TestUtil::next_int(random, min, max)
}

pub(crate) fn rarely<R>(random: &mut R) -> bool
where
  R: Rng + ?Sized,
{
  let mut p = if is_night_mode() { 5 } else { 1 };
  p += (p as f64 * (random_multiplier() as f64).ln()).round() as i32;
  let min = 100 - p.min(20); // Never more than 20% chance
  random.random_range(0..100) >= min
}
pub(crate) fn usually<R>(random: &mut R) -> bool
where
  R: Rng + ?Sized,
{
  !rarely(random)
}

pub(crate) fn new_index_writer_config<D, R>(random: &mut R) -> Result<IndexWriterConfig<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  // TODO: 这里简单返回IndexWriterConfig::new()，后续可以根据random随机生成不同的配置
  let mock = MockAnalyzer::new(random);
  new_index_writer_config_with_analyzer(random, mock)
}
pub(crate) fn new_index_writer_config_with_analyzer<D, T, R>(
  _random: &mut R,
  analyzer: T,
) -> Result<IndexWriterConfig<D>>
where
  D: Directory,
  R: Rng + ?Sized,
  T: Into<AnalyzerEnum>,
{
  // TODO: 这里简单返回IndexWriterConfig::with_analyzer()，后续可以根据random随机生成不同的配置
  IndexWriterConfig::with_analyzer(analyzer)
}

/// Creates a new index writer config with a snapshot deletion policy.
pub(crate) fn new_snapshot_index_writer_config<D, R>(random: &mut R) -> Result<IndexWriterConfig<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let mut config = new_index_writer_config(random)?;
  config.set_index_deletion_policy(SnapshotDeletionPolicy::new(NoDeletionPolicy));
  Ok(config)
}

pub fn new_merge_policy<D, R>(r: &mut R) -> Result<MergePolicyEnum<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  // TODO
  Ok(new_tiered_merge_policy(r).into())
}
pub fn new_merge_policy_with_mock_mp<D, R>(
  r: &mut R,
  _include_mock_mp: bool,
) -> Result<MergePolicyEnum<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  // TODO
  Ok(new_tiered_merge_policy(r).into())
}
pub fn new_tiered_merge_policy<R>(_r: &mut R) -> TieredMergePolicy
where
  R: Rng + ?Sized,
{
  // TODO
  TieredMergePolicy::new()
}
pub fn new_log_merge_policy_with_merge_factor_cfs<D, R>(
  r: &mut R,
  use_cfs: bool,
  merge_factor: i32,
) -> Result<MergePolicyEnum<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let lomp = new_log_merge_policy::<D, R>(r)?;
  let ratio = if use_cfs { 1.0 } else { 0.0 };
  match lomp {
    MergePolicyEnum::LogDoc(mut log_doc) => {
      MergePolicy::<D>::get_base_mut(&mut log_doc).set_no_cfs_ratio(ratio)?;
      log_doc.set_merge_factor(merge_factor as usize)?;
      Ok(log_doc.into())
    },
    MergePolicyEnum::LogBytesSize(mut log_bytes_size) => {
      MergePolicy::<D>::get_base_mut(&mut log_bytes_size).set_no_cfs_ratio(ratio)?;
      log_bytes_size.set_merge_factor(merge_factor as usize)?;
      Ok(log_bytes_size.into())
    },
    _ => Err(LuceneError::illegal_argument(
      "Expected a LogMergePolicyEnum variant",
    )),
  }
}
pub fn new_log_merge_policy_with_merge_factor<D, R>(
  r: &mut R,
  merge_factor: i32,
) -> Result<MergePolicyEnum<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let lomp = new_log_merge_policy::<D, R>(r)?;
  match lomp {
    MergePolicyEnum::LogDoc(mut log_doc) => {
      log_doc.set_merge_factor(merge_factor as usize)?;
      Ok(log_doc.into())
    },
    MergePolicyEnum::LogBytesSize(mut log_bytes_size) => {
      log_bytes_size.set_merge_factor(merge_factor as usize)?;
      Ok(log_bytes_size.into())
    },
    _ => Err(LuceneError::illegal_argument(
      "Expected a LogMergePolicyEnum variant",
    )),
  }
}
pub fn new_log_merge_policy<D, R>(r: &mut R) -> Result<MergePolicyEnum<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let logmp = if r.random_bool(0.5) {
    let mut v = LogMergePolicy::log_doc();
    set_meta::<D, R>(r, &mut v)?;
    v.into()
  } else {
    let mut v = LogMergePolicy::log_bytes_size();
    set_meta::<D, R>(r, &mut v)?;
    v.into()
  };

  Ok(logmp)
}
pub fn new_log_merge_policy_with_cfs<D, R>(r: &mut R, use_cfs: bool) -> Result<MergePolicyEnum<D>>
where
  D: Directory,
  R: Rng + ?Sized,
{
  let ratio = if use_cfs { 1.0 } else { 0.0 };
  let logmp = if r.random_bool(0.5) {
    let mut v = LogMergePolicy::log_doc();
    MergePolicy::<D>::get_base_mut(&mut v).set_no_cfs_ratio(ratio)?;
    v.into()
  } else {
    let mut v = LogMergePolicy::log_bytes_size();
    MergePolicy::<D>::get_base_mut(&mut v).set_no_cfs_ratio(ratio)?;
    set_meta::<D, R>(r, &mut v)?;
    v.into()
  };

  Ok(logmp)
}
fn set_meta<D, R>(r: &mut R, mp: &mut LogMergePolicy<impl LogMergePolicyBase>) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
{
  mp.set_calibrate_size_by_deletes(r.random_bool(0.5));
  mp.set_target_search_concurrency(TestUtil::next_int(r, 1, 16))?;

  if rarely(r) {
    mp.set_merge_factor(TestUtil::next_usize(r, 2, 9))?;
  } else {
    mp.set_merge_factor(TestUtil::next_usize(r, 10, 50))?;
  }

  configure_random::<D, R, _>(r, mp)
}
fn configure_random<D, R, MP>(r: &mut R, merge_policy: &mut MP) -> Result<()>
where
  D: Directory,
  R: Rng + ?Sized,
  MP: MergePolicy<D>,
{
  if r.random_bool(0.5) {
    merge_policy
      .get_base_mut()
      .set_no_cfs_ratio(0.1 + r.random::<f64>() * 0.8)?;
  } else {
    merge_policy
      .get_base_mut()
      .set_no_cfs_ratio(if r.random_bool(0.5) { 1.0 } else { 0.0 })?;
  }

  if rarely(r) {
    merge_policy
      .get_base_mut()
      .set_max_cfs_segment_size_mb(0.2 + r.random::<f64>() * 2.0)?;
  } else {
    merge_policy
      .get_base_mut()
      .set_max_cfs_segment_size_mb(f64::INFINITY)?;
  }

  Ok(())
}

pub(crate) fn new_maybe_virus_checking_directory<R>(random: &mut R) -> Result<Arc<DirEnum>>
where
  R: Rng + ?Sized,
{
  // TODO
  let dir = new_directory(random)?;
  Ok(Arc::new(dir))
}

pub(crate) fn new_mock_directory<R>(
  random: &mut R,
) -> Result<MockDirectoryWrapper<ByteBuffersDirectory<SingleInstanceLockFactory>>>
where
  R: Rng + ?Sized,
{
  Ok(MockDirectoryWrapper::new(
    random,
    ByteBuffersDirectory::new(),
  ))
}

pub(crate) fn new_mock_directory_with_lock_factory<R, LF>(
  random: &mut R,
  lock_factory: LF,
) -> Result<MockDirectoryWrapper<ByteBuffersDirectory<LF>>>
where
  R: Rng + ?Sized,
  LF: LockFactory + Send + Sync + 'static,
{
  Ok(MockDirectoryWrapper::new(
    random,
    ByteBuffersDirectory::with_lock_factory(lock_factory),
  ))
}

pub(crate) fn new_mock_fs_directory<R>(
  random: &mut R,
  temp_dir: TempDir,
) -> Result<MockDirectoryWrapper<DirEnum>>
where
  R: Rng + ?Sized,
{
  let dir = NIOFSDirectory::new(temp_dir.keep())?;
  Ok(MockDirectoryWrapper::new(random, dir))
}

// TODO: When we have implemented multiple directories, we need to select one
// randomly. Currently, we choose NIOFSDirectory.
pub(crate) fn new_directory_shared<R>(random: &mut R) -> Result<Arc<DirEnum>>
where
  R: Rng + ?Sized,
{
  let dir = new_directory(random)?;
  Ok(Arc::new(dir))
}
pub(crate) fn new_directory<R>(_random: &mut R) -> Result<DirEnum>
where
  R: Rng + ?Sized,
{
  let temp_dir = TempDir::new()?;
  NIOFSDirectory::new(temp_dir.keep())
}
pub(crate) fn new_directory_with_lock_factory<R, T>(
  _random: &mut R,
  lock_factory: T,
) -> Result<FSDirectory<LockFactoryEnum, NIOFSDirectory>>
where
  R: Rng + ?Sized,
  T: Into<LockFactoryEnum>,
{
  let temp_dir = TempDir::new()?;
  NIOFSDirectory::with_lock_factory(temp_dir.keep(), lock_factory.into())
}

pub(crate) fn new_fs_directory<R>(_random: &mut R, temp_dir: TempDir) -> Result<Arc<DirEnum>>
where
  R: Rng + ?Sized,
{
  Ok(Arc::new(NIOFSDirectory::new(temp_dir.keep())?))
}

pub(crate) fn new_string_field<S1, S2, R>(
  random: &mut R,
  name: S1,
  value: S2,
  stored: Store,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S1: Into<String>,
  S2: Into<String>,
{
  let field_type = match stored {
    Store::Yes => crate::core::document::string_field::TYPE_STORED.clone(),
    Store::No => crate::core::document::string_field::TYPE_NOT_STORED.clone(),
  };

  new_field_with_random(
    random,
    name.into(),
    FieldDataEnum::String(value.into()),
    &field_type,
    field_to_type,
  )
}

pub(crate) fn new_string_field_binary<S, R>(
  random: &mut R,
  name: S,
  value: BytesRef<Vec<u8>>,
  stored: Store,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S: Into<String>,
{
  let field_type = match stored {
    Store::Yes => crate::core::document::string_field::TYPE_STORED.clone(),
    Store::No => crate::core::document::string_field::TYPE_NOT_STORED.clone(),
  };

  new_field_with_random(
    random,
    name.into(),
    value.into(),
    &field_type,
    field_to_type,
  )
}
pub(crate) fn new_text_field<S1, S2, R>(
  random: &mut R,
  name: S1,
  value: S2,
  stored: Store,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S1: Into<String>,
  S2: Into<String>,
{
  let field_type = match stored {
    Store::Yes => crate::core::document::text_field::TYPE_STORED.clone(),
    Store::No => crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  };

  new_field_with_random(
    random,
    name,
    FieldDataEnum::String(value.into()),
    &field_type,
    field_to_type,
  )
}
pub(crate) fn new_string_field_string_with_random<S1, S2, R>(
  random: &mut R,
  name: S1,
  value: S2,
  stored: Store,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S1: Into<String>,
  S2: Into<String>,
{
  let field_type = match stored {
    Store::Yes => crate::core::document::string_field::TYPE_STORED.clone(),
    Store::No => crate::core::document::string_field::TYPE_NOT_STORED.clone(),
  };

  new_field_with_random(
    random,
    name,
    FieldDataEnum::String(value.into()),
    &field_type,
    field_to_type,
  )
}
pub(crate) fn new_string_field_binary_with_random<S, R>(
  random: &mut R,
  name: S,
  value: BytesRef<Vec<u8>>,
  stored: Store,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S: Into<String>,
{
  let field_type = match stored {
    Store::Yes => crate::core::document::string_field::TYPE_STORED.clone(),
    Store::No => crate::core::document::string_field::TYPE_NOT_STORED.clone(),
  };
  new_field_with_random(
    random,
    name,
    FieldDataEnum::Binary(value),
    &field_type,
    field_to_type,
  )
}

pub(crate) fn new_field<S, V, R>(
  random: &mut R,
  name: S,
  value: V,
  field_type: &FieldType,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S: Into<String>,
  V: Into<FieldDataEnum>,
{
  new_field_with_random(random, name, value.into(), field_type, field_to_type)
}
// TODO: if we can pull out the "make term vector options
// consistent across all instances of the same field name"
// write-once schema helper type, then we can
// remove the sync here.  We can also fold the random
// "enable norms" (now commented out, below) into that:
pub(crate) fn new_field_with_random<S, R>(
  random: &mut R,
  name: S,
  value: FieldDataEnum,
  field_type: &FieldType,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<Field>
where
  R: Rng + ?Sized,
  S: Into<String>,
{
  let name = name.into();

  let map = field_to_type;
  if let Some(prev_type) = map.get(&name) {
    return create_field(&name, value, prev_type.clone());
  }
  // TODO: once all core & test codecs can index
  // offsets, sometimes randomly turn on offsets if we are
  // already indexing positions...
  let mut new_type = FieldType::from_ref(field_type)?;
  if !new_type.stored() && random.random_bool(0.5) {
    new_type.set_stored(true)?; // randomly store it
  }

  if *new_type.index_options() != IndexOptions::None
    && !new_type.store_term_vectors()
    && random.random_bool(0.5)
  {
    new_type.set_store_term_vectors(true)?;

    if !new_type.store_term_vector_positions() && random.random_bool(0.5) {
      new_type.set_store_term_vector_positions(true)?;

      if !new_type.store_term_vector_payloads() {
        new_type.set_store_term_vector_payloads(random.random_bool(0.5))?;
      }
    }

    // Check for strings as offsets are disallowed on binary fields
    if matches!(value, FieldDataEnum::String(_)) && !new_type.store_term_vector_offsets() {
      new_type.set_store_term_vector_offsets(random.random_bool(0.5))?;
    }

    if cfg!(feature = "test_log_verbose") {
      println!(
        "NOTE: LuceneTestCase: upgrade name={} type={:?}",
        name, new_type
      );
    }
  }
  new_type.freeze();
  map.insert(name.clone(), new_type.clone());
  create_field(&name, value, new_type)
}
pub(crate) fn create_field(
  name: &str,
  value: FieldDataEnum,
  field_type: FieldType,
) -> Result<Field> {
  match value {
    FieldDataEnum::String(_) => Ok(Field::new(name, value, field_type)),
    FieldDataEnum::Binary(_) => Ok(Field::new(name, value, field_type)),
    _ => Err(LuceneError::illegal_argument(
      "Unsupported FieldDataEnum variant",
    )),
  }
}

pub(crate) fn new_io_context<R>(random: &mut R) -> Result<IOContext>
where
  R: Rng + ?Sized,
{
  new_io_context_with_default(random, &IO_CONTEXT_DEFAULT)
}

pub(crate) fn new_io_context_with_default<R>(
  random: &mut R,
  old_context: &IOContext,
) -> Result<IOContext>
where
  R: Rng + ?Sized,
{
  if *old_context == *IO_CONTEXT_READ_ONCE {
    // Don't modify the READONCE SINGLETON
    return Ok(old_context.clone());
  }

  // Generate random parameters
  let random_num_docs: i32 = random.random_range(0..4192);
  let size = random.random_range(0..512) * random_num_docs as i64;

  if let Some(flush_info) = &old_context.flush_info {
    // Always return at least the estimatedSegmentSize of the incoming
    // IOContext
    Ok(IOContext::with_flush(FlushInfo::new(
      random_num_docs,
      size.max(flush_info.get_estimated_segment_size()),
    ))?)
  } else if let Some(merge_info) = &old_context.merge_info {
    // Always return at least the estimatedMergeBytes of the incoming
    // IOContext
    IOContext::with_merge(MergeInfo::new(
      random_num_docs,
      size.max(merge_info.get_estimated_merge_bytes()),
      random.random_bool(0.5), /* Randomly decide if it's an external
                                * merge  */
      random.random_range(1..=100),
    ))
  } else {
    // Make a totally random IOContext, except READONCE which has semantic
    // implications
    let context_type = random.random_range(0..3);
    match context_type {
      0 => Ok(IOContext::default_io_context()?),
      1 => Ok(IOContext::with_merge(MergeInfo::new(
        random_num_docs,
        size,
        true,
        -1,
      ))?),
      2 => Ok(IOContext::with_flush(FlushInfo::new(
        random_num_docs,
        size,
      ))?),
      _ => Ok(IOContext::default_io_context()?),
    }
  }
}
/// What level of concurrency is supported by the searcher being created
pub enum Concurrency {
  /// No concurrency, meaning an executor won't be provided to the searcher
  None,
  /// Inter-segment concurrency, meaning an executor will be provided to the searcher and slices will be randomly created to concurrently search entire segments
  InterSegment,
  /// Intra-segment concurrency, meaning an executor will be provided to the searcher and slices will be randomly created to concurrently search segment partitions
  IntraSegment,
}
pub fn new_searcher<CR>(
  composite_reader: CR,
  _may_be_wrap: bool,
  _wrap_with_assertions: bool,
) -> Result<DefaultIndexSearcher<CompositeReaderContext<CR>>>
where
  CR: CompositeReader,
{
  let irc = get_context(composite_reader)?;
  IndexSearcher::new(irc)
}
pub fn new_searcher_with_lr<LR>(
  leaf_reader: LR,
) -> Result<DefaultIndexSearcher<LeafReaderContext<LR>>>
where
  LR: LeafReader,
{
  new_searcher_with_lr_wrap(leaf_reader, false)
}
pub fn new_searcher_with_lr_wrap<LR>(
  leaf_reader: LR,
  _may_be_wrap: bool,
) -> Result<DefaultIndexSearcher<LeafReaderContext<LR>>>
where
  LR: LeafReader,
{
  // TODO 多线程未实现
  let irc = crate::core::index::leaf_reader::get_context(leaf_reader)?;
  IndexSearcher::new(irc)
}
pub fn new_searcher_with_wrap<CR, R>(
  random: &mut R,
  composite_reader: CR,
  may_be_wrap: bool,
) -> Result<DefaultIndexSearcher<CompositeReaderContext<CR>>>
where
  CR: CompositeReader,
  R: Rng + ?Sized,
{
  new_searcher_with_wrap_assert(random, composite_reader, may_be_wrap, true)
}
pub fn new_searcher_with_wrap_assert<CR, R>(
  random: &mut R,
  composite_reader: CR,
  may_be_wrap: bool,
  wrap_with_assertions: bool,
) -> Result<DefaultIndexSearcher<CompositeReaderContext<CR>>>
where
  CR: CompositeReader,
  R: Rng + ?Sized,
{
  let threads = random.random_bool(0.5);
  new_searcher_with_threads(
    random,
    composite_reader,
    may_be_wrap,
    wrap_with_assertions,
    threads,
  )
}
pub fn new_searcher_with_threads<R, CR>(
  random: &mut R,
  composite_reader: CR,
  _may_be_wrap: bool,
  _wrap_with_assertions: bool,
  use_threads: bool,
) -> Result<DefaultIndexSearcher<CompositeReaderContext<CR>>>
where
  CR: CompositeReader,
  R: Rng + ?Sized,
{
  let irc = get_context(composite_reader)?;
  if use_threads {
    let threads = random.random_range(2..=5);
    IndexSearcher::with_threads(irc, threads)
  } else {
    IndexSearcher::new(irc)
  }
}

pub fn new_searcher_with_leaf_reader<LR>(
  lr_reader: LR,
) -> Result<DefaultIndexSearcher<LeafReaderContext<LR>>>
where
  LR: LeafReader,
{
  let irc = crate::core::index::leaf_reader::get_context(lr_reader)?;
  IndexSearcher::new(irc)
}
pub fn new_searcher_with_reader<CR>(
  composite_reader: CR,
) -> Result<DefaultIndexSearcher<CompositeReaderContext<CR>>>
where
  CR: CompositeReader,
{
  let irc = get_context(composite_reader)?;
  IndexSearcher::new(irc)
}

pub(crate) fn slow_file_exists(dir: &impl Directory, name: &str) -> Result<bool> {
  match dir.open_input(name, &IOContext::read_once_io_context()?) {
    Ok(mut input) => {
      input.close()?;
      Ok(true)
    },
    Err(LuceneError::IoWithPath { source, .. }) if source.kind() == ErrorKind::NotFound => {
      Ok(false)
    },
    Err(LuceneError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => Ok(false),
    Err(LuceneError::NoSuchFile(_)) => Ok(false),
    Err(error) => Err(error),
  }
}
/// Ensures that the MergePolicy has sane values for tests that test with lots of documents.
pub(crate) fn ensure_sane_iwc_on_nightly<D>(conf: &mut IndexWriterConfig<D>) -> Result<()>
where
  D: Directory,
{
  if is_night_mode() {
    conf.set_use_compound_file(true);
    let mp = conf.get_merge_policy_mut();

    match mp {
      MergePolicyEnum::Tiered(mp) => {
        mp.set_max_merged_segment_mb(5000.0)?;
      },
      MergePolicyEnum::LogBytesSize(mp) => {
        mp.set_max_merge_mb(1000.0);
      },
      MergePolicyEnum::LogDoc(mp) => {
        mp.set_max_merge_docs(100000);
      },
      _ => {},
    }

    let no_cfs_ratio = mp.get_base().get_no_cfs_ratio();
    mp.get_base_mut().set_no_cfs_ratio(no_cfs_ratio.max(0.25))?;
  }
  Ok(())
}

/// Creates a `BytesRef` holding UTF-8 bytes for the incoming string,
/// that sometimes uses a non-zero offset and non-zero end-padding to
/// tickle latent bugs that fail to look at `BytesRef.offset`.
pub(crate) fn new_bytes_ref_from_string<R, AV>(random: &mut R, s: &str) -> Result<BytesRef<AV>>
where
  R: Rng + ?Sized,
  AV: SharedAccessVec<u8>,
{
  let bytes = s.as_bytes();
  new_bytes_ref(random, bytes, 0, bytes.len() as i32)
}

/// Creates a copy of the incoming `BytesRef` that sometimes uses a non-zero
/// offset, and non-zero end-padding, to tickle latent bugs that fail to look at
/// `BytesRef.offset`.
pub(crate) fn new_bytes_ref_from_bytes_ref<R, AV>(
  random: &mut R,
  b: &BytesRef<AV>,
) -> Result<BytesRef<AV>>
where
  R: Rng + ?Sized,
  AV: SharedAccessVec<u8>,
{
  assert!(b.is_valid()?);
  b.bytes
    .access(|bytes| new_bytes_ref(random, bytes, b.offset as i32, b.length as i32))
}

/// Creates a random `BytesRef` from the incoming bytes, sometimes using a
/// non-zero offset, and non-zero end-padding, to tickle latent bugs that fail
/// to look at `BytesRef.offset`.
pub(crate) fn new_bytes_ref_from_bytes<R, AV>(
  random: &mut R,
  bytes_in: &[u8],
) -> Result<BytesRef<AV>>
where
  R: Rng + ?Sized,
  AV: SharedAccessVec<u8>,
{
  new_bytes_ref(random, bytes_in, 0, bytes_in.len() as i32)
}

/// Creates a random empty `BytesRef` that sometimes uses a non-zero offset, and
/// non-zero end-padding, to tickle latent bugs that fail to look at
/// `BytesRef.offset`.
pub(crate) fn new_bytes_ref_empty<R, AV>(random: &mut R) -> Result<BytesRef<AV>>
where
  R: Rng + ?Sized,
  AV: SharedAccessVec<u8>,
{
  // Calling the existing `new_bytes_ref` function
  new_bytes_ref(random, &[], 0, 0)
}

/// Creates a random empty `BytesRef`, with at least the requested length of
/// bytes free, that sometimes uses a non-zero offset and non-zero end-padding
/// to tickle latent bugs that fail to look at `BytesRef.offset`.
pub(crate) fn new_bytes_ref_with_length<R, AV>(
  byte_length: i32,
  random: &mut R,
) -> Result<BytesRef<AV>>
where
  R: Rng + ?Sized,
  AV: SharedAccessVec<u8>,
{
  let bytes_in = vec![0u8; byte_length as usize];
  new_bytes_ref(random, &bytes_in, 0, byte_length)
}

/// Creates a copy of the incoming bytes slice that sometimes uses a non-zero
/// `offset`, and non-zero end-padding, to expose latent bugs that fail to
/// account for `BytesRef::offset`.
pub(crate) fn new_bytes_ref<R, AV>(
  random: &mut R,
  bytes_in: &[u8],
  offset: i32,
  length: i32,
) -> Result<BytesRef<AV>>
where
  R: Rng + ?Sized,
  AV: SharedAccessVec<u8>,
{
  assert!(
    bytes_in.len() >= (offset + length) as usize,
    "got offset={} length={} bytesIn.length={}",
    offset,
    length,
    bytes_in.len()
  );
  // Randomly set a non-zero offset
  let start_offset = if random.random_bool(0.5) {
    random.random_range(1..=20)
  } else {
    0
  };

  // Randomly set an end padding (between 1 and 20)
  let end_padding = if random.random_bool(0.5) {
    random.random_range(1..=20)
  } else {
    0
  };

  let mut bytes = vec![0u8; (start_offset + length + end_padding) as usize];

  bytes.copy_from(
    &bytes_in[offset as usize..(offset + length) as usize],
    start_offset as usize,
  );
  // Create a BytesRef and return it
  let vec = AV::from_vec(bytes);
  let it = BytesRef {
    bytes: vec,
    offset: start_offset as usize,
    length: length as usize,
  };
  assert!(it.is_valid()?);

  if random.random_range(1..=17) == 7 {
    return it
      .bytes
      .access(|bytes| new_bytes_ref(random, bytes, it.offset as i32, it.length as i32));
  }
  Ok(it)
}

/// Retrieves the seed from the environment variable "tests.seed".
/// If the environment variable is not set or cannot be parsed as a `u64`,
/// it generates a random seed and logs the result.
///
/// # Returns
/// A valid `u64` seed.
pub(crate) fn get_seed_from_env() -> u64 {
  static GLOBAL_SEED: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

  fn current_seed() -> u64 {
    if let Some(seed) = GLOBAL_SEED.get() {
      *seed
    } else if let Ok(seed_str) = std::env::var(TestSeed.to_string()) {
      if let Ok(seed) = seed_str.parse::<u64>() {
        println!("Using Global Seed from environment: '{}'", seed);
        seed
      } else {
        println!("Environment variable tests.seed is invalid: '{}'", seed_str);
        let seed = rand::rng().random_range(0..u64::MAX);
        println!("Generated random seed: {}", seed);
        seed
      }
    } else {
      let seed = rand::rng().random_range(0..u64::MAX);
      println!("Generated random seed: {}", seed);
      seed
    }
  }
  current_seed()
}

pub(crate) fn random() -> StdRng {
  StdRng::seed_from_u64(get_seed_from_env())
}

pub(crate) fn random_from_seed(seed: u64) -> StdRng {
  StdRng::seed_from_u64(seed)
}

/// Inspects the stack trace to figure out if a method of a specific type
/// called us.
#[inline(never)]
pub(crate) fn call_stack_contains<T>(method_name: &str) -> bool {
  let type_name = std::any::type_name::<T>();
  let type_name = type_name.split('<').next().unwrap_or(type_name);
  let method_name = format!("::{method_name}");
  Backtrace::force_capture().to_string().lines().any(|frame| {
    frame.contains(type_name)
      && frame.match_indices(&method_name).any(|(index, _)| {
        let suffix = &frame[index + method_name.len()..];
        suffix.is_empty() || suffix.starts_with("::<") || suffix.starts_with("::{closure")
      })
  })
}

/// Inspects the stack trace to figure out if one of the given method names (no
/// type restriction) called us.
#[inline(never)]
pub(crate) fn call_stack_contains_any_of(method_names: &[&str]) -> bool {
  let backtrace = Backtrace::force_capture().to_string();
  method_names.iter().any(|method_name| {
    let method_name = format!("::{method_name}");
    backtrace.lines().any(|frame| {
      frame.match_indices(&method_name).any(|(index, _)| {
        let suffix = &frame[index + method_name.len()..];
        suffix.is_empty() || suffix.starts_with("::<") || suffix.starts_with("::{closure")
      })
    })
  })
}

/// Inspects the stack trace to figure out if a method of a specific type
/// called us.
#[inline(never)]
pub(crate) fn call_stack_contains_type<T>() -> bool {
  let type_name = std::any::type_name::<T>();
  let type_name = type_name.split('<').next().unwrap_or(type_name);
  Backtrace::force_capture()
    .to_string()
    .lines()
    .any(|frame| frame.contains(type_name))
}

pub fn is_night_mode() -> bool {
  std::env::var(NightMode.to_string()).is_ok_and(|v| v == "true")
}
pub fn create_temp_dir() -> Result<TempDir> {
  let temp_dir = TempDir::new()?;
  Ok(temp_dir)
}
pub fn create_temp_dir_with_prefix<T>(prefix: T) -> Result<TempDir>
where
  T: Into<String>,
{
  let name = prefix.into();
  let temp_dir = TempDir::with_prefix(name)?;
  Ok(temp_dir)
}
