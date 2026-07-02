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
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, random,
};

use crate::core::index::documents_writer_per_thread_pool::DocumentsWriterPerThreadPool;

use crate::core::index::lockable_concurrent_approximate_priority_queue::Lock;

use crate::core::index::index_writer::IndexWriter;

use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::info_stream::{InfoStreamEnum, NoOutput};

use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestDocumentsWriterPerThreadPool;

#[test]
fn test_lock_release_and_close() -> Result<()> {
  let mut random = random();
  let directory_orig = new_directory_shared(&mut random)?;
  let iw = IndexWriter::new(directory_orig, new_index_writer_config(&mut random)?)?;
  let queue = Arc::new(DocumentsWriterDeleteQueue::new(Arc::new(
    InfoStreamEnum::NoOutput(NoOutput),
  )));

  let pool = DocumentsWriterPerThreadPool::new()?;
  let first = pool.get_and_lock(&iw, || queue.clone())?;
  assert_eq!(pool.size(), 1);

  let second = pool.get_and_lock(&iw, || queue.clone())?;
  assert_eq!(pool.size(), 2);

  let first_id = first.id().to_string();
  pool.mark_as_free_and_unlock(first)?;
  assert_eq!(pool.size(), 2);

  let third = pool.get_and_lock(&iw, || queue.clone())?;
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
  let directory_orig = new_directory_shared(&mut random)?;
  let iw = IndexWriter::new(directory_orig, new_index_writer_config(&mut random)?)?;
  let queue = Arc::new(DocumentsWriterDeleteQueue::new(Arc::new(
    InfoStreamEnum::NoOutput(NoOutput),
  )));

  let pool = Arc::new(DocumentsWriterPerThreadPool::new()?);

  let first = pool.get_and_lock(&iw, || queue.clone())?;
  pool.lock_new_writers();

  let ready = Arc::new(AtomicBool::new(false));
  let ready_clone = ready.clone();
  let pool_clone = pool.clone();

  let handle = thread::spawn(move || {
    ready_clone.store(true, Ordering::SeqCst);
    let result = pool_clone.get_and_lock(&iw, || queue.clone());
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
