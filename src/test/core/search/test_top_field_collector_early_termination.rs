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
use crate::core::document::field::Store;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::query::Query;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_field_collector::can_early_terminate;
use crate::core::search::top_field_collector_manager::TopFieldCollectorManager;
use crate::core::search::total_hits::Relation;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::check_hits::CheckHits;
use crate::test::core::util::DefaultCRReader;
use crate::test::core::util::lucene_test_case::{
  at_least_usize, new_directory_shared, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::prelude::IndexedRandom;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

#[allow(dead_code)] // for quick search
struct TestTopFieldCollectorEarlyTermination;
static SORT: LazyLock<Arc<Sort>> = LazyLock::new(|| {
  Arc::from(
    Sort::with_fields(vec![
      SortField::new(Some("ndv1"), SortFieldType::Long).unwrap(),
    ])
    .unwrap(),
  )
});
const FORCE_MERGE_MAX_SEGMENT_COUNT: i32 = 5;
fn random_document<R>(random: &mut R, terms: &[String]) -> Result<Document>
where
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(NumericDocValuesField::new(
    "ndv1",
    random.random_range(0..10),
  ));
  doc.add(NumericDocValuesField::new(
    "ndv2",
    random.random_range(0..10),
  ));
  doc.add(StringField::from_string(
    "s",
    terms.choose(random).unwrap(),
    Store::Yes,
  )?);
  Ok(doc)
}
fn create_random_index<R>(
  random: &mut R,
  single_sorted_segment: bool,
) -> Result<(DefaultCRReader, Vec<String>)>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;

  let num_docs = at_least_usize(random, 150);
  let num_terms = TestUtil::next_usize(random, 1, num_docs / 5);

  let mut random_terms = HashSet::new();
  while random_terms.len() < num_terms {
    random_terms.insert(TestUtil::random_simple_string(random));
  }

  let terms = random_terms.into_iter().collect::<Vec<_>>();

  let _seed: u64 = random.random();
  let analyzer = MockAnalyzer::new(random);
  let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);

  iwc.set_merge_scheduler(SerialMergeScheduler::new());
  iwc.set_index_sort(SORT.clone())?;

  let mut iw = RandomIndexWriter::with_config(random, dir.clone(), iwc);
  iw.set_do_random_force_merge(false);

  for i in 0..num_docs {
    let doc = random_document(random, &terms)?;
    iw.add_document(doc)?;

    if i == num_docs / 2 || (i != num_docs - 1 && random.random_range(0..8) == 0) {
      iw.commit()?;
    }

    if random.random_range(0..15) == 0 {
      let term = terms.choose(random).unwrap();
      iw.delete_documents_with_terms(vec![Term::from_text("s", term)])?;
    }
  }

  if single_sorted_segment {
    iw.force_merge(1)?;
  } else if random.random_bool(0.5) {
    iw.force_merge(FORCE_MERGE_MAX_SEGMENT_COUNT)?;
  }

  let reader = iw.get_reader()?;

  let v = if reader.num_docs()? == 0 {
    iw.add_document(Document::new())?;
    reader.close()?;
    iw.get_reader()?
  } else {
    reader
  };

  Ok((v, terms))
}
#[test]
fn test_early_termination() -> Result<()> {
  let mut random = random();
  do_test_early_termination(&mut random, false)
}

