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
use crate::core::document::float_doc_values_field::FloatDocValuesField;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::reader_util::ReaderUtil;
use crate::core::index::term::Term;
use crate::core::search::collector::Collector;
use crate::core::search::collector_manager::CollectorManager;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::field_value_hit_queue::TopFieldScoreDoc;
use crate::core::search::index_searcher::DefaultIndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::{self, TopDocs};
use crate::core::search::top_docs_collector::TopDocsCollector;
use crate::core::search::top_field_collector_manager::TopFieldCollectorManager;
use crate::core::search::top_score_doc_collector_manager::TopScoreDocCollectorManager;
use crate::core::search::total_hits::{Relation, TotalHits};
use crate::core::search::weight::Weight;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use std::fmt;

#[allow(dead_code)] // for quick search
pub struct TestTopDocsMerge;

struct ShardSearcher<'a, IRC>
where
  IRC: IndexReaderContext + 'static,
{
  ctx_ord: usize,
  searcher: &'a DefaultIndexSearcher<IRC>,
}

impl<'a, IRC> ShardSearcher<'a, IRC>
where
  IRC: IndexReaderContext + 'static,
{
  fn new(
    ctx: &LeafReaderContext<IRC::LeafReader>,
    searcher: &'a DefaultIndexSearcher<IRC>,
  ) -> Self {
    Self {
      ctx_ord: ctx.ord,
      searcher,
    }
  }

  fn search<W, C>(&self, weight: &W, collector: &mut C) -> Result<()>
  where
    W: Weight<IRC> + ?Sized,
    C: Collector,
  {
    self
      .searcher
      .search_leaf(self.ctx_ord, 0, NO_MORE_DOCS, weight, collector)
  }

  fn search_top_docs<W>(&self, weight: &W, top_n: usize) -> Result<TopDocs<ScoreDoc>>
  where
    W: Weight<IRC> + ?Sized,
  {
    let manager = TopScoreDocCollectorManager::new(top_n, i32::MAX as usize)?;
    let mut collector = manager.new_collector()?;
    self.search(weight, &mut collector)?;
    collector.top_docs()
  }
}

impl<IRC> fmt::Display for ShardSearcher<'_, IRC>
where
  IRC: IndexReaderContext + 'static,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

#[test]
fn test_sort_1() -> Result<()> {
  test_sort(false)
}

#[test]
fn test_sort_2() -> Result<()> {
  test_sort(true)
}

