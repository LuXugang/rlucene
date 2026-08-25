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
use crate::core::document::document::Document;
use crate::core::document::field::FieldDataEnum;
use crate::core::document::field_type::FieldType;
use crate::core::index::base_composite_reader::{
  BCRStoredFieldsImpl, BCRTermVectorsImpl, BaseCompositeReader, BaseCompositeReaderBase,
};
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader;
use crate::core::index::directory_reader::{DirectoryReader, DirectoryReaderBase};
use crate::core::index::index_commit::IndexCommit;
use crate::core::index::index_reader::{
  CacheHelperEnum2, CompositeReaderContextKind, IndexReader, IndexReaderBase,
};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::soft_deletes_directory_reader_wrapper::{
  SoftDeletesCodecReader, SoftDeletesDirectoryReaderWrapper,
};
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::term_query::TermQuery;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::dummy::dummy_comparator::DummyComparator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_night_mode, new_index_writer_config_with_analyzer,
  new_maybe_virus_checking_directory, new_searcher_with_reader, random, random_from_seed,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::RngExt;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

const SOFT_DELETES_FIELD: &str = "___soft_deletes";
type StandardDirReader = StandardDirectoryReader<DirEnum>;
type SoftDeletesDirReader = SoftDeletesDirectoryReaderWrapper<StandardDirReader>;
type StressLeafReader = SoftDeletesCodecReader<<StandardDirReader as CompositeReader>::LeafReader>;
type DirReader = Arc<StressDirReader>;

enum StressDirReader {
  Standard {
    reader: StandardDirReader,
    base: BaseCompositeReaderBase<StressLeafReader>,
    index_base: IndexReaderBase,
  },
  SoftDeletes {
    reader: SoftDeletesDirReader,
    base: BaseCompositeReaderBase<StressLeafReader>,
    index_base: IndexReaderBase,
  },
}

impl StressDirReader {
  fn from_standard(reader: StandardDirReader) -> Result<Self> {
    let sub_readers = reader
      .get_sequential_sub_readers()
      .iter()
      .cloned()
      .map(SoftDeletesCodecReader::A)
      .collect();
    let index_base = IndexReaderBase::new();
    let base = BaseCompositeReaderBase::new::<DummyComparator>(sub_readers, None, &index_base)?;
    Ok(Self::Standard {
      reader,
      base,
      index_base,
    })
  }

  fn from_soft_deletes(reader: SoftDeletesDirReader) -> Result<Self> {
    let index_base = IndexReaderBase::new();
    let base = BaseCompositeReaderBase::new::<DummyComparator>(
      reader.get_sequential_sub_readers().to_vec(),
      None,
      &index_base,
    )?;
    Ok(Self::SoftDeletes {
      reader,
      base,
      index_base,
    })
  }
}

impl BaseCompositeReader for StressDirReader {}

impl CompositeReader for StressDirReader {
  type LeafReader = StressLeafReader;
  type SubReader = StressLeafReader;

  fn get_sequential_sub_readers(&self) -> &[Self::SubReader] {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => {
        base.get_sequential_sub_readers()
      },
    }
  }

  fn visit_leaves<F>(&self, visitor: &mut F) -> Result<()>
  where
    F: FnMut(&Self::LeafReader) -> Result<()>,
  {
    for reader in self.get_sequential_sub_readers() {
      visitor(reader)?;
    }
    Ok(())
  }

  fn to_string(&self) -> String {
    match self {
      Self::Standard { reader, .. } => CompositeReader::to_string(reader),
      Self::SoftDeletes { reader, .. } => CompositeReader::to_string(reader),
    }
  }
}

impl IndexReader for StressDirReader {
  type ContextKind = CompositeReaderContextKind;
  type TermVectors = BCRTermVectorsImpl<StressLeafReader>;

