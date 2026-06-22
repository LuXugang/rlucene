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
use crate::core::document::field_type::FieldType;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::get_context;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::field_comparator::FieldComparatorValue;
use crate::core::search::field_doc::FieldDoc;
use crate::core::search::index_searcher::{
  IndexSearcher, LeafReaderContextPartition, LeafSlice, do_slices,
};
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::query::Query;
use crate::core::search::query::QueryWeight;
use crate::core::search::query_caching_policy::{QueryCachingPolicy, QueryCachingPolicyEnum};
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::{QueryCache, QueryCacheEnum};
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_string_field, random,
};
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(dead_code)]
pub struct TestIndexSearcher {
  pub(crate) dir: Arc<DirEnum>,
  pub(crate) reader: StandardDirectoryReaderType<DirEnum>,
}

impl TestIndexSearcher {
  pub fn set_up<R>(random: &mut R) -> Result<Self>
  where
    R: rand::Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    let iw = RandomIndexWriter::new(random, dir.clone());
    let mut field_to_type: HashMap<String, FieldType> = HashMap::new();

    for i in 0..100 {
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "field",
        i.to_string(),
        Store::No,
        &mut field_to_type,
      )?);

      let field2_value = if i % 2 == 0 { "true" } else { "false" };
      doc.add(new_string_field(
        random,
        "field2",
        field2_value,
        Store::No,
        &mut field_to_type,
      )?);
      doc.add(SortedDocValuesField::new(
        "field2",
        BytesRef::from_string(field2_value),
      ));
      iw.add_document(random, doc)?;

      if random.random_bool(0.5) {
        iw.commit(random)?;
      }
    }

    let reader = iw.get_reader(random)?;
    iw.close(random)?;

    Ok(Self { dir, reader })
  }
}

#[test]
fn test_huge_n() -> Result<()> {
  let mut random = random();
  let TestIndexSearcher { dir: _dir, reader } = TestIndexSearcher::set_up(&mut random)?;

  let reader = Arc::new(reader);
  let searchers = vec![
    IndexSearcher::from_cr(reader.clone())?,
    IndexSearcher::from_cr_with_thread(reader.clone(), 4)?,
  ];
  let queries: Vec<Query> = vec![
    MatchAllDocsQuery::new().into(),
    TermQuery::new(Term::from_text("field", "1")).into(),
  ];
  let sorts = vec![
    None,
    Some(Sort::with_fields(vec![SortField::new(
      Some("field2"),
      SortFieldType::String,
    )?])?),
  ];
  let afters: Vec<Option<FieldDoc>> = vec![
    None,
    Some(FieldDoc::with_fields(
      0,
      0.0,
      vec![FieldComparatorValue::TermVal(BytesRef::from_string("boo!"))],
    )),
  ];

  for searcher in &searchers {
    for after in &afters {
      for query in &queries {
        for sort in &sorts {
          searcher.search(query.clone(), usize::MAX)?;
          searcher.search_after_score(
            after.as_ref().map(|a| a.base.clone()),
            query.clone(),
            usize::MAX,
          )?;
          if let Some(sort) = sort {
            searcher.search_with_sort(query.clone(), usize::MAX, sort.clone())?;
            searcher.search_with_sort_score(query.clone(), usize::MAX, sort.clone(), true)?;
            searcher.search_with_sort_score(query.clone(), usize::MAX, sort.clone(), false)?;
            searcher.search_after(after.clone(), query.clone(), usize::MAX, sort.clone())?;
            searcher.search_after_field_with_score(
              after.clone(),
              query.clone(),
              usize::MAX,
              sort.clone(),
              true,
            )?;
            searcher.search_after_field_with_score(
              after.clone(),
              query.clone(),
              usize::MAX,
              sort.clone(),
              false,
            )?;
          }
        }
      }
    }
  }

  Ok(())
}

