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
use rlucene::index::documents_writer_delete_queue::{
    DocumentsWriterDeleteQueue, NodeEnum, TermNodeArray,
};
use rlucene::index::field_term_iterator::FieldTermIterator;
use rlucene::index::frozen_buffered_updates::FrozenBufferedUpdates;
use rlucene::index::term::Term;
use rlucene::index::BytesRefBuilder;
use rlucene::search::dummy::dummy_query::DummyQuery;
use rlucene::search::query::Query;
use rlucene::store::dummy::dummy_directory::DummyDirectory;
use rlucene::util::bytes_ref_iterator::BytesRefIterator;
use rlucene::util::error::lucene_error::LuceneError;
use rlucene::util::info_stream::get_default_info_stream;
use std::collections::HashSet;

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