#[test]
fn test_inconsistent_top_docs_fail() {
  let mut top_docs = vec![
    TopDocs::new(
      TotalHits::new(1, Relation::EqualTo),
      vec![ScoreDoc::with_shard_index(1, 1.0, 5)],
    ),
    TopDocs::new(
      TotalHits::new(1, Relation::EqualTo),
      vec![ScoreDoc::with_shard_index(1, 1.0, -1)],
    ),
  ];

  if random().random::<bool>() {
    top_docs.swap(0, 1);
  }

  let err = top_docs::merge_top_docs_with_start(0, 2, top_docs);
  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_pre_assigned_shard_index() -> Result<()> {
  let mut random = random();
  let use_constant_score = random.random::<bool>();
  let num_top_docs = 2 + random.random_range(0..10);
  let mut top_docs = Vec::with_capacity(num_top_docs);
  let mut shard_result_mapping = HashMap::new();
  let mut num_hits_total = 0usize;

  for i in 0..num_top_docs {
    let num_hits = 1 + random.random_range(0..10);
    num_hits_total += num_hits;
    let mut score_docs = Vec::with_capacity(num_hits);
    for j in 0..num_hits {
      let score = if use_constant_score {
        1.0
      } else {
        random.random::<f32>()
      };
      score_docs.push(ScoreDoc::with_shard_index(
        (100 * i + j) as i32,
        score,
        i as i32,
      ));
    }
    let shard_top_docs = TopDocs::new(TotalHits::new(num_hits, Relation::EqualTo), score_docs);
    top_docs.push(shard_top_docs.clone());
    shard_result_mapping.insert(i as i32, shard_top_docs);
  }

  top_docs.shuffle(&mut random);
  let from = random.random_range(0..(num_hits_total - 1));
  let size = 1 + random.random_range(0..(num_hits_total - from));

  let merge = top_docs::merge_top_docs_with_start(from, size, top_docs.clone())?;
  assert!(!merge.score_docs.is_empty());
  for score_doc in &merge.score_docs {
    assert_ne!(score_doc.shard_index, -1);
    let shard_top_docs = shard_result_mapping.get(&score_doc.shard_index);
    assert!(shard_top_docs.is_some());
    let found = shard_top_docs
      .unwrap()
      .score_docs
      .iter()
      .any(|shard_score_doc| shard_score_doc == score_doc);
    assert!(found);
  }

  top_docs.shuffle(&mut random);
  let merge2 = top_docs::merge_top_docs_with_start(from, size, top_docs)?;
  assert_eq!(merge.score_docs, merge2.score_docs);
  Ok(())
}

fn test_sort(use_from: bool) -> Result<()> {
  let mut random = random();
  let num_docs = at_least(&mut random, if is_night_mode() { 1000 } else { 100 }) as usize;
  let tokens = ["a", "b", "c", "d", "e"];

  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone())?;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut content = Vec::with_capacity(at_least(&mut random, 20) as usize);

  for _ in 0..content.capacity() {
    let mut s = String::new();
    let num_tokens = TestUtil::next_int(&mut random, 1, 10) as usize;
    for _ in 0..num_tokens {
      s.push_str(tokens[random.random_range(0..tokens.len())]);
      s.push(' ');
    }
    content.push(s);
  }

  for _doc_idx in 0..num_docs {
    let mut doc = Document::new();
    let content_idx = random.random_range(0..content.len());
    doc.add(SortedDocValuesField::new(
      "string",
      BytesRef::from_string(&TestUtil::random_realistic_unicode_string(&mut random)),
    ));
    doc.add(new_text_field(
      &mut random,
      "text",
      content[content_idx].clone(),
      Store::No,
      &mut field_to_type,
    )?);
    doc.add(FloatDocValuesField::new("float", random.random::<f32>()));

    let int_value = if random.random_range(0..100) == 17 {
      i32::MIN
    } else if random.random_range(0..100) == 17 {
      i32::MAX
    } else {
      random.random::<i32>()
    };
    doc.add(NumericDocValuesField::new("int", int_value as i64));
    w.add_document(&mut random, doc)?;
  }

  let reader = w.get_reader(&mut random)?;
  w.close(&mut random)?;

  // NOTE: sometimes reader has just one segment, which is
  // important to test.
  let searcher = new_searcher_with_reader(reader)?;
  let ctx = searcher.get_top_reader_context();
  let leaves = ctx.leaves()?;

  let mut sub_searchers = Vec::with_capacity(leaves.len());
  let mut doc_starts = Vec::with_capacity(leaves.len());
  let mut doc_base = 0;
  for leaf in leaves {
    sub_searchers.push(ShardSearcher::new(leaf, &searcher));
    doc_starts.push(doc_base);
    doc_base += leaf.reader().max_doc()? as usize;
  }

  let sort_fields = vec![
    SortField::with_reverse(Some("string"), SortFieldType::String, true)?,
    SortField::with_reverse(Some("string"), SortFieldType::String, false)?,
    SortField::with_reverse(Some("int"), SortFieldType::Int, true)?,
    SortField::with_reverse(Some("int"), SortFieldType::Int, false)?,
    SortField::with_reverse(Some("float"), SortFieldType::Float, true)?,
    SortField::with_reverse(Some("float"), SortFieldType::Float, false)?,
    SortField::with_reverse::<String>(None, SortFieldType::Score, true)?,
    SortField::with_reverse::<String>(None, SortFieldType::Score, false)?,
    SortField::with_reverse::<String>(None, SortFieldType::Doc, true)?,
    SortField::with_reverse::<String>(None, SortFieldType::Doc, false)?,
  ];

  let num_iters = at_least(&mut random, 300);
  for _iter in 0..num_iters {
    let query: Query = TermQuery::new(Term::from_text(
      "text",
      tokens[random.random_range(0..tokens.len())],
    ))
    .into();

    let sort = if random.random_range(0..10) == 4 {
      // Sort by score.
      None
    } else {
      let mut random_sort_fields =
        Vec::with_capacity(TestUtil::next_int(&mut random, 1, 3) as usize);
      for _ in 0..random_sort_fields.capacity() {
        random_sort_fields.push(sort_fields[random.random_range(0..sort_fields.len())].clone());
      }
      Some(Sort::with_fields(random_sort_fields)?)
    };

    let num_hits = TestUtil::next_int(&mut random, 1, num_docs as i32 + 5) as usize;
    // let num_hits = 5;

    let mut from = -1;
    let mut size = -1;

    // First search on whole index:
    let top_hits: TopDocs<TopFieldScoreDoc>;
    match sort.as_ref() {
      None => {
        if use_from {
          from = TestUtil::next_int(&mut random, 0, num_hits as i32 - 1);
          size = num_hits as i32 - from;
          let manager = TopScoreDocCollectorManager::new(num_hits, i32::MAX as usize)?;
          let temp_top_hits = searcher.search_with_collector_manager(query.clone(), &manager)?;
          if (from as usize) < temp_top_hits.score_docs.len() {
            // Cannot use `TopDocs::top_docs(start, how_many)`, since it behaves differently when
            // start >= hitCount than TopDocs#merge currently has.
            let end = std::cmp::min(
              from as usize + size as usize,
              temp_top_hits.score_docs.len(),
            );
            top_hits = TopDocs::new(
              temp_top_hits.total_hits,
              temp_top_hits.score_docs[from as usize..end]
                .iter()
                .cloned()
                .map(TopFieldScoreDoc::from)
                .collect(),
            );
          } else {
            top_hits = TopDocs::new(temp_top_hits.total_hits, vec![]);
          }
        } else {
          let hits = searcher.search(query.clone(), num_hits)?;
          top_hits = TopDocs::new(
            hits.total_hits,
            hits
              .score_docs
              .into_iter()
              .map(TopFieldScoreDoc::from)
              .collect(),
          );
        }
      },
      Some(sort) => {
        let manager = TopFieldCollectorManager::new(sort.clone(), num_hits, i32::MAX as usize)?;
        let mut top_field_docs = searcher.search_with_collector_manager(query.clone(), &manager)?;
        if use_from {
          from = TestUtil::next_int(&mut random, 0, num_hits as i32 - 1);
          size = num_hits as i32 - from;
          if (from as usize) < top_field_docs.base.score_docs.len() {
            // Cannot use `TopDocs::top_docs(start, how_many)`, since it behaves differently when
            // start >= hitCount than TopDocs#merge currently has.
            let end = std::cmp::min(
              from as usize + size as usize,
              top_field_docs.base.score_docs.len(),
            );
            top_field_docs.base.score_docs =
              top_field_docs.base.score_docs[from as usize..end].to_vec();
            top_hits = top_field_docs.base;
          } else {
            top_hits = TopDocs::new(top_field_docs.base.total_hits, vec![]);
          }
        } else {
          top_hits = top_field_docs.base;
        }
      },
    }

    // ... then all shards:
    let rewritten = searcher.rewrite(query.clone())?;
    let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

    let mut shard_hits = Vec::with_capacity(sub_searchers.len());
    for (shard_idx, sub_searcher) in sub_searchers.iter().enumerate() {
      let mut sub_hits = if let Some(sort) = sort.as_ref() {
        let manager = TopFieldCollectorManager::new(sort.clone(), num_hits, i32::MAX as usize)?;
        let mut collector = manager.new_collector()?;
        sub_searcher.search(&weight, &mut collector)?;
        collector.top_docs()?.base
      } else {
        let sub_hits = sub_searcher.search_top_docs(&weight, num_hits)?;
        TopDocs::new(
          sub_hits.total_hits,
          sub_hits
            .score_docs
            .into_iter()
            .map(TopFieldScoreDoc::from)
            .collect(),
        )
      };

      for score_doc in &mut sub_hits.score_docs {
        score_doc.set_shard_index(shard_idx as i32);
      }
      shard_hits.push(sub_hits);
    }

    // Merge:
    let merged_hits = if use_from {
      if let Some(sort) = sort.as_ref() {
        top_docs::merge_top_field_docs_with_start(sort, from as usize, size as usize, shard_hits)?
          .base
      } else {
        top_docs::merge_top_docs_with_start(from as usize, size as usize, shard_hits)?
      }
    } else if let Some(sort) = sort.as_ref() {
      top_docs::merge_top_field_docs(sort, num_hits, shard_hits)?.base
    } else {
      top_docs::merge_top_docs(num_hits, shard_hits)?
    };

    // Make sure the returned shards are correct:
    for score_doc in &merged_hits.score_docs {
      assert_eq!(
        ReaderUtil::sub_index(score_doc.doc() as usize, &doc_starts),
        score_doc.shard_index(),
        "doc={} wrong shard",
        score_doc.doc()
      );
    }

    TestUtil::assert_consistent(&top_hits, &merged_hits);
  }

  searcher.get_index_reader().close()?;
  dir.close()?;
  Ok(())
}
#[test]
fn test_merge_total_hits_relation() -> Result<()> {
  let top_docs1 = TopDocs::new(
    TotalHits::new(2, Relation::EqualTo),
    vec![ScoreDoc::with_shard_index(42, 2.0, 0)],
  );
  let top_docs2 = TopDocs::new(
    TotalHits::new(1, Relation::EqualTo),
    vec![ScoreDoc::with_shard_index(42, 2.0, 1)],
  );
  let top_docs3 = TopDocs::new(
    TotalHits::new(1, Relation::GreaterThanOrEqualTo),
    vec![ScoreDoc::with_shard_index(42, 2.0, 2)],
  );
  let top_docs4 = TopDocs::new(
    TotalHits::new(3, Relation::GreaterThanOrEqualTo),
    vec![ScoreDoc::with_shard_index(42, 2.0, 3)],
  );

  let merged1 = top_docs::merge_top_docs(1, vec![top_docs1.clone(), top_docs2.clone()])?;
  assert_eq!(TotalHits::new(3, Relation::EqualTo), merged1.total_hits);

  let merged2 = top_docs::merge_top_docs(1, vec![top_docs1.clone(), top_docs3.clone()])?;
  assert_eq!(
    TotalHits::new(3, Relation::GreaterThanOrEqualTo),
    merged2.total_hits
  );

  let merged3 = top_docs::merge_top_docs(1, vec![top_docs3.clone(), top_docs4.clone()])?;
  assert_eq!(
    TotalHits::new(4, Relation::GreaterThanOrEqualTo),
    merged3.total_hits
  );

  let merged4 = top_docs::merge_top_docs(1, vec![top_docs4, top_docs2])?;
  assert_eq!(
    TotalHits::new(4, Relation::GreaterThanOrEqualTo),
    merged4.total_hits
  );

  Ok(())
}
