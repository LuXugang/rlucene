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
use crate::core::index::approximate_priority_queue::IdentityId;
use crate::core::index::documents_writer_delete_queue::DocumentsWriterDeleteQueue;
use crate::core::index::documents_writer_per_thread::{DocumentsWriterPerThread, State};
use crate::core::index::field_infos::build::Builder;
use crate::core::index::index_writer::{IndexWriter, IndexWriterBase};
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::lockable_concurrent_approximate_priority_queue::{
    Lock, LockableConcurrentApproximatePriorityQueue,
};
use crate::core::store::directory::Directory;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::{Condvar, Mutex, MutexGuard};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// [`DocumentsWriterPerThreadPool`] controls [`DocumentsWriterPerThread`] instances and their thread assignments during indexing.
/// Each [`DocumentsWriterPerThread`] is obtained from the pool and exclusively used for indexing a single document or list of documents by the obtaining thread.
/// Each indexing thread must obtain such a [`DocumentsWriterPerThread`] to make progress. Depending on the [`DocumentsWriterPerThreadPool`] implementation, [`DocumentsWriterPerThread`]
/// assignments might differ from document to document.
///
/// Once a [`DocumentsWriterPerThread`] is selected for flush, it will be checked out of the thread pool and won’t be reused for indexing. See [`checkout`](DocumentsWriterPerThreadPool::checkout)
pub(crate) struct DocumentsWriterPerThreadPool<D>
where
    D: Directory,
{
    pub(crate) inner: Mutex<Inner<D>>,
    free_list: LockableConcurrentApproximatePriorityQueue<Arc<DwptWrapper<D>>>,
    pausing: Condvar,
    closed: AtomicBool,
}
pub(crate) struct Inner<D>
where
    D: Directory,
{
    pub(crate) dwpts: HashMap<String, Arc<DwptWrapper<D>>>,
    taken_writer_permits: i32,
}

