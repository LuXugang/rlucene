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
use crate::core::document::keyword_field::KeywordField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::abstract_multi_term_query_constant_score_wrapper::BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::DOC_VALUES_REWRITE;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::query_caching_policy::QueryCachingPolicy;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_in_set_query::TermInSetQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::usage_tracking_query_caching_policy::UsageTrackingQueryCachingPolicy;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::core_helper::CoreHelper;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_bytes_ref_from_bytes, new_bytes_ref_from_string, new_directory_shared,
  new_searcher_with_reader, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use rand::seq::SliceRandom;
use std::collections::HashSet;

#[allow(dead_code)] // for quick search
struct TestTermInSetQuery;

#[test]
fn test_all_docs_in_field_term() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  let field = "f";

  let dense_term_string = TestUtil::random_analysis_string(&mut random, 10, true);
  let dense_term = new_bytes_ref_from_string(&mut random, &dense_term_string)?;

  let mut random_terms = HashSet::new();
  while random_terms.len() < BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD {
    let term_string = TestUtil::random_analysis_string(&mut random, 10, true);
    random_terms.insert(new_bytes_ref_from_string(&mut random, &term_string)?);
  }
  assert_eq!(BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD, random_terms.len());
  let other_terms: Vec<_> = random_terms.iter().cloned().collect();

  let num_docs = 10 * other_terms.len();
  for i in 0..num_docs {
    let mut doc = Document::new();
    doc.add(StringField::from_bytes_ref(
      field,
      dense_term.clone(),
      Store::No,
    )?);
    let sparse_term = other_terms[i % other_terms.len()].clone();
    doc.add(StringField::from_bytes_ref(field, sparse_term, Store::No)?);
    writer.add_document(doc)?;
  }

  for _ in 0..100 {
    let mut doc = Document::new();
    doc.add(StringField::from_string("foo", "bar", Store::No)?);
  }

  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut query_terms = other_terms;
  query_terms.push(dense_term);

  let query = TermInSetQuery::new(field, query_terms);
  let top_docs = searcher.search(query, num_docs)?;
  assert_eq!(num_docs, top_docs.total_hits().value());

  writer.close()?;
  Ok(())
}

// TODO IMPORTANT DocValuesRewriteMethod未实现
fn test_duel() -> Result<()> {
  let mut random = random();
  let iters = at_least(&mut random, 2);
  let field = "f";
  for _ in 0..iters {
    let mut all_terms = Vec::new();
    let max_terms_power = TestUtil::next_int(&mut random, 1, 10);
    let num_terms = TestUtil::next_int(&mut random, 1, 1 << max_terms_power);
    for _ in 0..num_terms {
      let value = TestUtil::random_analysis_string(&mut random, 10, true);
      all_terms.push(new_bytes_ref_from_string(&mut random, &value)?);
    }
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir);
    let num_docs = at_least(&mut random, 10_000);
    for _ in 0..num_docs {
      let mut doc = Document::new();
      let term = all_terms[random.random_range(0..all_terms.len())].clone();
      doc.add(StringField::from_bytes_ref(field, term.clone(), Store::No)?);
      doc.add(SortedSetDocValuesField::indexed_field(field, term));
      writer.add_document(doc)?;
    }
    if num_terms > 1 && random.random_bool(0.5) {
      writer.delete_documents_with_terms(vec![Term::new(field, all_terms[0].clone())])?;
    }
    writer.commit()?;
    let reader = writer.get_reader()?;
    let searcher = new_searcher_with_reader(reader)?;
    writer.close()?;

    if searcher.get_index_reader().num_docs()? == 0 {
      continue;
    }

    for _ in 0..100 {
      let boost = random.random::<f32>() * 10.0;
      let max_query_terms_power = TestUtil::next_int(&mut random, 1, 8);
      let num_query_terms = TestUtil::next_int(&mut random, 1, 1 << max_query_terms_power);
      let mut query_terms = Vec::new();
      for _ in 0..num_query_terms {
        query_terms.push(all_terms[random.random_range(0..all_terms.len())].clone());
      }
      let mut bq = BooleanQueryBuilder::new();
      for term in &query_terms {
        bq.add(
          TermQuery::new(Term::new(field, term.clone())),
          Occur::Should,
        )?;
      }
      let q1: Query = ConstantScoreQuery::new(bq.build()).into();
      let q2: Query = TermInSetQuery::new(field, query_terms.clone()).into();
      let q3: Query =
        TermInSetQuery::new_with_rewrite_method(DOC_VALUES_REWRITE, field, query_terms).into();
      assert_same_matches(
        &searcher,
        BoostQuery::new(q1.clone(), boost)?.into(),
        BoostQuery::new(q2, boost)?.into(),
        true,
      )?;
      assert_same_matches(
        &searcher,
        BoostQuery::new(q1, boost)?.into(),
        BoostQuery::new(q3, boost)?.into(),
        false,
      )?;
    }
  }
  Ok(())
}

