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
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::{
  IndexReader, IndexReaderContextKind, IndexReaderContextType,
};
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::reference_manager::{
  ReferenceManager, ReferenceManagerBase, RefreshListenerArc,
};
use crate::core::search::searcher_factory::SearcherFactory;
use crate::core::store::directory::Directory;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

type ManagedSearcher<DR> = IndexSearcher<CompositeReaderContext<Arc<DR>>>;

struct SearcherManagerBase<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  searcher_factory: SearcherFactory<DR>,
  marker: PhantomData<fn() -> DR>,
}

/// Utility struct for safely sharing [`IndexSearcher`] instances across multiple threads, while
/// periodically reopening. This struct ensures each searcher is closed only once all threads have
/// finished using it.
///
/// Use [`acquire`](Self::acquire) to obtain the current searcher and [`release`](Self::release) to
/// release it:
///
/// ```text
/// let searcher = manager.acquire()?;
/// let result = search(&searcher);
/// manager.release(searcher)?;
/// result
/// ```
///
/// Periodically call [`maybe_refresh`](Self::maybe_refresh). Although it is possible to call this
/// immediately before every query, doing so penalizes the queries that need to refresh. It is
/// better to use a separate background thread that periodically calls `maybe_refresh`. Call
/// [`close`](Self::close) when finished.
///
/// @lucene.experimental
pub struct SearcherManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  reference_manager: ReferenceManager<ManagedSearcher<DR>, SearcherManagerBase<DR>>,
}

impl<D> SearcherManager<StandardDirectoryReader<D>>
where
  D: Directory + 'static,
{
  /// Creates and returns a new [`SearcherManager`] from the given [`IndexWriter`].
  ///
  /// Pass `None` for `searcher_factory` if the searcher does not need to be warmed before going
  /// live and no other custom behavior is required.
  ///
  /// # Errors
  ///
  /// Returns an error if there is a low-level I/O failure.
  pub fn from_writer(
    writer: &Arc<IndexWriter<D>>,
    searcher_factory: Option<SearcherFactory<StandardDirectoryReader<D>>>,
  ) -> Result<Self> {
    Self::with_writer_deletes(writer, true, false, searcher_factory)
  }

  /// Expert: creates and returns a new [`SearcherManager`] from the given [`IndexWriter`],
  /// controlling whether past deletions should be applied.
  ///
  /// If `apply_all_deletes` is `true`, all buffered deletes are applied and made visible in the
  /// [`IndexSearcher`] and [`DirectoryReader`]. If it is `false`, deletes may or may not be applied,
  /// but remain buffered in the [`IndexWriter`] so that they will be applied in the future.
  /// Applying deletes can be costly, so applications that can tolerate deleted documents being
  /// returned may gain performance by passing `false`.
  ///
  /// If `write_all_deletes` is `true`, new deletes are forcefully written to index files.
  ///
  /// Pass `None` for `searcher_factory` if the searcher does not need to be warmed before going
  /// live and no other custom behavior is required.
  ///
  /// # Errors
  ///
  /// Returns an error if there is a low-level I/O failure.
  pub fn with_writer_deletes(
    writer: &Arc<IndexWriter<D>>,
    apply_all_deletes: bool,
    write_all_deletes: bool,
    searcher_factory: Option<SearcherFactory<StandardDirectoryReader<D>>>,
  ) -> Result<Self> {
    let searcher_factory = searcher_factory.unwrap_or_default();
    let current = get_searcher(
      &searcher_factory,
      Arc::new(directory_reader::open_with_writer_deletes(
        writer,
        apply_all_deletes,
        write_all_deletes,
      )?),
      None,
    )?;
    Ok(Self {
      reference_manager: ReferenceManager::new(
        current,
        SearcherManagerBase {
          searcher_factory,
          marker: PhantomData,
        },
      ),
    })
  }

  /// Creates and returns a new [`SearcherManager`] from the given [`Directory`].
  ///
  /// Pass `None` for `searcher_factory` if the searcher does not need to be warmed before going
  /// live and no other custom behavior is required.
  ///
  /// # Errors
  ///
  /// Returns an error if there is a low-level I/O failure.
  pub fn from_directory(
    directory: Arc<D>,
    searcher_factory: Option<SearcherFactory<StandardDirectoryReader<D>>>,
  ) -> Result<Self> {
    let searcher_factory = searcher_factory.unwrap_or_default();
    let current = get_searcher(
      &searcher_factory,
      Arc::new(directory_reader::open(directory)?),
      None,
    )?;
    Ok(Self {
      reference_manager: ReferenceManager::new(
        current,
        SearcherManagerBase {
          searcher_factory,
          marker: PhantomData,
        },
      ),
    })
  }
}