  fn term_vectors(&self) -> Result<Self::TermVectors> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => base.term_vector(self),
    }
  }

  fn max_doc(&self) -> Result<i32> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => Ok(base.max_doc()),
    }
  }

  fn num_docs(&self) -> Result<i32> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => base.num_docs(),
    }
  }

  type StoredFields = BCRStoredFieldsImpl<StressLeafReader>;

  fn stored_fields(&self) -> Result<Self::StoredFields> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => base.stored_fields(self),
    }
  }

  fn do_close(&self) -> Result<()> {
    match self {
      Self::Standard { reader, .. } => reader.close(),
      Self::SoftDeletes { reader, .. } => reader.close(),
    }
  }

  type ReaderCacheHelper = CacheHelperEnum2<
    <StandardDirReader as IndexReader>::ReaderCacheHelper,
    <SoftDeletesDirReader as IndexReader>::ReaderCacheHelper,
  >;

  fn get_reader_cache_helper(&self) -> Result<Option<Self::ReaderCacheHelper>> {
    match self {
      Self::Standard { reader, .. } => {
        Ok(reader.get_reader_cache_helper()?.map(CacheHelperEnum2::A))
      },
      Self::SoftDeletes { reader, .. } => {
        Ok(reader.get_reader_cache_helper()?.map(CacheHelperEnum2::B))
      },
    }
  }

  fn doc_freq(&self, term: &Term) -> Result<i32> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => base.doc_freq(term, self),
    }
  }

  fn total_term_freq(&self, term: &Term) -> Result<i64> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => {
        base.total_term_freq(term, self)
      },
    }
  }

  fn get_sum_doc_freq(&self, field: &str) -> Result<i64> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => {
        base.get_sum_doc_freq(field, self)
      },
    }
  }

  fn get_doc_count(&self, field: &str) -> Result<i32> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => {
        base.get_doc_count(field, self)
      },
    }
  }

  fn get_sum_total_term_freq(&self, field: &str) -> Result<i64> {
    match self {
      Self::Standard { base, .. } | Self::SoftDeletes { base, .. } => {
        base.get_sum_total_term_freq(field, self)
      },
    }
  }

  fn index_base(&self) -> &IndexReaderBase {
    match self {
      Self::Standard { index_base, .. } | Self::SoftDeletes { index_base, .. } => index_base,
    }
  }
}

impl DirectoryReader for StressDirReader {
  type DirectoryReader = Self;
  type Directory = DirEnum;

  fn directory(&self) -> &DirectoryReaderBase<Self::Directory> {
    match self {
      Self::Standard { reader, .. } => reader.directory(),
      Self::SoftDeletes { reader, .. } => reader.directory(),
    }
  }

  fn do_open_if_changed(&self) -> Result<Option<Self::DirectoryReader>> {
    match self {
      Self::Standard { reader, .. } => reader
        .do_open_if_changed()?
        .map(Self::from_standard)
        .transpose(),
      Self::SoftDeletes { reader, .. } => reader
        .do_open_if_changed()?
        .map(Self::from_soft_deletes)
        .transpose(),
    }
  }

  fn do_open_if_changed_with_commit<IC>(
    &self,
    commit: Option<&IC>,
  ) -> Result<Option<Self::DirectoryReader>>
  where
    IC: IndexCommit<Directory = Arc<Self::Directory>>,
  {
    match self {
      Self::Standard { reader, .. } => reader
        .do_open_if_changed_with_commit(commit)?
        .map(Self::from_standard)
        .transpose(),
      Self::SoftDeletes { reader, .. } => reader
        .do_open_if_changed_with_commit(commit)?
        .map(Self::from_soft_deletes)
        .transpose(),
    }
  }

  fn do_open_if_changed_with_deletes(
    &self,
    writer: &Arc<IndexWriter<Self::Directory>>,
    apply_deletes: bool,
  ) -> Result<Option<Self::DirectoryReader>> {
    match self {
      Self::Standard { reader, .. } => reader
        .do_open_if_changed_with_deletes(writer, apply_deletes)?
        .map(Self::from_standard)
        .transpose(),
      Self::SoftDeletes { reader, .. } => reader
        .do_open_if_changed_with_deletes(writer, apply_deletes)?
        .map(Self::from_soft_deletes)
        .transpose(),
    }
  }

  fn get_version(&self) -> Result<i64> {
    match self {
      Self::Standard { reader, .. } => reader.get_version(),
      Self::SoftDeletes { reader, .. } => reader.get_version(),
    }
  }