#[test]
fn test_returns_null_score_supplier() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir);
  for ch in 'a'..='z' {
    let mut doc = Document::new();
    let value = ch.to_string();
    doc.add(KeywordField::from_string("id", value.clone(), Store::Yes)?);
    doc.add(KeywordField::from_string("content", value, Store::Yes)?);
    writer.add_document(doc)?;
  }
  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  writer.close()?;

  let mut terms = Vec::new();
  for ch in 'a'..='z' {
    terms.push(new_bytes_ref_from_string(&mut random, &ch.to_string())?);
  }
  let query2: Query = TermInSetQuery::new("content", terms).into();

  {
    let query1: Query = TermInSetQuery::new(
      "id",
      vec![
        new_bytes_ref_from_string(&mut random, "aaa")?,
        new_bytes_ref_from_string(&mut random, "bbb")?,
      ],
    )
    .into();
    let mut query_builder = BooleanQueryBuilder::new();
    query_builder.add(query1.clone(), Occur::Filter)?;
    query_builder.add(query2.clone(), Occur::Filter)?;
    let bool_query: Query = query_builder.build().into();

    let ctx = &searcher.get_leaf_contexts()?[0];
    let rewritten_query1 = searcher.rewrite(query1)?;
    let weight1 = searcher.create_weight(rewritten_query1, ScoreMode::Complete, 1.0)?;
    let scorer_supplier1 = weight1.scorer_supplier(ctx, &searcher)?;
    assert!(scorer_supplier1.is_none());
    let rewritten_bool_query = searcher.rewrite(bool_query)?;
    let weight = searcher.create_weight(rewritten_bool_query, ScoreMode::Complete, 1.0)?;
    let scorer_supplier = weight.scorer_supplier(ctx, &searcher)?;
    assert!(scorer_supplier.is_none());
  }

  {
    let query1: Query = TermInSetQuery::new(
      "id",
      vec![
        new_bytes_ref_from_string(&mut random, "aaa")?,
        new_bytes_ref_from_string(&mut random, "bbb")?,
        new_bytes_ref_from_string(&mut random, "b")?,
      ],
    )
    .into();
    let mut query_builder = BooleanQueryBuilder::new();
    query_builder.add(query1.clone(), Occur::Filter)?;
    query_builder.add(query2, Occur::Filter)?;
    let bool_query: Query = query_builder.build().into();

    let ctx = &searcher.get_leaf_contexts()?[0];
    let rewritten_query1 = searcher.rewrite(query1)?;
    let weight1 = searcher.create_weight(rewritten_query1, ScoreMode::Complete, 1.0)?;
    let scorer_supplier1 = weight1.scorer_supplier(ctx, &searcher)?;
    assert!(scorer_supplier1.is_some());
    let rewritten_bool_query = searcher.rewrite(bool_query)?;
    let weight = searcher.create_weight(rewritten_bool_query, ScoreMode::Complete, 1.0)?;
    let scorer_supplier = weight.scorer_supplier(ctx, &searcher)?;
    assert!(scorer_supplier.is_some());
  }

  Ok(())
}