impl<DR> SearcherManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  /// Creates and returns a new [`SearcherManager`] from an existing [`DirectoryReader`]. This
  /// steals the incoming reference.
  ///
  /// Pass `None` for `searcher_factory` if the searcher does not need to be warmed before going
  /// live and no other custom behavior is required.
  ///
  /// # Errors
  ///
  /// Returns an error if there is a low-level I/O failure.
  pub fn new(reader: DR, searcher_factory: Option<SearcherFactory<DR>>) -> Result<Self> {
    let searcher_factory = searcher_factory.unwrap_or_default();
    let current = get_searcher(&searcher_factory, Arc::new(reader), None)?;
    Ok(Self {
      reference_manager: ReferenceManager::new(
        current,
        SearcherManagerBase {
          searcher_factory,
          marker: PhantomData,
        },
      ),
    })
  }
}

impl<DR> ReferenceManagerBase<ManagedSearcher<DR>> for SearcherManagerBase<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  fn dec_ref(&self, reference: &ManagedSearcher<DR>) -> Result<()> {
    reference.get_index_reader().dec_ref()
  }

  fn refresh_if_needed(
    &self,
    reference_to_refresh: &ManagedSearcher<DR>,
  ) -> Result<Option<ManagedSearcher<DR>>> {
    let reader = reference_to_refresh.get_index_reader();
    let Some(new_reader) = directory_reader::open_if_changed(reader.as_ref())? else {
      return Ok(None);
    };
    get_searcher(&self.searcher_factory, Arc::new(new_reader), Some(reader)).map(Some)
  }

  fn try_inc_ref(&self, reference: &ManagedSearcher<DR>) -> Result<bool> {
    Ok(reference.get_index_reader().try_inc_ref())
  }

  fn get_ref_count(&self, reference: &ManagedSearcher<DR>) -> i32 {
    reference.get_index_reader().get_ref_count()
  }
}

impl<DR> SearcherManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  /// Returns `true` if no changes have occurred since this searcher's reader was opened, otherwise
  /// `false`.
  ///
  /// # Errors
  ///
  /// Returns an error if checking the reader or releasing the acquired searcher fails.
  pub fn is_searcher_current(&self) -> Result<bool> {
    let searcher = self.acquire()?;
    let current_result = catch_unwind(AssertUnwindSafe(|| {
      searcher.get_index_reader().is_current()
    }));
    self.release(searcher)?;
    match current_result {
      Ok(result) => result,
      Err(payload) => resume_unwind(payload),
    }
  }

  pub fn acquire(&self) -> Result<Arc<ManagedSearcher<DR>>> {
    self.reference_manager.acquire()
  }

  pub fn close(&self) -> Result<()> {
    self.reference_manager.close()
  }

  pub fn maybe_refresh(&self) -> Result<bool> {
    self.reference_manager.maybe_refresh()
  }

  pub fn maybe_refresh_blocking(&self) -> Result<()> {
    self.reference_manager.maybe_refresh_blocking()
  }

  pub fn release(&self, reference: Arc<ManagedSearcher<DR>>) -> Result<()> {
    self.reference_manager.release(reference)
  }

  pub fn add_listener(&self, listener: RefreshListenerArc) {
    self.reference_manager.add_listener(listener);
  }

  pub fn remove_listener(&self, listener: &RefreshListenerArc) {
    self.reference_manager.remove_listener(listener);
  }
}

/// Expert: creates a searcher from the provided [`IndexReader`] using the provided
/// [`SearcherFactory`]. This decrements the incoming reader's reference count if an error or panic
/// occurs.
pub fn get_searcher<IR>(
  searcher_factory: &SearcherFactory<IR>,
  reader: Arc<IR>,
  previous_reader: Option<&Arc<IR>>,
) -> Result<IndexSearcher<IndexReaderContextType<Arc<IR>>>>
where
  IR: IndexReader + 'static,
  IR::ContextKind: IndexReaderContextKind<Arc<IR>>,
  IndexReaderContextType<Arc<IR>>: Sync + 'static,
{
  let reader_to_check = reader.clone();
  let searcher_result = catch_unwind(AssertUnwindSafe(|| -> Result<_> {
    let searcher = searcher_factory.new_searcher(reader, previous_reader)?;
    if !Arc::ptr_eq(searcher.get_index_reader(), &reader_to_check) {
      return Err(LuceneError::illegal_state(format!(
        "SearcherFactory must wrap exactly the provided reader (got {} but expected {})",
        searcher.get_index_reader(),
        reader_to_check
      )));
    }
    Ok(searcher)
  }));
  let success = matches!(&searcher_result, Ok(Ok(_)));
  if !success {
    reader_to_check.dec_ref()?;
  }
  match searcher_result {
    Ok(result) => result,
    Err(payload) => resume_unwind(payload),
  }
}

impl<DR> Closeable for SearcherManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  fn close(&mut self) -> Result<()> {
    SearcherManager::close(self)
  }
}

impl<DR> CloseableRef for SearcherManager<DR>
where
  DR: DirectoryReader<DirectoryReader = DR> + 'static,
  DR::ContextKind: IndexReaderContextKind<Arc<DR>, Context = CompositeReaderContext<Arc<DR>>>,
  CompositeReaderContext<Arc<DR>>: Sync + 'static,
{
  fn close(&self) -> Result<()> {
    SearcherManager::close(self)
  }
}
