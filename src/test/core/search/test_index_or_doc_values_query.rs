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
use crate::core::document::long_field::LongField;
use crate::core::document::long_point::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::index_or_doc_values_query::IndexOrDocValuesQuery;
use crate::core::search::query::Query;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
};
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestIndexOrDocValuesQuery;

#[test]
fn test_use_index_for_selective_queries() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);

  let writer = IndexWriter::new(dir.clone(), iwc)?;

  for i in 0..2000 {
    let mut doc = Document::new();
    if i == 42 {
      doc.add(StringField::from_string("f1", "bar", Store::No)?);
      doc.add(LongPoint::new("f2", [42i64])?);
      doc.add(NumericDocValuesField::new("f2", 42i64));
    } else if i == 100 {
      doc.add(StringField::from_string("f1", "foo", Store::No)?);
      doc.add(LongPoint::new("f2", [2i64])?);
      doc.add(NumericDocValuesField::new("f2", 2i64));
    } else {
      doc.add(StringField::from_string("f1", "bar", Store::No)?);
      doc.add(LongPoint::new("f2", [2i64])?);
      doc.add(NumericDocValuesField::new("f2", 2i64));
    }
    writer.add_document(doc)?;
  }

  writer.force_merge(1)?;
  let reader = directory_reader::open_from_writer(&writer)?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None);

  // The term query is more selective, so the IndexOrDocValuesQuery should use doc values
  let mut q1 = Builder::new();
  q1.add(TermQuery::new(Term::from_text("f1", "foo")), Occur::Must)?;
  q1.add(
    IndexOrDocValuesQuery::new(
      LongPoint::new_exact_query("f2", 2i64)?,
      NumericDocValuesField::new_slow_range_query("f2", 2i64, 2i64),
    ),
    Occur::Must,
  )?;
  let q1: Query = q1.build().into();

  QueryUtils::check_from_searcher(&mut random, q1.clone(), &searcher)?;

  let rewritten_q1 = searcher.rewrite(q1)?;
  let w1 = searcher.create_weight(rewritten_q1, ScoreMode::Complete, 1.0)?;
  let leaves = searcher.get_leaf_contexts()?;
  let s1 = w1.scorer(&leaves[0], &searcher)?.unwrap();
  assert!(s1.two_phase_iterator().is_some()); // means we use doc values

  // The term query is less selective, so the IndexOrDocValuesQuery should use points
  let mut q2 = Builder::new();
  q2.add(TermQuery::new(Term::from_text("f1", "bar")), Occur::Must)?;
  q2.add(
    IndexOrDocValuesQuery::new(
      LongPoint::new_exact_query("f2", 42i64)?,
      NumericDocValuesField::new_slow_range_query("f2", 42i64, 42i64),
    ),
    Occur::Must,
  )?;
  let q2: Query = q2.build().into();

  QueryUtils::check_from_searcher(&mut random, q2.clone(), &searcher)?;

  let rewritten_q2 = searcher.rewrite(q2)?;
  let w2 = searcher.create_weight(rewritten_q2, ScoreMode::Complete, 1.0)?;
  let s2 = w2
    .scorer(&leaves[0], &searcher)?
    .ok_or_else(|| LuceneError::illegal_state("scorer is None"))?;
  assert!(s2.two_phase_iterator().is_none()); // means we use points

  writer.close()?;
  Ok(())
}

