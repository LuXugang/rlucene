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
use crate::util::lucene_test_case::{random, random_multiplier};
use crate::util::test_error::TestError;
use rand::Rng;
use rlucene::index::buffered_updates::BufferedUpdates;
use rlucene::index::doc_values_type::DocValuesType;
use rlucene::index::doc_values_update::{
    BinaryDocValuesUpdate, DocValuesUpdate, DocValuesUpdateEnum,
};
use rlucene::index::documents_writer_delete_queue::{
    DeleteSlice, DocumentsWriterDeleteQueue, NodeEnum, TermNodeArray,
};
use rlucene::index::field_term_iterator::FieldTermIterator;
use rlucene::index::frozen_buffered_updates::FrozenBufferedUpdates;
use rlucene::index::term::Term;
use rlucene::index::{BytesRef, BytesRefBuilder};
use rlucene::search::dummy::dummy_query::DummyQuery;
use rlucene::search::query::Query;
use rlucene::search::term_query::TermQuery;
use rlucene::store::dummy::dummy_directory::DummyDirectory;
use rlucene::util::bytes_ref_iterator::BytesRefIterator;
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::info_stream::get_default_info_stream;
use std::collections::HashSet;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::{hash, thread, vec};

#[allow(dead_code)]
pub struct TestDocumentsWriterDeleteQueue;

#[test]
fn test_update_delete_slices() -> Result<(), TestError> {
    let mut random = random();
    let mut queue: DocumentsWriterDeleteQueue<DummyQuery> =
        DocumentsWriterDeleteQueue::new(get_default_info_stream());
    let size = 200 + random.gen_range(0..500) * random_multiplier();
    let mut ids: Vec<i32> = Vec::with_capacity(size as usize);
    for _ in 0..size {
        ids.push(random.gen());
    }
    let mut slice1 = queue.new_slice()?;
    let mut slice2 = queue.new_slice()?;
    let mut bd1 = BufferedUpdates::new("bd1".to_string());
    let mut bd2 = BufferedUpdates::new("bd2".to_string());
    let mut last1 = 0;
    let mut last2 = 0;
    let mut unique_values = HashSet::new();
    for (j, &id) in ids.iter().enumerate() {
        let term = Term::from_text("id".to_string(), &id.to_string());
        unique_values.insert(term.clone());
        queue.add_delete_term(Vec::from([term.clone()]))?;
        if random.gen_range(0..20) == 0 || j == (size - 1) as usize {
            queue.update_slice(&mut slice1)?;
            assert!(
                slice1.is_tail_item(&NodeEnum::TermNodeArray(TermNodeArray::new(Vec::from([
                    term.clone()
                ]))))
            );
            slice1.apply(&mut bd1, j as i32)?;
            test_assert_all_between(last1 as i32, j as i32, &mut bd1, &ids)?;
            last1 = j + 1;
        }
        if random.gen_range(0..10) == 5 || j == size as usize - 1 {
            queue.update_slice(&mut slice2)?;
            assert!(
                slice2.is_tail_item(&NodeEnum::TermNodeArray(TermNodeArray::new(Vec::from([
                    term.clone()
                ]))))
            );
            slice2.apply(&mut bd2, j as i32)?;
            test_assert_all_between(last2 as i32, j as i32, &mut bd2, &ids)?;
            last2 = j + 1;
        }
        let num_deletes = queue.num_global_term_deletes()? as usize;
        assert_eq!(unique_values.len(), num_deletes);
    }

    let bd1_terms_set: HashSet<Term> = bd1.delete_terms.key_set()?;
    let bd2_terms_set: HashSet<Term> = bd2.delete_terms.key_set()?;
    assert_eq!(unique_values, bd1_terms_set);
    assert_eq!(unique_values, bd2_terms_set);

    let frozen: FrozenBufferedUpdates<DummyDirectory, DummyQuery> =
        queue.freeze_global_buffer(None)?.unwrap();
    let mut iter = frozen.delete_terms.iterator();
    let mut frozen_set: HashSet<Term> = HashSet::new();
    let mut bytes_ref = BytesRefBuilder::new();
    while let Some(byte_ref) = iter.next()? {
        bytes_ref.copy_bytes_with_ref(&byte_ref)?;
        let term = Term::new(iter.field().to_string(), bytes_ref.get_bytes_ref());
        frozen_set.insert(term.clone());
    }
    assert_eq!(unique_values, frozen_set);
    let num_deletes_after = queue.num_global_term_deletes()?;
    assert_eq!(0, num_deletes_after, "num deletes must be 0 after freeze");

    Ok(())
}