impl<D> DocumentsWriterPerThreadPool<D>
where
    D: Directory,
{
    pub fn new() -> Result<Self> {
        let inner = Mutex::new(Inner {
            dwpts: HashMap::new(),
            taken_writer_permits: 0,
        });
        Ok(Self {
            inner,
            free_list: LockableConcurrentApproximatePriorityQueue::new()?,
            pausing: Condvar::new(),
            closed: AtomicBool::new(false),
        })
    }
    /// Returns the active number of [`DocumentsWriterPerThread`] instances.
    pub(crate) fn size(&self) -> usize {
        let inner = self.inner.lock();
        inner.dwpts.len()
    }

    pub(crate) fn lock_new_writers(&self) {
        // this is similar to a semaphore - we need to acquire all permits ie. takenWriterPermits must
        // be == 0
        // any call to lockNewWriters() must be followed by unlockNewWriters() otherwise we will
        // deadlock at some
        // point
        let mut inner = self.inner.lock();
        debug_assert!(inner.taken_writer_permits >= 0);
        inner.taken_writer_permits += 1;
    }
    pub(crate) fn unlock_new_writers(&self) {
        let mut inner = self.inner.lock();
        debug_assert!(inner.taken_writer_permits > 0);
        inner.taken_writer_permits -= 1;

        if inner.taken_writer_permits == 0 {
            self.pausing.notify_all();
        }
    }
    pub(crate) fn new_dwpt<L, B>(
        index_writer: &IndexWriter<D, L, B>,
        delete_queue: Arc<DocumentsWriterDeleteQueue>,
    ) -> Result<Arc<DwptWrapper<D>>>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        let infos = Builder::new(index_writer.global_field_number_map.clone());
        let dwpt = DocumentsWriterPerThread::new(
            index_writer.get_index_major_version_created(),
            &index_writer.new_segment_name(None),
            index_writer.directory_orig.clone(),
            index_writer.directory.clone(),
            index_writer.config.as_ref(),
            delete_queue,
            infos,
            index_writer.pending_num_docs.clone(),
            index_writer.enable_test_points,
        )?;
        Ok(Arc::new(DwptWrapper::new(dwpt)))
    }
    /// Returns a new already locked [`DocumentsWriterPerThread`]
    pub(crate) fn new_writer<L, B>(
        &self,
        writer: &IndexWriter<D, L, B>,
        delete_queue: Arc<DocumentsWriterDeleteQueue>,
    ) -> Result<Arc<DwptWrapper<D>>>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        let mut inner = self.inner.lock();
        debug_assert!(inner.taken_writer_permits >= 0);
        while inner.taken_writer_permits > 0 {
            self.pausing.wait(&mut inner);
        }
        // we must check if we are closed since this might happen while we are waiting for the writer
        // permit
        // and if we miss that we might release a new DWPT even though the pool is closed. Yet, that
        // wouldn't be the
        // end of the world it's violating the contract that we don't release any new DWPT after this
        // pool is closed
        self.ensure_open()?;
        let dwpt = Self::new_dwpt(writer, delete_queue)?;
        dwpt.lock();

        inner.dwpts.insert(dwpt.id().to_string(), dwpt.clone());
        Ok(dwpt)
    }
    /// This method is used by `DocumentsWriter`/`FlushControl` to obtain a DWPT to do an indexing
    /// operation (add/updateDocument).
    pub(crate) fn get_and_lock<L, B>(
        &self,
        writer: &IndexWriter<D, L, B>,
        delete_queue: Arc<DocumentsWriterDeleteQueue>,
    ) -> Result<Arc<DwptWrapper<D>>>
    where
        L: LiveIndexWriterConfig,
        B: IndexWriterBase,
    {
        self.ensure_open()?;

        if let Some(dwpt) = self.free_list.lock_and_poll() {
            return Ok(dwpt);
        }
        // newWriter() adds the DWPT to the `dwpts` set as a side-effect. However it is not added to
        // `freeList` at this point, it will be added later on once DocumentsWriter has indexed a
        // document into this DWPT and then gives it back to the pool by calling
        // #marksAsFreeAndUnlock.
        self.new_writer(writer, delete_queue)
    }

    fn ensure_open(&self) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(LuceneError::already_closed("DWPTPool is already closed"));
        }
        Ok(())
    }

    pub(crate) fn contains(&self, state_id: &str) -> bool {
        let inner = self.inner.lock();
        inner.dwpts.contains_key(state_id)
    }
    pub(crate) fn mark_as_free_and_unlock(&self, wrap_dwpt: Arc<DwptWrapper<D>>) -> Result<()> {
        debug_assert!(wrap_dwpt.state.is_locked());
        let ram_bytes_used = wrap_dwpt.dwpt.lock().ram_bytes_used()?;

        debug_assert!(
            !wrap_dwpt.state.is_flush_pending()
                && !wrap_dwpt.state.is_aborted()
                && !wrap_dwpt.state.is_queue_advanced(),
            "DWPT has pending flush: {}, aborted={}, queueAdvanced={}",
            wrap_dwpt.state.is_flush_pending(),
            wrap_dwpt.state.is_aborted(),
            wrap_dwpt.state.is_queue_advanced()
        );

        debug_assert!(
            self.contains(&wrap_dwpt.state.id),
            "Tried to add a DWPT back to the pool but the pool doesn't know about this DWPT"
        );
        let v = match self.inner.lock().dwpts.get(&wrap_dwpt.state.id) {
            Some(v) => v.clone(),
            None => {
                return Err(LuceneError::illegal_state(
                    "Tried to add a DWPT back to the pool but the pool doesn't know about this DWPT",
                ));
            },
        };
        drop(wrap_dwpt);
        self.free_list.add_and_unlock(v, ram_bytes_used);
        Ok(())
    }
    pub(crate) fn iterator(
        &self,
        inner: Option<&Inner<D>>,
    ) -> HashMap<String, Arc<DwptWrapper<D>>> {
        let inner = match inner {
            Some(s) => s,
            None => &*self.inner.lock(),
        };
        inner.dwpts.clone()
    }

    /// Filters all `DocumentsWriterPerThread`s that the given predicate applies to and that can be checked out of the pool via [`checkout`](Self::checkout).
    /// All returned DWPTs are already locked, and [`is_registered`](Self::is_registered) will return `true` for each one.
    pub(crate) fn filter_and_lock<F1>(&self, predicate: F1) -> Result<Vec<Arc<DwptWrapper<D>>>>
    where
        F1: Fn(&Arc<DwptWrapper<D>>) -> bool,
    {
        let mut list = Vec::new();
        let inner = self.inner.lock();
        let cloned_dwpt = self.iterator(Some(&inner));
        for (id, state) in cloned_dwpt.iter() {
            if predicate(state) {
                state.lock();
                if self.is_registered_with_state(id, Some(&inner)) {
                    list.push(state.clone());
                } else {
                    state.state.unlock();
                }
            }
        }
        Ok(list)
    }
    /// Removes the given DWPT from the pool unless it has already been removed.
    ///
    /// # Returns
    ///
    /// `true` if the DWPT was removed; `false` otherwise.
    pub(crate) fn checkout(
        &self,
        per_thread: &MutexGuard<'_, DocumentsWriterPerThread<D>>,
    ) -> Option<Arc<DwptWrapper<D>>> {
        debug_assert!(per_thread.state.is_locked());
        let mut inner = self.inner.lock();
        match inner.dwpts.remove(&per_thread.state.id) {
            Some(v) => {
                self.free_list.remove(&per_thread.state.id);
                Some(v)
            },
            None => {
                debug_assert!(!self.free_list.contains(&per_thread.state.id));
                None
            },
        }
    }
    ///  Returns `true` if this DWPT is still part of the pool
    pub(crate) fn is_registered(&self, per_thread: &str) -> bool {
        let inner = self.inner.lock();
        self.is_registered_with_state(per_thread, Some(&inner))
    }
    fn is_registered_with_state(&self, per_thread: &str, state: Option<&Inner<D>>) -> bool {
        let state = match state {
            Some(s) => s,
            None => &*self.inner.lock(),
        };
        state.dwpts.contains_key(per_thread)
    }
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}
pub struct DwptWrapper<D>
where
    D: Directory,
{
    pub(crate) dwpt: Mutex<DocumentsWriterPerThread<D>>,
    pub(crate) state: Arc<State>,
}
impl<D> DwptWrapper<D>
where
    D: Directory,
{
    pub(crate) fn new(dwpt: DocumentsWriterPerThread<D>) -> Self {
        let state = dwpt.state.clone();
        Self {
            dwpt: Mutex::new(dwpt),
            state,
        }
    }
}

