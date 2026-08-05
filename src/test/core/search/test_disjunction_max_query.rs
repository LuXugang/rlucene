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
use crate::core::document::text_field::TextField;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::{self, IndexSearcher};
use crate::core::search::query::{Query, QueryBase};
use crate::test_framework::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, is_night_mode, new_directory_shared, new_field,
  new_index_writer_config_with_analyzer, new_log_merge_policy, new_searcher_with_reader,
  new_text_field, random,
};

use crate::core::analysis::reader::StringReader;
use crate::core::analysis::standard::standard_analyzer::StandardAnalyzer;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::search::query_utils::QueryUtils;
pub use crate::test_framework::core::search::similarity::{TestSimilarity, new_test_similarity};
use crate::test_framework::core::util::DefaultIndexSearchLR;
use rand::{Rng, RngExt};
use std::collections::HashMap;

#[allow(dead_code)] //for quick search
struct TestDisjunctionMaxQuery;
const SCORE_COMP_THRESH: f32 = 0.0000f32;
fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchLR>
where
  R: Rng + ?Sized,
{
  let index = new_directory_shared(random)?;

  let analyzer = MockAnalyzer::new(random);
  let mut iwc = new_index_writer_config_with_analyzer(random, analyzer)?;
  let sim = new_test_similarity();
  iwc.set_similarity(sim.clone());
  iwc.set_merge_policy(new_log_merge_policy(random)?);

  let writer = RandomIndexWriter::with_config(random, index, iwc);
  let mut field_to_type = HashMap::new();
  let mut non_analyzed_type =
    FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  non_analyzed_type.set_tokenized(false)?;

  {
    let mut d1 = Document::new();
    d1.add(new_field(
      random,
      "id",
      "d1",
      &non_analyzed_type.clone(),
      &mut field_to_type,
    )?);
    d1.add(new_text_field(
      random,
      "hed",
      "elephant",
      Store::Yes,
      &mut field_to_type,
    )?);
    d1.add(new_text_field(
      random,
      "dek",
      "elephant",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(random, d1)?;
  }

  // d2
  {
    let mut d2 = Document::new();
    d2.add(new_field(
      random,
      "id",
      "d2",
      &non_analyzed_type.clone(),
      &mut field_to_type,
    )?);
    d2.add(new_text_field(
      random,
      "hed",
      "elephant",
      Store::Yes,
      &mut field_to_type,
    )?);
    d2.add(new_text_field(
      random,
      "dek",
      "albino",
      Store::Yes,
      &mut field_to_type,
    )?);
    d2.add(new_text_field(
      random,
      "dek",
      "elephant",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(random, d2)?;
  }

  // d3
  {
    let mut d3 = Document::new();
    d3.add(new_field(
      random,
      "id",
      "d3",
      &non_analyzed_type.clone(),
      &mut field_to_type,
    )?);
    d3.add(new_text_field(
      random,
      "hed",
      "albino",
      Store::Yes,
      &mut field_to_type,
    )?);
    d3.add(new_text_field(
      random,
      "hed",
      "elephant",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(random, d3)?;
  }

  // d4
  {
    let mut d4 = Document::new();
    d4.add(new_field(
      random,
      "id",
      "d4",
      &non_analyzed_type.clone(),
      &mut field_to_type,
    )?);
    d4.add(new_text_field(
      random,
      "hed",
      "albino",
      Store::Yes,
      &mut field_to_type,
    )?);
    d4.add(new_field(
      random,
      "hed",
      "elephant",
      &non_analyzed_type.clone(),
      &mut field_to_type,
    )?);
    d4.add(new_text_field(
      random,
      "dek",
      "albino",
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(random, d4)?;
  }

  writer.force_merge(random, 1)?;

  let reader = writer.get_reader(random)?;
  let r = get_only_leaf_reader(&reader)?;
  writer.close(random)?;

  let mut s = index_searcher::from_reader(r)?;
  s.set_similarity(sim);

  Ok(s)
}
#[test]
fn test_skip_to_firsttime_miss() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;
  let dq = DisjunctionMaxQuery::new(
    vec![tq("id", "d1").into(), tq("dek", "DOES_NOT_EXIST").into()],
    0.0,
  )?;

  QueryUtils::check_from_searcher(&mut random, dq.clone(), &s)?;

  let leaves = s.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let rewritten = s.rewrite(dq)?;
  let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

  let mut scorer = weight.scorer(ctx, &s)?.unwrap();

  let skip_ok = scorer.iterator_mut().advance(3)? != NO_MORE_DOCS;

  if skip_ok {
    let doc = scorer.doc_id()?;
    let stored = s.reader_context.reader().stored_fields()?.document(doc)?;
    unreachable!(
      "firsttime skipTo found a match? ... {}",
      stored.get("id")?.unwrap()
    );
  }

  Ok(())
}

#[test]
fn test_skip_to_firsttime_hit() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let dq = DisjunctionMaxQuery::new(
    vec![
      tq("dek", "albino").into(),
      tq("dek", "DOES_NOT_EXIST").into(),
    ],
    0.0,
  )?;

  QueryUtils::check_from_searcher(&mut random, dq.clone(), &s)?;

  let leaves = s.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let rewritten = s.rewrite(dq)?;
  let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

  let mut ds = weight.scorer(ctx, &s)?.unwrap();

  let hit = ds.iterator_mut().advance(3)? != NO_MORE_DOCS;
  assert!(hit, "firsttime skipTo found no match");

  let doc = ds.doc_id()?;
  let stored = s.reader_context.reader().stored_fields()?.document(doc)?;
  assert_eq!(
    "d4",
    stored.get("id")?.unwrap().as_ref(),
    "found wrong docid"
  );

  Ok(())
}
#[test]
fn test_simple_equal_scores1() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let q = DisjunctionMaxQuery::new(
    vec![tq("hed", "albino").into(), tq("hed", "elephant").into()],
    0.0,
  )?;

  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(4, h.len(), "all docs should match {}", q.to_string("")?);
  let score = h[0].score;
  for (i, item) in h.iter().enumerate().skip(1) {
    assert!(
      (score - item.score).abs() <= SCORE_COMP_THRESH,
      "score #{} is not the same",
      i
    );
  }
  Ok(())
}
#[test]
fn test_simple_equal_scores2() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let q = DisjunctionMaxQuery::new(
    vec![tq("dek", "albino").into(), tq("dek", "elephant").into()],
    0.0,
  )?;

  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(3, h.len(), "3 docs should match {}", q.to_string("")?);
  let score = h[0].score;
  for (i, item) in h.iter().enumerate().skip(1) {
    assert!(
      (score - item.score).abs() <= SCORE_COMP_THRESH,
      "score #{} is not the same",
      i
    );
  }

  Ok(())
}

#[test]
fn test_simple_equal_scores3() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let q = DisjunctionMaxQuery::new(
    vec![
      tq("hed", "albino").into(),
      tq("hed", "elephant").into(),
      tq("dek", "albino").into(),
      tq("dek", "elephant").into(),
    ],
    0.0,
  )?;

  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(4, h.len(), "all docs should match {}", q.to_string("")?);
  let score = h[0].score;
  for (i, sd) in h.iter().enumerate().skip(1) {
    assert!(
      (score - sd.score).abs() <= SCORE_COMP_THRESH,
      "score #{} is not the same",
      i
    );
  }

  Ok(())
}

#[test]
fn test_simple_tiebreaker() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let q = DisjunctionMaxQuery::new(
    vec![tq("dek", "albino").into(), tq("dek", "elephant").into()],
    0.01,
  )?;

  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(3, h.len(), "3 docs should match {}", q.to_string("")?);

  let mut stored_fields = s.stored_fields()?;
  let first_doc = stored_fields.document(h[0].doc)?;
  assert_eq!("d2", first_doc.get("id")?.unwrap().as_ref(), "wrong first");

  let score0 = h[0].score;
  let score1 = h[1].score;
  let score2 = h[2].score;

  assert!(
    score0 > score1,
    "d2 does not have better score then others: {} >? {}",
    score0,
    score1
  );

  assert!(
    (score1 - score2).abs() <= SCORE_COMP_THRESH,
    "d4 and d1 don't have equal scores"
  );

  Ok(())
}

#[test]
fn test_boolean_required_equal_scores() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let mut builder = Builder::new();

  {
    let q1 = DisjunctionMaxQuery::new(
      vec![tq("hed", "albino").into(), tq("dek", "albino").into()],
      0.0,
    )?;
    builder.add(q1.clone(), Occur::Must)?;
    QueryUtils::check_from_searcher(&mut random, q1.clone(), &s)?;
  }

  {
    let q2 = DisjunctionMaxQuery::new(
      vec![tq("hed", "elephant").into(), tq("dek", "elephant").into()],
      0.0,
    )?;
    builder.add(q2.clone(), Occur::Must)?;
    QueryUtils::check_from_searcher(&mut random, q2.clone(), &s)?;
  }

  let q = builder.build();
  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(3, h.len(), "3 docs should match {}", q.to_string("")?);

  let score = h[0].score;
  for (i, sd) in h.iter().enumerate().skip(1) {
    assert!(
      (score - sd.score).abs() <= SCORE_COMP_THRESH,
      "score #{} is not the same",
      i
    );
  }

  Ok(())
}

#[test]
fn test_boolean_optional_no_tiebreaker() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let mut builder = Builder::new();

  {
    let q1 = DisjunctionMaxQuery::new(
      vec![tq("hed", "albino").into(), tq("dek", "albino").into()],
      0.0,
    )?;
    builder.add(q1.clone(), Occur::Should)?;
  }

  {
    let q2 = DisjunctionMaxQuery::new(
      vec![tq("hed", "elephant").into(), tq("dek", "elephant").into()],
      0.0,
    )?;
    builder.add(q2.clone(), Occur::Should)?;
  }

  let q = builder.build();
  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(4, h.len(), "4 docs should match {}", q.to_string("")?);

  let score = h[0].score;
  for (i, sd) in h.iter().enumerate().skip(1).take(h.len().saturating_sub(2)) {
    assert!(
      (score - sd.score).abs() <= SCORE_COMP_THRESH,
      "score #{} is not the same",
      i
    );
  }

  let mut stored_fields = s.stored_fields()?;
  let last_doc = stored_fields.document(h[h.len() - 1].doc)?;
  assert_eq!("d1", last_doc.get("id")?.unwrap().as_ref(), "wrong last");

  let score1 = h[h.len() - 1].score;
  assert!(
    score > score1,
    "d1 does not have worse score then others: {} >? {}",
    score,
    score1
  );

  Ok(())
}