fn test_assert_all_between<Q>(
    start: i32,
    end: i32,
    deletes: &mut BufferedUpdates<Q>,
    ids: &[i32],
) -> Result<(), LuceneError>
where
    Q: Query,
{
    for i in start..=end {
        let term = Term::from_text("id".to_string(), &ids[i as usize].to_string());
        assert_eq!(end, deletes.delete_terms.get(&term)?);
    }
    Ok(())
}
#[test]
fn test_clear() -> Result<(), TestError> {
    let mut random = random();
    let queue = DocumentsWriterDeleteQueue::new(get_default_info_stream());
    assert!(!queue.any_changes()?);
    queue.clear()?;
    assert!(!queue.any_changes()?);
    let size = 200 + random.gen_range(0..500) * random_multiplier();
    for i in 0..size {
        let term = Term::from_text("id".to_string(), &i.to_string());
        if random.gen_range(0..10) == 0 {
            queue.add_delete_query(Vec::from([Arc::new(TermQuery::new(term.clone()))]))?;
        } else {
            queue.add_delete_term(vec![term.clone()])?;
        }
        assert!(queue.any_changes()?);

        if random.gen_range(0..10) == 0 {
            queue.clear()?;
            queue.try_apply_global_slice()?;
            assert!(!queue.any_changes()?);
        }
    }

    Ok(())
}
#[test]
fn test_any_changes() -> Result<(), TestError> {
    let mut random = random();
    let mut queue = DocumentsWriterDeleteQueue::new(get_default_info_stream());
    let size = 200 + random.gen_range(0..500) * random_multiplier();
    let mut terms_since_freeze = 0;
    let mut queries_since_freeze = 0;

    for i in 0..size {
        let term = Term::from_text("id".to_string(), &i.to_string());
        if random.gen_range(0..10) == 0 {
            queue.add_delete_query(vec![Arc::new(TermQuery::new(term.clone()))])?;
            queries_since_freeze += 1;
        } else {
            queue.add_delete_term(vec![term.clone()])?;
            terms_since_freeze += 1;
        }

        assert!(queue.any_changes()?);

        if random.gen_range(0..5) == 0 {
            if let Some(frozen) = queue.freeze_global_buffer::<DummyDirectory>(None)? {
                assert_eq!(terms_since_freeze, frozen.delete_terms.size());
                assert_eq!(queries_since_freeze, frozen.delete_queries.len());
                terms_since_freeze = 0;
                queries_since_freeze = 0;
                assert!(!queue.any_changes()?);
            }
        }
    }
    Ok(())
}
#[test]
fn test_partially_applied_global_slice() -> Result<(), LuceneError> {
    let queue_: DocumentsWriterDeleteQueue<DummyQuery> =
        DocumentsWriterDeleteQueue::new(get_default_info_stream());
    let queue = Arc::new(Mutex::new(queue_));
    let lock = queue.lock().unwrap();
    let handle = thread::spawn({
        let queue = Arc::clone(&queue);
        move || {
            let term = Term::from_text("foo".to_string(), "bar");
            queue.lock().unwrap().add_delete_term(vec![term]).unwrap();
        }
    });
    drop(lock);
    handle.join().unwrap();
    let mut queue = queue.lock().unwrap();
    assert!(queue.any_changes()?);
    queue.try_apply_global_slice()?;
    assert!(queue.any_changes()?);
    let frozen_global_buffer_wrap = queue.freeze_global_buffer::<DummyDirectory>(None)?;
    assert!(frozen_global_buffer_wrap.is_some());
    let frozen_global_buffer = frozen_global_buffer_wrap.unwrap();
    assert!(frozen_global_buffer.any());
    assert_eq!(1, frozen_global_buffer.delete_terms.size());
    assert!(!queue.any_changes()?);
    Ok(())
}
#[test]
fn test_stress_delete_queue() -> Result<(), TestError> {
    let mut random = random();
    let queue = Arc::new(DocumentsWriterDeleteQueue::<DummyQuery>::new(
        get_default_info_stream(),
    ));
    let mut unique_values = HashSet::new();
    let size = 10000 + random.gen_range(0..500) * random_multiplier();
    let ids: Vec<i32> = (0..size).map(|_| random.gen()).collect();
    for id in &ids {
        unique_values.insert(Term::from_text("id".to_string(), &id.to_string()));
    }

    let barrier = Arc::new(Barrier::new(1));
    let index = Arc::new(AtomicI32::new(0));
    let num_threads = 2 + random.gen_range(0..5);

    let mut threads = Vec::new();
    for _ in 0..num_threads {
        let thread = UpdateThread::new(
            Arc::clone(&queue),
            Arc::clone(&index),
            ids.clone(),
            Arc::clone(&barrier),
        )?;
        threads.push(Arc::new(Mutex::new(thread)));
    }

    let mut handles = Vec::new();
    for thread in &threads {
        let thread = Arc::clone(thread);
        handles.push(thread::spawn(move || {
            let mut thread = thread.lock().unwrap();
            thread.run().expect("Thread execution failed");
        }));
    }
    for handle in handles {
        handle.join().expect("Thread join failed");
    }
    for thread in threads {
        let mut guard = thread.lock().unwrap();
        queue.update_slice(&mut guard.slice)?;
        let deletes = guard.deletes.clone();
        let mut deletes_guard = deletes.lock().unwrap();
        guard
            .slice
            .apply(&mut deletes_guard, BufferedUpdates::MAX_INT)?;
        assert_eq!(unique_values, deletes_guard.delete_terms.key_set()?);
    }

    queue.try_apply_global_slice()?;
    let mut frozen_set = HashSet::new();
    let frozen = queue.freeze_global_buffer::<DummyDirectory>(None)?.unwrap();
    let mut iter = frozen.delete_terms.iterator();
    let mut builder = BytesRefBuilder::new();
    while let Some(byte_ref) = iter.next()? {
        builder.copy_bytes_with_ref(&byte_ref)?;
        let term = Term::new(iter.field().to_string(), builder.get_bytes_ref());
        frozen_set.insert(term);
    }
    assert_eq!(unique_values.len(), frozen_set.len());
    assert_eq!(unique_values, frozen_set);
    assert_eq!(0, queue.num_global_term_deletes()?);
    Ok(())
}