impl<D> Lock for Arc<DwptWrapper<D>>
where
    D: Directory,
{
    fn lock(&self) {
        self.state.lock()
    }

    fn try_lock(&self) -> bool {
        self.state.try_lock()
    }

    fn unlock(&self) {
        self.state.unlock()
    }

    fn is_locked(&self) -> bool {
        self.state.is_locked()
    }
}
impl<D> IdentityId for Arc<DwptWrapper<D>>
where
    D: Directory,
{
    fn id(&self) -> &str {
        &self.state.id
    }
}
impl<D> PartialEq for DwptWrapper<D>
where
    D: Directory,
{
    fn eq(&self, other: &Self) -> bool {
        self.state.id == other.state.id
    }
}
#[cfg(test)]
mod tests {

    use crate::core::index::approximate_priority_queue::IdentityId;
    use crate::core::index::documents_writer_delete_queue::DocumentsWriterDeleteQueue;

    use crate::core::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool;
    use crate::core::index::dummy::dummy_live_index_writer_config::DummyLiveIndexWriterConfig;

    use crate::core::index::lockable_concurrent_approximate_priority_queue::Lock;

    use crate::core::index::index_writer::IndexWriter;

    use crate::core::util::error::lucene_error::{LuceneError, Result};
    use crate::core::util::info_stream::{InfoStreamEnum, NoOutput};

    use crate::test::util::lucene_test_case::lucene_test_case_util::{new_directory, random};

    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestDocumentsWriterPerThreadPool;

    #[test]
    fn test_lock_release_and_close() -> Result<()> {
        let mut random = random();
        let directory_orig = Arc::new(new_directory(&mut random)?);
        // TODO: LuceneTestCase::newIndexWriterConfig 为实现
        let dummy_config = DummyLiveIndexWriterConfig::new();
        let iw = IndexWriter::new(directory_orig, dummy_config)?;
        let queue = Arc::new(DocumentsWriterDeleteQueue::new(Arc::new(
            InfoStreamEnum::NoOutput(NoOutput),
        )));

        let pool = DocumentsWriterPerThreadPool::new()?;
        let first = pool.get_and_lock(&iw, queue.clone())?;
        assert_eq!(pool.size(), 1);

        let second = pool.get_and_lock(&iw, queue.clone())?;
        assert_eq!(pool.size(), 2);

        let first_id = first.id().to_string();
        pool.mark_as_free_and_unlock(first)?;
        assert_eq!(pool.size(), 2);

        let third = pool.get_and_lock(&iw, queue.clone())?;
        assert_eq!(first_id, third.id().to_string());
        assert_eq!(pool.size(), 2);

        pool.checkout(&third.dwpt.lock());
        assert_eq!(pool.size(), 1);

        pool.close();
        assert_eq!(pool.size(), 1);

        pool.mark_as_free_and_unlock(second)?;
        assert_eq!(pool.size(), 1);

        let v = pool.filter_and_lock(|_| true)?;
        for dwpt in v {
            pool.checkout(&dwpt.dwpt.lock());
            assert!(dwpt.state.is_locked());
            dwpt.unlock();
        }
        assert_eq!(pool.size(), 0);
        Ok(())
    }
    #[test]
    fn test_close_while_new_writers_locked() -> Result<()> {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use std::thread;
        use std::time::Duration;

        let mut random = random();
        let directory_orig = Arc::new(new_directory(&mut random)?);
        // TODO: LuceneTestCase::newIndexWriterConfig 为实现
        let dummy_config = DummyLiveIndexWriterConfig::new();
        let iw = IndexWriter::new(directory_orig, dummy_config)?;
        let queue = Arc::new(DocumentsWriterDeleteQueue::new(Arc::new(
            InfoStreamEnum::NoOutput(NoOutput),
        )));

        let pool = Arc::new(DocumentsWriterPerThreadPool::new()?);

        let first = pool.get_and_lock(&iw, queue.clone())?;
        pool.lock_new_writers();

        let ready = Arc::new(AtomicBool::new(false));
        let ready_clone = ready.clone();
        let pool_clone = pool.clone();

        let handle = thread::spawn(move || {
            ready_clone.store(true, Ordering::SeqCst);
            let result = pool_clone.get_and_lock(&iw, queue.clone());
            assert!(matches!(result, Err(LuceneError::AlreadyClosed(_))));
        });

        while !ready.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(10));
        }

        thread::sleep(Duration::from_millis(1000));

        first.unlock();
        pool.close();
        pool.unlock_new_writers();

        handle.join().unwrap();
        for dwpt in pool.filter_and_lock(|_| true)? {
            assert!(pool.checkout(&dwpt.dwpt.lock()).is_some());
            dwpt.unlock();
        }

        assert_eq!(pool.size(), 0);
        Ok(())
    }
}