  fn is_current(&self) -> Result<bool> {
    match self {
      Self::Standard { reader, .. } => reader.is_current(),
      Self::SoftDeletes { reader, .. } => reader.is_current(),
    }
  }

  type IndexCommit = <StandardDirReader as DirectoryReader>::IndexCommit;

  fn get_index_commit(&self) -> Result<Self::IndexCommit> {
    match self {
      Self::Standard { reader, .. } => reader.get_index_commit(),
      Self::SoftDeletes { reader, .. } => reader.get_index_commit(),
    }
  }
}

impl Display for StressDirReader {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Standard { reader, .. } => write!(f, "{reader}"),
      Self::SoftDeletes { reader, .. } => write!(f, "{reader}"),
    }
  }
}

/// Corresponds to Java fields protected by `synchronized (TestStressNRT.this)`:
/// `reader`, `committedModel`, `snapshotCount`, `committedModelClock`.
struct SyncedState {
  reader: Option<DirReader>,
  committed_model: HashMap<i32, i64>,
  snapshot_count: i64,
  committed_model_clock: i64,
}

struct TestStressNRT {
  synced: Mutex<SyncedState>,
  model: Mutex<HashMap<i32, i64>>,
  last_id: AtomicI32,
  field: &'static str,
  sync_arr: OnceLock<Vec<Arc<Mutex<()>>>>,
}

impl TestStressNRT {
  fn init_model(&self, ndocs: i32) {
    let mut synced = self.synced.lock();
    synced.snapshot_count = 0;
    synced.committed_model_clock = 0;
    synced.committed_model.clear();
    drop(synced);

    self.last_id.store(0, Ordering::SeqCst);

    let mut sync_vec = Vec::with_capacity(ndocs as usize);
    for _ in 0..ndocs {
      sync_vec.push(Arc::new(Mutex::new(())));
    }
    let _ = self.sync_arr.set(sync_vec);

    let mut model = self.model.lock();
    for i in 0..ndocs {
      model.insert(i, -1i64);
    }

    let mut synced = self.synced.lock();
    synced
      .committed_model
      .extend(model.iter().map(|(&k, &v)| (k, v)));
  }