#[test]
fn test_search_after_passed_max_doc() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());
  w.add_document(&mut random, Document::new())?;
  let r = w.get_reader(&mut random)?;
  w.close(&mut random)?;

  let max_doc = r.max_doc()?;
  let s = IndexSearcher::from_cr(r)?;
  let err = match s.search_after_score(
    Some(ScoreDoc::new(max_doc, 0.54)),
    MatchAllDocsQuery::new(),
    10,
  ) {
    Ok(_) => panic!("searchAfter after maxDoc should fail"),
    Err(err) => err,
  };
  assert!(matches!(err, LuceneError::IllegalArgument(_)));

  Ok(())
}

#[test]
fn test_count() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());
  let num_docs = at_least(&mut random, 100);

  for _ in 0..num_docs {
    let mut doc = Document::new();
    if random.random_bool(0.5) {
      doc.add(StringField::from_string("foo", "bar", Store::No)?);
    }
    if random.random_bool(0.5) {
      doc.add(StringField::from_string("foo", "baz", Store::No)?);
    }
    if random.random_range(0..100) == 0 {
      doc.add(StringField::from_string("delete", "yes", Store::No)?);
    }
    w.add_document(&mut random, doc)?;
  }

  for delete in [false, true] {
    if delete {
      w.delete_documents_with_terms(&mut random, vec![Term::from_text("delete", "yes")])?;
    }

    let reader = w.get_reader(&mut random)?;
    let searcher = IndexSearcher::from_cr(reader)?;

    let mut boolean_query = Builder::new();
    boolean_query.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
    boolean_query.add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;

    let queries: Vec<Query> = vec![
      MatchAllDocsQuery::new().into(),
      MatchNoDocsQuery::new().into(),
      TermQuery::new(Term::from_text("foo", "bar")).into(),
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "baz"))).into(),
      boolean_query.build().into(),
    ];

    for query in queries {
      assert_eq!(
        searcher.count(query.clone())?,
        searcher.search(query, 1)?.total_hits.value as i32
      );
    }
  }

  w.close(&mut random)?;
  Ok(())
}

#[test]
fn test_get_query_cache() -> Result<()> {
  let mut searcher = IndexSearcher::from_cr(MultiReader::empty()?)?;
  assert!(searcher.get_query_cache().is_some());

  let dummy_cache = QueryCacheEnum::custom(DummyQueryCache);
  searcher.set_query_cache(Some(dummy_cache));
  assert!(matches!(
    searcher.get_query_cache(),
    Some(QueryCacheEnum::Custom(_))
  ));

  searcher.set_query_cache(None);
  assert!(searcher.get_query_cache().is_none());

  Ok(())
}

#[test]
fn test_get_query_caching_policy() -> Result<()> {
  let mut searcher = IndexSearcher::from_cr(MultiReader::empty()?)?;
  assert!(matches!(
    searcher.get_query_caching_policy().as_ref(),
    QueryCachingPolicyEnum::UsageTracking(_)
  ));

  searcher.set_query_caching_policy(QueryCachingPolicyEnum::custom(DummyQueryCachingPolicy));
  assert!(matches!(
    searcher.get_query_caching_policy().as_ref(),
    QueryCachingPolicyEnum::Custom(_)
  ));

  Ok(())
}

#[test]
fn test_get_slices_no_leaves_no_executor() -> Result<()> {
  let searcher = IndexSearcher::from_cr(MultiReader::empty()?)?;
  let slices = searcher.get_slices()?;
  assert_eq!(0, slices.len());
  Ok(())
}
#[test]
fn test_get_slices_no_leaves_with_executor() -> Result<()> {
  let searcher = IndexSearcher::from_cr(MultiReader::empty()?)?;
  let slices = searcher.get_slices()?;
  assert_eq!(0, slices.len());
  Ok(())
}