#[test]
fn test_use_index_for_selective_multi_value_queries() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let num_docs = at_least(&mut random, 1000);
  for i in 0..num_docs {
    let mut doc = Document::new();
    if i < num_docs / 2 {
      doc.add(StringField::from_string("f1", "bar", Store::No)?);
      for _ in 0..500 {
        doc.add(LongField::new("f2", 42i64, Store::No)?);
      }
    } else if i == num_docs / 2 {
      doc.add(StringField::from_string("f1", "foo", Store::No)?);
      doc.add(LongField::new("f2", 2i64, Store::No)?);
    } else {
      doc.add(StringField::from_string("f1", "bar", Store::No)?);
      for _ in 0..100 {
        doc.add(LongField::new("f2", 2i64, Store::No)?);
      }
    }
    writer.add_document(doc)?;
  }

  writer.force_merge(1)?;
  let reader = directory_reader::open_from_writer(&writer)?;
  let mut searcher = new_searcher_with_reader(reader)?;
  searcher.set_query_cache(None);

  // The term query is less selective, so the IndexOrDocValuesQuery should use points
  let mut q1 = Builder::new();
  q1.add(TermQuery::new(Term::from_text("f1", "bar")), Occur::Must)?;
  q1.add(
    IndexOrDocValuesQuery::new(
      LongPoint::new_exact_query("f2", 2i64)?,
      SortedNumericDocValuesField::new_slow_range_query("f2", 2i64, 2i64),
    ),
    Occur::Must,
  )?;
  let q1: Query = q1.build().into();

  QueryUtils::check_from_searcher(&mut random, q1.clone(), &searcher)?;

  let rewritten_q1 = searcher.rewrite(q1)?;
  let w1 = searcher.create_weight(rewritten_q1, ScoreMode::Complete, 1.0)?;
  let leaves = searcher.get_leaf_contexts()?;
  let s1 = w1.scorer(&leaves[0], &searcher)?.unwrap();
  assert!(s1.two_phase_iterator().is_none()); // means we use points

  // The term query is less selective, so the IndexOrDocValuesQuery should use points
  let mut q2 = Builder::new();
  q2.add(TermQuery::new(Term::from_text("f1", "bar")), Occur::Must)?;
  q2.add(
    IndexOrDocValuesQuery::new(
      LongPoint::new_exact_query("f2", 42i64)?,
      SortedNumericDocValuesField::new_slow_range_query("f2", 42i64, 42i64),
    ),
    Occur::Must,
  )?;
  let q2: Query = q2.build().into();

  QueryUtils::check_from_searcher(&mut random, q2.clone(), &searcher)?;

  let rewritten_q2 = searcher.rewrite(q2)?;
  let w2 = searcher.create_weight(rewritten_q2, ScoreMode::Complete, 1.0)?;
  let s2 = w2.scorer(&leaves[0], &searcher)?.unwrap();
  assert!(s2.two_phase_iterator().is_none()); // means we use points

  // The term query is more selective, so the IndexOrDocValuesQuery should use doc values
  let mut q3 = Builder::new();
  q3.add(TermQuery::new(Term::from_text("f1", "foo")), Occur::Must)?;
  q3.add(
    IndexOrDocValuesQuery::new(
      LongPoint::new_exact_query("f2", 42i64)?,
      SortedNumericDocValuesField::new_slow_range_query("f2", 42i64, 42i64),
    ),
    Occur::Must,
  )?;
  let q3: Query = q3.build().into();

  QueryUtils::check_from_searcher(&mut random, q3.clone(), &searcher)?;

  let rewritten_q3 = searcher.rewrite(q3)?;
  let w3 = searcher.create_weight(rewritten_q3, ScoreMode::Complete, 1.0)?;
  let s3 = w3.scorer(&leaves[0], &searcher)?.unwrap();
  assert!(s3.two_phase_iterator().is_some()); // means we use doc values

  writer.close()?;
  Ok(())
}
#[test]
fn test_query_matches_count() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let num_docs = random.random_range(0..5000);
  for _ in 0..num_docs {
    let mut doc = Document::new();
    doc.add(LongPoint::new("f2", [42i64])?);
    doc.add(SortedNumericDocValuesField::new("f2", 42i64));
    writer.add_document(doc)?;
  }

  writer.force_merge(1)?;
  let reader = directory_reader::open_from_writer(&writer)?;
  let searcher = new_searcher_with_reader(reader)?;

  let query = IndexOrDocValuesQuery::new(
    LongPoint::new_exact_query("f2", 42i64)?,
    SortedNumericDocValuesField::new_slow_range_query("f2", 42i64, 42i64),
  );

  QueryUtils::check_from_searcher(&mut random, query.clone(), &searcher)?;

  let search_count = searcher.count(query.clone())?;

  let weight = searcher.create_weight(query, ScoreMode::Complete, 1.0)?;
  let leaves = searcher.get_leaf_contexts()?;

  let mut weight_count = 0;
  for leaf in leaves {
    weight_count += weight.count(leaf)?;
  }

  assert_eq!(search_count, weight_count);

  writer.close()?;
  Ok(())
}