// TODO IMPORTANT DocValuesRewriteMethod未实现
fn test_skipper_optimization_gap_assumption() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir);
  for _ in 0..10_000 {
    let mut doc = Document::new();
    let term = new_bytes_ref_from_string(&mut random, "b")?;
    doc.add(SortedSetDocValuesField::new("field", term.clone()));
    doc.add(SortedSetDocValuesField::indexed_field("idx_field", term));
    writer.add_document(doc)?;
  }

  let mut doc = Document::new();
  let term = new_bytes_ref_from_string(&mut random, "a")?;
  doc.add(SortedSetDocValuesField::new("field", term.clone()));
  doc.add(SortedSetDocValuesField::indexed_field("idx_field", term));
  writer.add_document(doc)?;

  let mut doc = Document::new();
  let term = new_bytes_ref_from_string(&mut random, "c")?;
  doc.add(SortedSetDocValuesField::new("field", term.clone()));
  doc.add(SortedSetDocValuesField::indexed_field("idx_field", term));
  writer.add_document(doc)?;

  writer.commit()?;
  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;
  writer.close()?;

  let query_terms = vec![
    new_bytes_ref_from_string(&mut random, "a")?,
    new_bytes_ref_from_string(&mut random, "c")?,
  ];
  let q1: Query =
    TermInSetQuery::new_with_rewrite_method(DOC_VALUES_REWRITE, "field", query_terms.clone())
      .into();
  let q2: Query =
    TermInSetQuery::new_with_rewrite_method(DOC_VALUES_REWRITE, "idx_field", query_terms).into();
  assert_same_matches(&searcher, q1, q2, false)?;

  Ok(())
}

fn assert_same_matches<IRC>(
  searcher: &IndexSearcher<IRC>,
  q1: Query,
  q2: Query,
  scores: bool,
) -> Result<()>
where
  IRC: IndexReaderContext,
{
  let max_doc = searcher.get_index_reader().max_doc()? as usize;
  let td1 = searcher.search(q1, max_doc)?;
  let td2 = searcher.search(q2, max_doc)?;
  assert_eq!(td1.total_hits().value(), td2.total_hits().value());
  for i in 0..td1.score_docs.len() {
    assert_eq!(td1.score_docs[i].doc, td2.score_docs[i].doc);
    if scores {
      assert!(
        (td1.score_docs[i].score - td2.score_docs[i].score).abs() <= 10e-7,
        "score for {i} was not the same"
      );
    }
  }
  Ok(())
}

#[test]
fn test_hash_code_and_equals() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 100);
  let mut terms = Vec::new();
  let mut unique_terms = HashSet::new();
  for _ in 0..num {
    let string = TestUtil::random_realistic_unicode_string(&mut random);
    terms.push(new_bytes_ref_from_string(&mut random, &string)?);
    unique_terms.insert(new_bytes_ref_from_string(&mut random, &string)?);
    let left = TermInSetQuery::new("field", unique_terms.iter().cloned().collect());
    terms.shuffle(&mut random);
    let right = TermInSetQuery::new("field", terms.clone());
    assert_eq!(right, left);
    assert_eq!(
      CoreHelper::calculate_hash(&right),
      CoreHelper::calculate_hash(&left)
    );
    if unique_terms.len() > 1 {
      let mut as_list: Vec<_> = unique_terms.iter().cloned().collect();
      as_list.remove(0);
      let not_equal = TermInSetQuery::new("field", as_list);
      assert_ne!(left, not_equal);
      assert_ne!(right, not_equal);
    }
  }

  let mut tq1 = TermInSetQuery::new(
    "thing",
    vec![new_bytes_ref_from_string(&mut random, "apple")?],
  );
  let mut tq2 = TermInSetQuery::new(
    "thing",
    vec![new_bytes_ref_from_string(&mut random, "orange")?],
  );
  assert_ne!(
    CoreHelper::calculate_hash(&tq1),
    CoreHelper::calculate_hash(&tq2)
  );

  tq1 = TermInSetQuery::new(
    "thing",
    vec![new_bytes_ref_from_string(&mut random, "apple")?],
  );
  tq2 = TermInSetQuery::new(
    "thing2",
    vec![new_bytes_ref_from_string(&mut random, "apple")?],
  );
  assert_ne!(
    CoreHelper::calculate_hash(&tq1),
    CoreHelper::calculate_hash(&tq2)
  );
  Ok(())
}