#[test]
fn test_get_slices() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  for _ in 0..10 {
    w.add_document(&mut random, Document::new())?;
    w.flush()?;
  }

  let r = Arc::new(w.get_reader(&mut random)?);
  w.close(&mut random)?;

  let context = get_context(r.clone())?;
  let leaves_len = context.leaves()?.len();
  let searcher = IndexSearcher::new(context)?;
  let slices = searcher.get_slices()?;
  assert_eq!(1, slices.len());
  assert_eq!(leaves_len, slices[0].partitions.len());

  let context = get_context(r)?;
  let mut searcher = IndexSearcher::with_threads(context, 2)?;
  searcher.set_slice_strategy(|leaves| do_slices(leaves, 1, 1, false));
  let slices = searcher.get_slices()?;
  for slice in slices.iter() {
    assert_eq!(1, slice.partitions.len());
  }
  assert_eq!(leaves_len, slices.len());

  Ok(())
}

#[test]
fn test_slices_offloaded_to_the_executor() -> Result<()> {
  let mut random = random();
  let TestIndexSearcher { dir: _dir, reader } = TestIndexSearcher::set_up(&mut random)?;

  let context = get_context(reader)?;
  let leaves_len = context.leaves()?.len();
  let mut searcher = IndexSearcher::with_threads(context, leaves_len.max(1))?;
  searcher.set_slice_strategy(|leaves| {
    leaves
      .iter()
      .map(|ctx| {
        Ok(LeafSlice::new(vec![
          LeafReaderContextPartition::create_for_entire_segment(ctx)?,
        ]))
      })
      .collect()
  });
  let num_executions = Arc::new(AtomicUsize::new(0));
  searcher.set_offloaded_slice_counter(num_executions.clone());

  searcher.search(MatchAllDocsQuery::new(), 10)?;
  let expected_executions = if leaves_len > 1 { leaves_len } else { 0 };
  assert_eq!(expected_executions, num_executions.load(Ordering::Relaxed));

  Ok(())
}

#[test]
fn test_null_executor_non_null_task_executor() -> Result<()> {
  test_not_required_in_rust_lucene!();
}

#[test]
fn test_segment_partitions_same_slice() -> Result<()> {
  let mut random = random();
  let TestIndexSearcher { dir: _dir, reader } = TestIndexSearcher::set_up(&mut random)?;

  let context = get_context(reader)?;
  let mut searcher = IndexSearcher::with_threads(context, 2)?;
  searcher.set_slice_strategy(|leaves| {
    leaves
      .iter()
      .map(|ctx| {
        Ok(LeafSlice::new(vec![
          LeafReaderContextPartition::create_from_and_to(ctx, 0, 1)?,
          LeafReaderContextPartition::create_from_and_to(ctx, 1, ctx.reader().max_doc()?)?,
        ]))
      })
      .collect()
  });

  for ctx in searcher.get_leaf_contexts()? {
    if ctx.reader().max_doc()? <= 1 {
      // mock Java's assumeTrue
      return Ok(());
    }
  }

  let error = match searcher.get_slices() {
    Ok(_) => panic!("get_slices should reject multiple partitions of the same leaf in one slice"),
    Err(error) => error,
  };
  assert!(matches!(
    error,
    LuceneError::IllegalState(ref error)
      if error.message == "The same slice targets multiple leaf partitions of the same leaf reader context. A physical segment should rather get partitioned to be searched concurrently from as many slices as the number of leaf partitions it is split into."
  ));

  Ok(())
}

struct DummyQueryCache;

impl<IRC> QueryCache<IRC> for DummyQueryCache
where
  IRC: IndexReaderContext + 'static,
{
  fn do_cache(
    &self,
    weight: QueryWeight<IRC>,
    _policy: Arc<QueryCachingPolicyEnum>,
  ) -> QueryWeight<IRC>
  where
    IRC: IndexReaderContext + 'static,
  {
    weight
  }
}

struct DummyQueryCachingPolicy;

impl QueryCachingPolicy for DummyQueryCachingPolicy {
  fn on_use(&self, _query: &Query) {}

  fn should_cache(&self, _query: &Query) -> Result<bool> {
    Ok(false)
  }
}