#[test]
fn test_boolean_optional_with_tiebreaker() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let mut builder = Builder::new();

  {
    let q1 = DisjunctionMaxQuery::new(
      vec![tq("hed", "albino").into(), tq("dek", "albino").into()],
      0.01,
    )?;
    builder.add(q1, Occur::Should)?;
  }

  {
    let q2 = DisjunctionMaxQuery::new(
      vec![tq("hed", "elephant").into(), tq("dek", "elephant").into()],
      0.01,
    )?;
    builder.add(q2, Occur::Should)?;
  }

  let q = builder.build();
  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(4, h.len(), "4 docs should match {}", q.to_string("")?);

  let score0 = h[0].score;
  let score1 = h[1].score;
  let score2 = h[2].score;
  let score3 = h[3].score;

  let mut stored_fields = s.stored_fields()?;
  let doc0 = stored_fields
    .document(h[0].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();
  let doc1 = stored_fields
    .document(h[1].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();
  let doc2 = stored_fields
    .document(h[2].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();
  let doc3 = stored_fields
    .document(h[3].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();

  assert!(
    doc0 == "d2" || doc0 == "d4",
    "doc0 should be d2 or d4: {}",
    doc0
  );
  assert!(
    doc1 == "d2" || doc1 == "d4",
    "doc1 should be d2 or d4: {}",
    doc1
  );

  assert!(
    (score0 - score1).abs() <= SCORE_COMP_THRESH,
    "score0 and score1 should match"
  );

  assert_eq!("d3", doc2, "wrong third");
  assert!(
    score1 > score2,
    "d3 does not have worse score then d2 and d4: {} >? {}",
    score1,
    score2
  );

  assert_eq!("d1", doc3, "wrong fourth");
  assert!(
    score2 > score3,
    "d1 does not have worse score then d3: {} >? {}",
    score2,
    score3
  );

  Ok(())
}

#[test]
fn test_boolean_optional_with_tiebreaker_and_boost() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let mut builder = Builder::new();

  {
    let q1 = DisjunctionMaxQuery::new(
      vec![
        tq_with_boost("hed", "albino", 1.5)?.into(),
        tq("dek", "albino").into(),
      ],
      0.01,
    )?;
    builder.add(q1, Occur::Should)?;
  }

  {
    let q2 = DisjunctionMaxQuery::new(
      vec![
        tq_with_boost("hed", "elephant", 1.5)?.into(),
        tq("dek", "elephant").into(),
      ],
      0.01,
    )?;
    builder.add(q2, Occur::Should)?;
  }

  let q = builder.build();
  QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

  let h = s.search(q.clone(), 1000)?.score_docs;

  assert_eq!(4, h.len(), "4 docs should match {}", q.to_string("")?);

  let score0 = h[0].score;
  let score1 = h[1].score;
  let score2 = h[2].score;
  let score3 = h[3].score;

  let mut stored_fields = s.stored_fields()?;
  let doc0 = stored_fields
    .document(h[0].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();
  let doc1 = stored_fields
    .document(h[1].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();
  let doc2 = stored_fields
    .document(h[2].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();
  let doc3 = stored_fields
    .document(h[3].doc)?
    .get("id")?
    .unwrap()
    .as_ref()
    .to_string();

  assert_eq!("d4", doc0, "doc0 should be d4:");
  assert_eq!("d3", doc1, "doc1 should be d3:");
  assert_eq!("d2", doc2, "doc2 should be d2:");
  assert_eq!("d1", doc3, "doc3 should be d1:");

  assert!(
    score0 > score1,
    "d4 does not have a better score then d3: {} >? {}",
    score0,
    score1
  );
  assert!(
    score1 > score2,
    "d3 does not have a better score then d2: {} >? {}",
    score1,
    score2
  );
  assert!(
    score2 > score3,
    "d3 does not have a better score then d1: {} >? {}",
    score2,
    score3
  );

  Ok(())
}

#[test]
fn test_rewrite_boolean() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let sub1: Query = tq("hed", "albino").into();
  let sub2: Query = tq("hed", "elephant").into();

  let q = DisjunctionMaxQuery::new(vec![sub1.clone(), sub2.clone()], 1.0)?;

  let rewritten = s.rewrite(q.clone())?;

  let mut builder = Builder::new();
  builder.add(sub1, Occur::Should)?;
  builder.add(sub2, Occur::Should)?;
  let expected: Query = builder.build().into();

  assert_eq!(expected, rewritten);

  Ok(())
}

#[test]
fn test_rewrite_empty() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let q = DisjunctionMaxQuery::new(vec![], 0.0)?;
  let rewritten = s.rewrite(q)?;

  let expected: Query = MatchNoDocsQuery::new().into();

  assert_eq!(expected, rewritten);

  Ok(())
}

#[test]
fn test_disjunct_order_and_equals() -> Result<()> {
  let mut random = random();
  let _s = set_up(&mut random)?;

  let sub1: Query = tq("hed", "albino").into();
  let sub2: Query = tq("hed", "elephant").into();

  let q1: Query = DisjunctionMaxQuery::new(vec![sub1.clone(), sub2.clone()], 1.0)?.into();
  let q2: Query = DisjunctionMaxQuery::new(vec![sub2, sub1], 1.0)?.into();

  assert_eq!(q1, q2);

  Ok(())
}

#[test]
fn test_to_string_order_matters() -> Result<()> {
  let mut random = random();
  let _s = set_up(&mut random)?;

  let clause_nbr = random.random_range(4..=25);

  let mut terms = Vec::with_capacity(clause_nbr);
  for i in 0..clause_nbr {
    terms.push(((b'a' + i as u8) as char).to_string());
  }

  let expected = terms
    .iter()
    .map(|term| format!("test:{}", term))
    .collect::<Vec<_>>()
    .join(" | ");
  let expected = format!("({})~1.0", expected);

  let disjuncts: Vec<Query> = terms.iter().map(|term| tq("test", term).into()).collect();

  let source = DisjunctionMaxQuery::new(disjuncts, 1.0)?;

  assert_eq!(expected, source.to_string("")?);

  Ok(())
}
#[test]
fn test_random_top_docs() -> Result<()> {
  let mut random = random();
  let _s = set_up(&mut random)?;
  do_test_random_top_docs(&mut random, 2, &[0.05, 0.05])?;
  do_test_random_top_docs(&mut random, 2, &[1.0, 0.05])?;
  do_test_random_top_docs(&mut random, 3, &[1.0, 0.5, 0.05])?;
  do_test_random_top_docs(&mut random, 4, &[1.0, 0.5, 0.05, 0.0])?;
  do_test_random_top_docs(&mut random, 4, &[1.0, 0.5, 0.05, 0.0])?;
  Ok(())
}
#[test]
fn test_explain_match() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let sub1: Query = tq("hed", "elephant").into();
  let sub2: Query = tq("dek", "elephant").into();

  let dq = DisjunctionMaxQuery::new(vec![sub1, sub2], 0.0)?;

  let rewritten = s.rewrite(dq)?;
  let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

  let leaves = s.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let explanation = weight.explain(ctx, 1, &s)?;

  assert_eq!("max of:", explanation.get_description());
  assert_eq!(2, explanation.get_details().len());

  Ok(())
}
#[test]
fn test_explain_no_match() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let sub1: Query = tq("abc", "elephant").into();
  let sub2: Query = tq("def", "elephant").into();

  let dq = DisjunctionMaxQuery::new(vec![sub1, sub2], 0.0)?;

  let rewritten = s.rewrite(dq)?;
  let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

  let leaves = s.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let explanation = weight.explain(ctx, 1, &s)?;

  assert_eq!("No matching clause", explanation.get_description());
  assert_eq!(2, explanation.get_details().len());

  Ok(())
}

#[test]
fn test_explain_match_one_non_matching_subquery_not_included_in_explanation() -> Result<()> {
  let mut random = random();
  let s = set_up(&mut random)?;

  let sub1: Query = tq("hed", "elephant").into();
  let sub2: Query = tq("def", "elephant").into();

  let dq = DisjunctionMaxQuery::new(vec![sub1, sub2], 0.0)?;

  let rewritten = s.rewrite(dq)?;
  let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

  let leaves = s.get_top_reader_context().leaves()?;
  let ctx = &leaves[0];

  let explanation = weight.explain(ctx, 1, &s)?;

  assert_eq!("max of:", explanation.get_description());
  assert_eq!(1, explanation.get_details().len());

  Ok(())
}
fn do_test_random_top_docs<R>(random: &mut R, num_fields: usize, freqs: &[f64]) -> Result<()>
where
  R: Rng + ?Sized,
{
  assert_eq!(num_fields, freqs.len());

  let dir = new_directory_shared(random)?;
  let analyzer = StandardAnalyzer::new();
  let iwc = IndexWriterConfig::with_analyzer(analyzer)?;
  let writer = IndexWriter::new(dir.clone(), iwc)?;

  let num_docs = if is_night_mode() {
    at_least(random, 1000)
  } else {
    at_least(random, 100)
  };

  for _ in 0..num_docs {
    let mut doc = Document::new();

    for (j, freq) in freqs.iter().take(num_fields).enumerate() {
      let mut builder = String::new();

      let num_as = if random.random::<f64>() < *freq {
        0
      } else {
        1 + random.random_range(0..5)
      };

      for _ in 0..num_as {
        if !builder.is_empty() {
          builder.push(' ');
        }
        builder.push('a');
      }

      if random.random_bool(0.5) {
        doc.add(StringField::from_string("field", "c", Store::No)?);
      }

      let num_others = if random.random_bool(0.5) {
        0
      } else {
        1 + random.random_range(0..5)
      };

      for _ in 0..num_others {
        if !builder.is_empty() {
          builder.push(' ');
        }
        builder.push_str(&random.random::<i32>().to_string());
      }
      doc.add(TextField::from_reader(
        j.to_string(),
        StringReader::new(builder),
      )?);
    }

    writer.add_document(doc)?;
  }

  let reader = directory_reader::open_from_writer(&writer)?;
  writer.close()?;

  let searcher = new_searcher_with_reader(reader)?;

  for i in 0..4 {
    let mut clauses: Vec<Query> = Vec::new();

    for j in 0..num_fields {
      if i % 2 == 1 {
        clauses.push(tq(&j.to_string(), "a").into());
      } else {
        let boost = if random.random_bool(0.5) {
          0.0
        } else {
          random.random::<f32>()
        };

        if boost > 0.0 {
          clauses.push(tq_with_boost(&j.to_string(), "a", boost)?.into());
        } else {
          clauses.push(tq(&j.to_string(), "a").into());
        }
      }
    }

    let tie_breaker = random.random::<f32>();
    let query: Query = DisjunctionMaxQuery::new(clauses.clone(), tie_breaker)?.into();

    CheckHits::check_top_scores(random, &query, &searcher)?;

    let mut builder = Builder::new();
    builder.add(DisjunctionMaxQuery::new(clauses, tie_breaker)?, Occur::Must)?;
    builder.add(tq("field", "c"), Occur::Filter)?;

    let query: Query = builder.build().into();

    CheckHits::check_top_scores(random, &query, &searcher)?;
  }
  Ok(())
}
fn tq(field: &str, term: &str) -> TermQuery {
  TermQuery::new(Term::from_text(field, term))
}

fn tq_with_boost(field: &str, term: &str, boost: f32) -> Result<BoostQuery> {
  let q = tq(field, term);
  BoostQuery::new(q, boost)
}