#[test]
fn test_simple_equals() -> Result<()> {
  let mut random = random();
  let left = TermInSetQuery::new(
    "id",
    vec![
      new_bytes_ref_from_string(&mut random, "AaAaAa")?,
      new_bytes_ref_from_string(&mut random, "AaAaBB")?,
    ],
  );
  let right = TermInSetQuery::new(
    "id",
    vec![
      new_bytes_ref_from_string(&mut random, "AaAaAa")?,
      new_bytes_ref_from_string(&mut random, "BBBBBB")?,
    ],
  );
  assert_ne!(left, right);
  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  let mut random = random();
  let terms_query = TermInSetQuery::new(
    "field1",
    vec![
      new_bytes_ref_from_string(&mut random, "a")?,
      new_bytes_ref_from_string(&mut random, "b")?,
      new_bytes_ref_from_string(&mut random, "c")?,
    ],
  );
  assert_eq!("field1:(a b c)", terms_query.as_string("")?);
  Ok(())
}

#[test]
fn test_dedup() -> Result<()> {
  let mut random = random();
  let query1 = TermInSetQuery::new("foo", vec![new_bytes_ref_from_string(&mut random, "bar")?]);
  let query2 = TermInSetQuery::new(
    "foo",
    vec![
      new_bytes_ref_from_string(&mut random, "bar")?,
      new_bytes_ref_from_string(&mut random, "bar")?,
    ],
  );
  QueryUtils::check_equal(&query1, &query2);
  Ok(())
}

#[test]
fn test_order_does_not_matter() -> Result<()> {
  let mut random = random();
  let query1 = TermInSetQuery::new(
    "foo",
    vec![
      new_bytes_ref_from_string(&mut random, "bar")?,
      new_bytes_ref_from_string(&mut random, "baz")?,
    ],
  );
  let query2 = TermInSetQuery::new(
    "foo",
    vec![
      new_bytes_ref_from_string(&mut random, "baz")?,
      new_bytes_ref_from_string(&mut random, "bar")?,
    ],
  );
  QueryUtils::check_equal(&query1, &query2);
  Ok(())
}

#[test]
fn test_ram_bytes_used() -> Result<()> {
  // TODO: memory calculation not implement
  Ok(())
}

#[test]
fn test_pull_one_terms_enum() -> Result<()> {
  // TODO IMPORTANT TermsCountingSubReaderWrapper未实现
  Ok(())
}

#[test]
fn test_binary_to_string() -> Result<()> {
  let mut random = random();
  let query = TermInSetQuery::new(
    "field",
    vec![new_bytes_ref_from_bytes(&mut random, &[0xff, 0xfe])?],
  );
  assert_eq!("field:([ff fe])", query.as_string("")?);
  Ok(())
}

#[test]
fn test_is_considered_costly_by_query_cache() -> Result<()> {
  let mut random = random();
  let query: Query = TermInSetQuery::new(
    "foo",
    vec![
      new_bytes_ref_from_string(&mut random, "bar")?,
      new_bytes_ref_from_string(&mut random, "baz")?,
    ],
  )
  .into();
  let policy = UsageTrackingQueryCachingPolicy::new()?;
  assert!(!policy.should_cache(&query)?);
  policy.on_use(&query);
  policy.on_use(&query);
  assert!(policy.should_cache(&query)?);
  Ok(())
}

#[test]
fn test_visitor() -> Result<()> {
  // TODO IMPORTANT QueryVisitor未实现
  Ok(())
}

#[test]
fn test_terms_iterator() -> Result<()> {
  let mut random = random();
  let empty = TermInSetQuery::new("field", Vec::new());
  let mut iterator = empty.get_bytes_ref_iterator()?;
  assert!(iterator.next()?.is_none());

  let query = TermInSetQuery::new(
    "field",
    vec![
      new_bytes_ref_from_string(&mut random, "term1")?,
      new_bytes_ref_from_string(&mut random, "term2")?,
      new_bytes_ref_from_string(&mut random, "term3")?,
    ],
  );
  iterator = query.get_bytes_ref_iterator()?;
  assert_eq!(
    new_bytes_ref_from_string(&mut random, "term1")?,
    iterator.next()?.unwrap().into_owned()
  );
  assert_eq!(
    new_bytes_ref_from_string(&mut random, "term2")?,
    iterator.next()?.unwrap().into_owned()
  );
  assert_eq!(
    new_bytes_ref_from_string(&mut random, "term3")?,
    iterator.next()?.unwrap().into_owned()
  );
  assert!(iterator.next()?.is_none());
  Ok(())
}