  fn test(self: &Arc<Self>) -> Result<()> {
    let mut rand = random();

    // update variables
    let commit_percent = rand.random_range(0..20);
    let soft_commit_percent = rand.random_range(0..100); // what percent of the commits are soft
    let delete_percent = rand.random_range(0..50);
    let delete_by_query_percent = rand.random_range(0..25);
    let ndocs = at_least(&mut rand, 50);
    let n_write_threads = TestUtil::next_int(&mut rand, 1, if is_night_mode() { 10 } else { 5 });
    let max_concurrent_commits =
      TestUtil::next_int(&mut rand, 1, if is_night_mode() { 10 } else { 5 }); // number of committers at a time... needed if we want to avoid commit errors
    // due to exceeding the max
    let use_soft_deletes = rand.random_range(0..10) < 3;

    let tombstones = rand.random_bool(0.5);

    // query variables
    let operations = Arc::new(AtomicI64::new(at_least(&mut rand, 10000) as i64)); // number of query operations to perform in total

    let n_read_threads = TestUtil::next_int(&mut rand, 1, if is_night_mode() { 10 } else { 5 });
    self.init_model(ndocs);

    let stored_only_type = {
      let mut ft = FieldType::new();
      ft.set_stored(true)?;
      ft
    };

    if cfg!(feature = "test_log_verbose") {
      println!();
      println!("TEST: commitPercent={commit_percent}");
      println!("TEST: softCommitPercent={soft_commit_percent}");
      println!("TEST: deletePercent={delete_percent}");
      println!("TEST: deleteByQueryPercent={delete_by_query_percent}");
      println!("TEST: ndocs={ndocs}");
      println!("TEST: nWriteThreads={n_write_threads}");
      println!("TEST: nReadThreads={n_read_threads}");
      println!("TEST: maxConcurrentCommits={max_concurrent_commits}");
      println!("TEST: tombstones={tombstones}");
      println!("TEST: operations={}", operations.load(Ordering::SeqCst));
      println!();
    }

    let num_committing = Arc::new(AtomicI32::new(0));

    let dir = new_maybe_virus_checking_directory(&mut rand)?;

    let writer = {
      let analyzer = MockAnalyzer::new(&mut rand);
      let config = new_index_writer_config_with_analyzer(&mut rand, analyzer)?;
      let riw =
        RandomIndexWriter::with_soft_deletes(&mut rand, dir.clone(), config, use_soft_deletes);
      riw.set_do_random_force_merge_assert(false);
      riw
    };
    writer.commit(&mut rand)?;

    {
      let open_reader = if use_soft_deletes {
        StressDirReader::from_soft_deletes(SoftDeletesDirectoryReaderWrapper::new(
          directory_reader::open(dir.clone())?,
          SOFT_DELETES_FIELD,
        )?)?
      } else {
        StressDirReader::from_standard(directory_reader::open(dir.clone())?)?
      };
      let mut synced = self.synced.lock();
      synced.reader = Some(Arc::new(open_reader));
    }

    let thread_result = std::thread::scope(|scope| -> Result<()> {
      let mut handles = Vec::new();

      // Writer threads
      for i in 0..n_write_threads {
        let writer_ref = &writer;
        let seed = rand.random::<u64>();
        let stored_only_type = stored_only_type.clone();
        let name = format!("WRITER{i}");
        let slf = Arc::clone(self);
        let operations = Arc::clone(&operations);
        let num_committing = Arc::clone(&num_committing);

        handles.push(std::thread::Builder::new().name(name).spawn_scoped(
          scope,
          move || -> Result<()> {
            let mut thread_rand = random_from_seed(seed);

            while operations.load(Ordering::SeqCst) > 0 {
              let oper = thread_rand.random_range(0..100);

              if oper < commit_percent {
                if num_committing.fetch_add(1, Ordering::SeqCst) < max_concurrent_commits {
                  let (new_committed_model, version, old_reader) = {
                    let mut synced = slf.synced.lock();
                    let new_committed_model = {
                      let model = slf.model.lock();
                      model.clone()
                    }; // take a snapshot
                    let version = {
                      let v = synced.snapshot_count;
                      synced.snapshot_count += 1;
                      v
                    };
                    let old_reader = synced.reader.clone();
                    if let Some(ref r) = old_reader {
                      r.inc_ref()?;
                    }
                    (new_committed_model, version, old_reader)
                  };

                  let old_reader =
                    old_reader.ok_or_else(|| LuceneError::illegal_state("reader is None"))?;

                  let new_reader: DirReader =
                    if thread_rand.random_range(0..100) < soft_commit_percent {
                      if thread_rand.random_bool(0.5) {
                        if cfg!(feature = "test_log_verbose") {
                          println!(
                            "TEST: {}: call writer.getReader",
                            std::thread::current().name().unwrap_or("unknown")
                          );
                        }
                        let reader = writer_ref.get_reader(&mut thread_rand)?;
                        Arc::new(if use_soft_deletes {
                          StressDirReader::from_soft_deletes(
                            SoftDeletesDirectoryReaderWrapper::new(reader, SOFT_DELETES_FIELD)?,
                          )?
                        } else {
                          StressDirReader::from_standard(reader)?
                        })
                      } else {
                        if cfg!(feature = "test_log_verbose") {
                          let old_reader_id = format!("{}", old_reader);
                          println!(
                            "TEST: {}: reopen reader={old_reader_id} version={version}",
                            std::thread::current().name().unwrap_or("unknown")
                          );
                        }
                        match directory_reader::open_if_changed_with_writer(
                          &old_reader,
                          &writer_ref.w,
                        )? {
                          Some(new_r) => Arc::new(new_r),
                          None => {
                            old_reader.inc_ref()?;
                            old_reader.clone()
                          },
                        }
                      }
                    } else {
                      if cfg!(feature = "test_log_verbose") {
                        let old_reader_id = format!("{}", old_reader);
                        println!(
                          "TEST: {}: commit+reopen reader={old_reader_id} version={version}",
                          std::thread::current().name().unwrap_or("unknown")
                        );
                      }
                      writer_ref.commit(&mut thread_rand)?;
                      if cfg!(feature = "test_log_verbose") {
                        println!(
                          "TEST: {}: now reopen after commit",
                          std::thread::current().name().unwrap_or("unknown")
                        );
                      }
                      match directory_reader::open_if_changed(&old_reader)? {
                        Some(new_r) => Arc::new(new_r),
                        None => {
                          old_reader.inc_ref()?;
                          old_reader.clone()
                        },
                      }
                    };

                  old_reader.dec_ref()?;

                  {
                    let mut synced = slf.synced.lock();
                    assert!(new_reader.get_ref_count() > 0);
                    let current_reader_version = synced
                      .reader
                      .as_ref()
                      .map(|r| r.get_version())
                      .transpose()?
                      .unwrap_or(0);
                    assert!(
                      current_reader_version == 0
                        || synced
                          .reader
                          .as_ref()
                          .map(|r| r.get_ref_count())
                          .unwrap_or(0)
                          > 0
                    );
                    if new_reader.get_version()? > current_reader_version {
                      if cfg!(feature = "test_log_verbose") {
                        let new_reader_id = format!("{}", new_reader);
                        println!(
                          "TEST: {}: install new reader={new_reader_id}",
                          std::thread::current().name().unwrap_or("unknown")
                        );
                      }
                      if let Some(ref old_r) = synced.reader {
                        old_r.dec_ref()?;
                      }
                      // Silly: forces fieldInfos to be loaded so we don't hit IOE on later reader.toString
                      let _ = format!("{}", new_reader);
                      synced.reader = Some(new_reader);

                      // install this snapshot only if it's newer than the current one
                      if version >= synced.committed_model_clock {
                        if cfg!(feature = "test_log_verbose") {
                          println!(
                            "TEST: {}: install new model version={version}",
                            std::thread::current().name().unwrap_or("unknown")
                          );
                        }
                        synced.committed_model = new_committed_model;
                        synced.committed_model_clock = version;
                      } else if cfg!(feature = "test_log_verbose") {
                        println!(
                          "TEST: {}: skip install new model version={version}",
                          std::thread::current().name().unwrap_or("unknown")
                        );
                      }
                    } else {
                      // if the same reader, don't decRef.
                      if cfg!(feature = "test_log_verbose") {
                        let new_reader_id = format!("{}", new_reader);
                        println!(
                          "TEST: {}: skip install new reader={new_reader_id}",
                          std::thread::current().name().unwrap_or("unknown")
                        );
                      }
                      new_reader.dec_ref()?;
                    }
                  }
                }
                num_committing.fetch_sub(1, Ordering::SeqCst);
              } else {
                let id = thread_rand.random_range(0..ndocs);

                // set the lastId before we actually change it sometimes to try and
                // uncover more race conditions between writing and reading
                let before = thread_rand.random_bool(0.5);
                if before {
                  slf.last_id.store(id, Ordering::SeqCst);
                }

                // We can't concurrently update the same document and retain our invariants of
                // increasing values since we can't guarantee what order the updates will be executed.
                let _sync_guard = slf.sync_arr.get().unwrap()[id as usize].lock();
                let val = {
                  let model = slf.model.lock();
                  *model.get(&id).unwrap_or(&-1i64)
                };
                let next_val = val.abs() + 1;

                if oper < commit_percent + delete_percent {
                  // add tombstone first
                  if tombstones {
                    let mut d = Document::new();
                    d.add(crate::core::document::field::Field::new(
                      "id",
                      FieldDataEnum::String(format!("-{id}")),
                      crate::core::document::string_field::TYPE_STORED.clone(),
                    ));
                    d.add(crate::core::document::field::Field::new(
                      slf.field,
                      FieldDataEnum::String(format!("{next_val}")),
                      stored_only_type.clone(),
                    ));
                    writer_ref.update_document_with_term(
                      &mut thread_rand,
                      Term::from_text("id", format!("-{id}")),
                      d,
                    )?;
                  }

                  if cfg!(feature = "test_log_verbose") {
                    println!(
                      "TEST: {}: term delDocs id:{id} nextVal={next_val}",
                      std::thread::current().name().unwrap_or("unknown")
                    );
                  }
                  writer_ref.delete_documents_with_terms(
                    &mut thread_rand,
                    vec![Term::from_text("id", format!("{id}"))],
                  )?;
                  slf.model.lock().insert(id, -next_val);
                } else if oper < commit_percent + delete_percent + delete_by_query_percent {
                  // add tombstone first
                  if tombstones {
                    let mut d = Document::new();
                    d.add(crate::core::document::field::Field::new(
                      "id",
                      FieldDataEnum::String(format!("-{id}")),
                      crate::core::document::string_field::TYPE_STORED.clone(),
                    ));
                    d.add(crate::core::document::field::Field::new(
                      slf.field,
                      FieldDataEnum::String(format!("{next_val}")),
                      stored_only_type.clone(),
                    ));
                    writer_ref.update_document_with_term(
                      &mut thread_rand,
                      Term::from_text("id", format!("-{id}")),
                      d,
                    )?;
                  }

                  if cfg!(feature = "test_log_verbose") {
                    println!(
                      "TEST: {}: query delDocs id:{id} nextVal={next_val}",
                      std::thread::current().name().unwrap_or("unknown")
                    );
                  }
                  writer_ref.delete_documents_with_queries(
                    &mut thread_rand,
                    vec![Query::from(TermQuery::new(Term::from_text(
                      "id",
                      format!("{id}"),
                    )))],
                  )?;
                  slf.model.lock().insert(id, -next_val);
                } else {
                  let mut d = Document::new();
                  d.add(crate::core::document::field::Field::new(
                    "id",
                    FieldDataEnum::String(format!("{id}")),
                    crate::core::document::string_field::TYPE_STORED.clone(),
                  ));
                  d.add(crate::core::document::field::Field::new(
                    slf.field,
                    FieldDataEnum::String(format!("{next_val}")),
                    stored_only_type.clone(),
                  ));
                  if cfg!(feature = "test_log_verbose") {
                    println!(
                      "TEST: {}: u id:{id} val={next_val}",
                      std::thread::current().name().unwrap_or("unknown")
                    );
                  }
                  writer_ref.update_document_with_term(
                    &mut thread_rand,
                    Term::from_text("id", format!("{id}")),
                    d,
                  )?;
                  if tombstones {
                    // remove tombstone after new addition (this should be optional?)
                    writer_ref.delete_documents_with_terms(
                      &mut thread_rand,
                      vec![Term::from_text("id", format!("-{id}"))],
                    )?;
                  }
                  slf.model.lock().insert(id, next_val);
                }

                if !before {
                  slf.last_id.store(id, Ordering::SeqCst);
                }
              }
            }
            Ok(())
          },
        )?);
      }

      // Reader threads
      for i in 0..n_read_threads {
        let seed = rand.random::<u64>();
        let name = format!("READER{i}");
        let slf = Arc::clone(self);
        let operations = Arc::clone(&operations);

        handles.push(
          std::thread::Builder::new()
            .name(name)
            .spawn_scoped(scope, move || -> Result<()> {
              let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
                let mut thread_rand = random_from_seed(seed);
                let mut last_reader: Option<DirReader> = None;
                let mut last_searcher = None;

                while operations.fetch_sub(1, Ordering::SeqCst) > 0 {
                  // bias toward a recently changed doc
                  let id = if thread_rand.random_range(0..100) < 25 {
                    slf.last_id.load(Ordering::SeqCst)
                  } else {
                    thread_rand.random_range(0..ndocs)
                  };

                  // when indexing, we update the index, then the model
                  // so when querying, we should first check the model, and then the index

                  let (val, reader) = {
                    let synced = slf.synced.lock();
                    let val = *synced.committed_model.get(&id).unwrap_or(&-1i64);
                    let reader = synced
                      .reader
                      .as_ref()
                      .ok_or_else(|| LuceneError::illegal_state("reader is None"))?
                      .clone();
                    reader.inc_ref()?;
                    (val, reader)
                  };

                  if cfg!(feature = "test_log_verbose") {
                    println!(
                      "TEST: {}: s id={id} val={val} r={}",
                      std::thread::current().name().unwrap_or("unknown"),
                      reader.get_version()?
                    );
                  }

                  // Just re-use lastSearcher, else newSearcher may create too many thread pools (ExecutorService):
                  let same_reader = last_reader
                    .as_ref()
                    .is_some_and(|lr| Arc::ptr_eq(lr, &reader));

                  let searcher: &mut IndexSearcher<_> = if same_reader {
                    last_searcher
                      .as_mut()
                      .ok_or_else(|| LuceneError::illegal_state("last_searcher is None"))?
                  } else {
                    let new_searcher = new_searcher_with_reader(reader.clone())?;
                    last_reader = Some(reader.clone());
                    last_searcher = Some(new_searcher);
                    last_searcher.as_mut().unwrap()
                  };

                  let q = TermQuery::new(Term::from_text("id", format!("{id}")));
                  let mut results = searcher.search(q, 10)?;

                  if results.total_hits.value() == 0 && tombstones {
                    // if we couldn't find the doc, look for its tombstone
                    let qt = TermQuery::new(Term::from_text("id", format!("-{id}")));
                    results = searcher.search(qt, 1)?;
                    if results.total_hits.value() == 0 {
                      if val == -1i64 {
                        // expected... no doc was added yet
                        reader.dec_ref()?;
                        continue;
                      }
                      panic!(
                        "No documents or tombstones found for id {id}, expected at least {val} reader={reader}"
                      );
                    }
                  }

                  if results.total_hits.value() == 0 && !tombstones {
                    // nothing to do - we can't tell anything from a deleted doc without tombstones
                  } else {
                    // we should have found the document, or its tombstone
                    if results.total_hits.value() != 1 {
                      println!("FAIL: hits id:{id} val={val}");
                      for sd in &results.score_docs {
                        let doc = reader.stored_fields()?.document(sd.doc)?;
                        println!(
                          "  docID={} id:{} foundVal={}",
                          sd.doc,
                          doc.get("id")?.unwrap_or_default(),
                          doc.get(slf.field)?.unwrap_or_default()
                        );
                      }
                      panic!(
                        "id={id} reader={reader} totalHits={}",
                        results.total_hits.value()
                      );
                    }
                    let mut stored_fields = searcher.stored_fields()?;
                    let doc = stored_fields.document(results.score_docs[0].doc)?;
                    let found_val: i64 = doc
                      .get(slf.field)?
                      .unwrap_or_default()
                      .parse()
                      .unwrap_or(0);
                    if found_val < val.abs() {
                      panic!("foundVal={found_val} val={val} id={id} reader={reader}");
                    }
                  }

                  reader.dec_ref()?;
                }
                Ok(())
              }));

              match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => {
                  operations.store(-1, Ordering::SeqCst);
                  println!(
                    "{}: FAILED: unexpected error",
                    std::thread::current().name().unwrap_or("unknown")
                  );
                  println!("{error:?}");
                  Err(error)
                },
                Err(payload) => {
                  operations.store(-1, Ordering::SeqCst);
                  println!(
                    "{}: FAILED: unexpected error",
                    std::thread::current().name().unwrap_or("unknown")
                  );
                  std::panic::resume_unwind(payload);
                },
              }
            })?,
        );
      }

      for handle in handles {
        handle
          .join()
          .map_err(|_| LuceneError::illegal_state("thread panicked"))??;
      }

      Ok(())
    });

    writer.close(&mut rand)?;

    if cfg!(feature = "test_log_verbose") {
      let synced = self.synced.lock();
      if let Some(ref reader) = synced.reader {
        println!("TEST: close reader={reader}");
      }
    }

    {
      let mut synced = self.synced.lock();
      if let Some(reader) = synced.reader.take() {
        reader.close()?;
      }
    }

    dir.close()?;

    thread_result?;

    Ok(())
  }
}

#[test]
fn test() -> Result<()> {
  let nrt = Arc::new(TestStressNRT {
    synced: Mutex::new(SyncedState {
      reader: None,
      committed_model: HashMap::new(),
      snapshot_count: 0,
      committed_model_clock: 0,
    }),
    model: Mutex::new(HashMap::new()),
    last_id: AtomicI32::new(0),
    field: "val_l",
    sync_arr: OnceLock::new(),
  });
  nrt.test()
}