#[test]
fn test_early_termination_when_paging() -> Result<()> {
  let mut random = random();
  do_test_early_termination(&mut random, true)
}
fn do_test_early_termination<R>(random: &mut R, paging: bool) -> Result<()>
where
  R: Rng + ?Sized,
{
  let iters = at_least_usize(random, 1);

  for _ in 0..iters {
    let (reader, terms) = create_random_index(random, false)?;
    let reader = Arc::new(reader);

    for _ in 0..iters {
      let searcher = new_searcher_with_reader(reader.clone())?;

      let mut max_slice_size = 0;

      for slice in searcher.get_slices()?.as_slice() {
        let mut num_docs_slice = 0;

        for partition in &slice.partitions {
          let live_docs = searcher.get_leaf_contexts()?[partition.ctx]
            .reader()
            .get_live_docs()?;
          let max_doc = std::cmp::min(
            partition.max_doc_id,
            searcher.get_leaf_contexts()?[partition.ctx]
              .reader()
              .max_doc()?,
          );

          for doc in partition.min_doc_id..max_doc {
            if live_docs.is_none() || live_docs.as_ref().unwrap().get(doc as usize)? {
              num_docs_slice += 1;
            }
          }
        }

        max_slice_size = std::cmp::max(max_slice_size, num_docs_slice);
      }

      let num_hits = TestUtil::next_usize(random, 1, reader.max_doc()? as usize);

      let after = if paging {
        debug_assert!(searcher.get_index_reader().num_docs()? > 0);
        let mut td = searcher.search_with_sort(MatchAllDocsQuery::new(), 10, SORT.clone())?;
        let len = td.score_docs().len() - 1;
        let v = std::mem::take(&mut td.base.take_score_docs()[len]);
        v.into_field()
      } else {
        None
      };

      let manager1 = TopFieldCollectorManager::with_after(
        SORT.clone(),
        num_hits,
        after.clone(),
        i32::MAX as usize,
      )?;
      let manager2 = TopFieldCollectorManager::with_after(SORT.clone(), num_hits, after, 1)?;

      let query: Query = if random.random_bool(0.5) {
        let term = terms.choose(random).unwrap();
        TermQuery::new(Term::from_text("s", term)).into()
      } else {
        MatchAllDocsQuery::new().into()
      };

      let td1 = searcher.search_with_collector_manager(query.clone(), &manager1)?;
      let td2 = searcher.search_with_collector_manager(query.clone(), &manager2)?;

      assert_ne!(Relation::GreaterThanOrEqualTo, td1.total_hits().relation());

      if !paging && max_slice_size > num_hits && matches!(query, Query::MatchAllDocs(_)) {
        // Make sure that we sometimes early terminate
        assert_eq!(Relation::GreaterThanOrEqualTo, td2.total_hits().relation());
      }

      if td2.total_hits().relation() == Relation::GreaterThanOrEqualTo {
        assert!(td2.total_hits().value() >= td1.score_docs().len());
        assert!(td2.total_hits().value() <= reader.max_doc()? as usize);
      } else {
        assert_eq!(td2.total_hits().value(), td1.total_hits().value());
      }

      CheckHits::check_equal(&query, td1.score_docs(), td2.score_docs())?;
    }
  }

  Ok(())
}
#[test]
fn test_can_early_terminate_on_doc_id() -> Result<()> {
  assert!(can_early_terminate(
    &Sort::with_fields(vec![SortField::get_field_doc()?])?,
    Some(&Sort::with_fields(vec![SortField::get_field_doc()?])?)
  )?);

  assert!(can_early_terminate(
    &Sort::with_fields(vec![SortField::get_field_doc()?])?,
    None
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?,
    None
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?,
    Some(&Sort::with_fields(vec![SortField::with_reverse(
      Some("b"),
      SortFieldType::Long,
      false
    )?])?)
  )?);

  assert!(can_early_terminate(
    &Sort::with_fields(vec![SortField::get_field_doc()?])?,
    Some(&Sort::with_fields(vec![SortField::with_reverse(
      Some("b"),
      SortFieldType::Long,
      false
    )?])?)
  )?);

  assert!(can_early_terminate(
    &Sort::with_fields(vec![SortField::get_field_doc()?])?,
    Some(&Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::get_field_doc()?
    ])?)
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?,
    Some(&Sort::with_fields(vec![SortField::get_field_doc()?])?)
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::get_field_doc()?
    ])?,
    Some(&Sort::with_fields(vec![SortField::get_field_doc()?])?)
  )?);

  Ok(())
}
#[test]
fn test_can_early_terminate_on_prefix() -> Result<()> {
  assert!(can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?,
    Some(&Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?)
  )?);

  assert!(can_early_terminate(
    &Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?,
    Some(&Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?)
  )?);

  assert!(can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?,
    Some(&Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?)
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      true
    )?])?,
    None
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      true
    )?])?,
    Some(&Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?)
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?,
    Some(&Sort::with_fields(vec![SortField::with_reverse(
      Some("a"),
      SortFieldType::Long,
      false
    )?])?)
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?,
    Some(&Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("c"), SortFieldType::String, false)?,
    ])?)
  )?);

  assert!(!can_early_terminate(
    &Sort::with_fields(vec![
      SortField::with_reverse(Some("a"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?,
    Some(&Sort::with_fields(vec![
      SortField::with_reverse(Some("c"), SortFieldType::Long, false)?,
      SortField::with_reverse(Some("b"), SortFieldType::String, false)?,
    ])?)
  )?);

  Ok(())
}