#[test]
fn test_close() -> Result<(), TestError> {
    {
        let mut random = random();
        let mut queue = DocumentsWriterDeleteQueue::new(get_default_info_stream());
        assert!(queue.is_open()?);
        queue.close()?;
        if random.gen_bool(0.5) {
            queue.close()?; // double close
        }
        let result = queue.add_delete_term(vec![Term::from_text("foo".to_string(), "bar")]);
        matches!(result, Err(LuceneError::AlreadyClosed(_)));
        let result = queue.freeze_global_buffer::<DummyDirectory>(None);
        matches!(result, Err(LuceneError::AlreadyClosed(_)));
        let result = queue.add_delete_query(vec![Arc::new(TermQuery::new(Term::from_text(
            "foo".to_string(),
            "bar",
        )))]);
        matches!(result, Err(LuceneError::AlreadyClosed(_)));

        let sub_update = DocValuesUpdateEnum::Binary(BinaryDocValuesUpdate::new(Option::from(
            BytesRef::from_string(""),
        )));
        let update = DocValuesUpdate::new(
            DocValuesType::Binary,
            Term::from_text("id".to_string(), "0"),
            "enabled".to_string(),
            BufferedUpdates::MAX_INT,
            sub_update,
        );
        let result = queue.add_doc_values_updates(vec![update]);
        matches!(result, Err(LuceneError::AlreadyClosed(_)));
        let result = queue.maybe_freeze_global_buffer::<DummyDirectory>()?;
        assert!(result.is_none());
        assert!(!queue.is_open()?);
    }
    {
        let mut queue: DocumentsWriterDeleteQueue<DummyQuery> =
            DocumentsWriterDeleteQueue::new(get_default_info_stream());
        queue.add_delete_term(vec![Term::from_text("foo".to_string(), "bar")])?;
        let result = queue.close();
        matches!(result, Err(LuceneError::IllegalState(_)));

        assert!(queue.is_open()?);
        queue.try_apply_global_slice()?;
        queue.freeze_global_buffer::<DummyDirectory>(None)?;
        queue.close()?;
        assert!(!queue.is_open()?);
    }
    Ok(())
}

struct UpdateThread<Q>
where
    Q: Query,
{
    queue: Arc<DocumentsWriterDeleteQueue<Q>>,
    index: Arc<AtomicI32>,
    ids: Vec<i32>,
    slice: DeleteSlice<Q>,
    deletes: Arc<Mutex<BufferedUpdates<Q>>>,
    barrier: Arc<Barrier>,
}

impl<Q> UpdateThread<Q>
where
    Q: Query,
{
    fn new(
        queue: Arc<DocumentsWriterDeleteQueue<Q>>,
        index: Arc<AtomicI32>,
        ids: Vec<i32>,
        barrier: Arc<Barrier>,
    ) -> Result<Self, LuceneError> {
        let slice = queue.new_slice()?;
        let deletes = Arc::new(Mutex::new(BufferedUpdates::new("deletes".to_string())));

        Ok(UpdateThread {
            queue,
            index,
            ids,
            slice,
            deletes,
            barrier,
        })
    }
    fn run(&mut self) -> Result<(), LuceneError> {
        self.barrier.wait();
        let mut i = 0;
        while i < self.ids.len() {
            let term = Term::from_text("id".to_string(), &self.ids[i].to_string());
            let term_node = Arc::new(DocumentsWriterDeleteQueue::new_node_for_term(term));
            self.queue
                .add_with_slice(term_node.clone(), &mut self.slice)?;
            assert!(self.slice.is_tail(&term_node));

            let mut guard = self.deletes.lock().unwrap();
            self.slice.apply(&mut *guard, BufferedUpdates::MAX_INT)?;

            i = self.index.fetch_add(1, Ordering::SeqCst) as usize;
        }
        Ok(())
    }
}
