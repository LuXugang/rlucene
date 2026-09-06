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
use crate::core::document::field::Store::No;
use crate::core::document::field_type::FieldType;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_searcher_with_reader, new_text_field, random,
};

use crate::core::document::field::FieldBase;
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::multi_reader::MultiReader;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query;
use crate::core::search::boolean_query::Builder;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::{self, IndexSearcher, IndexSearcherHook};
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocs;
use crate::core::util::HasIdentity;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
pub use crate::test_framework::core::search::query::TestRewriteQuery;
use crate::test_framework::core::search::test_boolean_rewrites::NoRewriteIndexSearcher;
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(dead_code)]
struct TestBooleanRewrites;
#[test]
fn test_one_clause_rewrite_optimization() -> Result<()> {
  let mut random = random();
  let field = "content";
  let value = "foo";

  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;
  writer.close(&mut random)?;

  let reader = directory_reader::open(dir)?;
  let searcher = new_searcher_with_reader(reader)?;

  let expected: Query = TermQuery::new(Term::from_text(field, value)).into();

  let num_layers = at_least(&mut random, 3);
  let mut actual: Query = TermQuery::new(Term::from_text(field, value)).into();

  for _ in 0..num_layers {
    let mut bq = Builder::new();
    let occur = if random.random_bool(0.5) {
      Occur::Should
    } else {
      Occur::Must
    };
    bq.add(actual, occur)?;
    actual = bq.build().into();
  }

  let rewritten = searcher.rewrite(actual)?;
  assert_eq!(expected, rewritten);

  Ok(())
}
#[test]
fn test_single_filter_clause() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), IndexWriterConfig::new()?)?;

  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut doc = Document::new();
  doc.add(new_text_field(
    &mut random,
    "field",
    "a",
    No,
    &mut field_to_type,
  )?);
  writer.add_document(doc)?;
  writer.commit()?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let searcher = index_searcher::from_reader(reader)?;

  let mut query1 = Builder::new();
  query1.add(TermQuery::new(Term::from_text("field", "a")), Occur::Filter)?;
  let rewritten1 = query1.build().rewrite(&searcher)?;
  match rewritten1 {
    Some(Query::Boost(bq)) => {
      assert_eq!(0.0, bq.get_boost());
    },
    _ => return Err(LuceneError::illegal_state("expected BoostQuery")),
  }
  // When there are two clauses, we cannot rewrite, but if one of them creates
  // an absent scorer we will end up with a single filter scorer and will need to
  // make sure to set score=0
  let mut query2 = Builder::new();
  query2.add(TermQuery::new(Term::from_text("field", "a")), Occur::Filter)?;
  query2.add(
    TermQuery::new(Term::from_text("missing_field", "b")),
    Occur::Should,
  )?;

  let rewritten2 = searcher.rewrite(query2.build())?;
  let weight = searcher.create_weight(rewritten2, ScoreMode::Complete, 1.0)?;
  let leaf = &searcher.get_leaf_contexts()?[0];
  let mut scorer = weight
    .scorer(leaf, &searcher)?
    .ok_or_else(|| LuceneError::illegal_state("null scorer"))?;

  assert_eq!(0, scorer.iterator_mut().next_doc()?);
  assert_eq!(0.0, scorer.score()?);

  Ok(())
}
#[test]
fn test_single_must_match_all() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
  let bq = bq.build();

  assert_eq!(
    Query::ConstantScore(ConstantScoreQuery::new(TermQuery::new(Term::from_text(
      "foo", "bar"
    )))),
    searcher.rewrite(bq)?
  );

  let mut bq = Builder::new();
  bq.add(
    BoostQuery::new(Query::MatchAllDocs(MatchAllDocsQuery::new()), 42.0)?,
    Occur::Must,
  )?
  .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
  let bq = bq.build();

  let v: Query = BoostQuery::new(
    Query::ConstantScore(ConstantScoreQuery::new(TermQuery::new(Term::from_text(
      "foo", "bar",
    )))),
    42.0,
  )?
  .into();
  assert_eq!(v, searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();

  assert_eq!(
    Query::MatchAllDocs(MatchAllDocsQuery::new()),
    searcher.rewrite(bq)?
  );

  let mut bq = Builder::new();
  bq.add(
    BoostQuery::new(Query::MatchAllDocs(MatchAllDocsQuery::new()), 42.0)?,
    Occur::Must,
  )?
  .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();
  let v: Query = BoostQuery::new(Query::MatchAllDocs(MatchAllDocsQuery::new()), 42.0)?.into();
  assert_eq!(v, searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "bar")),
      Occur::MustNot,
    )?;
  let bq = bq.build();
  let v: Query = bq.clone().into();
  assert_eq!(v, searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();

  assert_eq!(
    Query::MatchAllDocs(MatchAllDocsQuery::new()),
    searcher.rewrite(bq)?
  );

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let bq = bq.build();

  let mut expected = Builder::new();
  expected
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let expected = expected.build();
  let v: Query = ConstantScoreQuery::new(expected).into();
  assert_eq!(v, searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(
      TermQuery::new(Term::from_text("foo", "baz")),
      Occur::MustNot,
    )?;
  let bq = bq.build();

  let mut expected = Builder::new();
  expected
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(
      TermQuery::new(Term::from_text("foo", "baz")),
      Occur::MustNot,
    )?;
  let expected = expected.build();
  let v: Query = ConstantScoreQuery::new(expected).into();
  assert_eq!(v, searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  let bq: Query = bq.build().into();

  assert_eq!(bq.clone(), searcher.rewrite(bq)?);

  Ok(())
}
#[test]
fn test_single_must_match_all_with_should_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let bq = bq.build();

  let mut expected = Builder::new();
  expected
    .add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "bar"))),
      Occur::Must,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let expected = expected.build();

  assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

  Ok(())
}
#[test]
fn test_deduplicate_must_and_filter() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
  let bq = bq.build();

  assert_eq!(
    Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
    searcher.rewrite(bq)?
  );

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let bq = bq.build();

  let mut expected = Builder::new();
  expected
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let expected = expected.build();

  assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

  Ok(())
}
#[test]
fn test_convert_should_and_filter_to_must() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?;
  let bq = bq.build();

  assert_eq!(
    Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
    searcher.rewrite(bq)?
  );

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "quz")), Occur::Should)?
    .set_minimum_number_should_match(2);
  let bq = bq.build();

  let mut expected = Builder::new();
  expected
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "quz")), Occur::Should)?
    .set_minimum_number_should_match(1);
  let expected = expected.build();

  assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

  Ok(())
}

#[test]
fn test_duplicate_must_or_filter_with_must_not() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "bar")),
      Occur::MustNot,
    )?;
  let bq = bq.build();

  assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq)?);

  let mut bq2 = Builder::new();
  bq2
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "bar")),
      Occur::MustNot,
    )?;
  let bq2 = bq2.build();

  assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq2)?);

  Ok(())
}

#[test]
fn test_match_all_must_not() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
    .add(
      Query::MatchAllDocs(MatchAllDocsQuery::new()),
      Occur::MustNot,
    )?;
  let bq = bq.build();

  assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq)?);

  let mut bq2 = Builder::new();
  bq2
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "bad")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "bor")),
      Occur::MustNot,
    )?
    .add(
      Query::MatchAllDocs(MatchAllDocsQuery::new()),
      Occur::MustNot,
    )?;
  let bq2 = bq2.build();

  assert_eq!(Query::from(MatchNoDocsQuery::new()), searcher.rewrite(bq2)?);

  Ok(())
}

#[test]
fn test_deeply_nested_boolean_rewrite_should_clauses() -> Result<()> {
  let mut random = random();

  // Java: newSearcher(new MultiReader())
  let reader = MultiReader::empty()?;
  let searcher = new_searcher_with_reader(reader)?;

  let depth = random.random_range(10..=30);

  let expected_rc = Arc::new(AtomicUsize::new(0));
  let rewrite_query_expected = TestRewriteQuery::new(expected_rc.clone());

  let rc_ = Arc::new(AtomicUsize::new(0));
  let rewrite_query = TestRewriteQuery::new(rc_.clone());

  let mut expected_query_builder = Builder::new();
  expected_query_builder.add(rewrite_query_expected, Occur::Filter)?;

  let mut deep_builder = {
    let mut b = Builder::new();
    b.add(rewrite_query.clone(), Occur::Should)?;
    b.set_minimum_number_should_match(1);
    b.build()
  };

  for i in (1..=depth).rev() {
    let tq = TermQuery::new(Term::from_text(format!("layer[{}]", i), "foo"));

    let mut bq = Builder::new();
    bq.set_minimum_number_should_match(2);
    bq.add(tq.clone(), Occur::Should)?;
    bq.add(deep_builder, Occur::Should)?;
    deep_builder = bq.build();

    expected_query_builder.add(tq, Occur::Filter)?;
    if i == depth {
      expected_query_builder.add(rewrite_query.clone(), Occur::Filter)?;
    }
  }

  let bq = {
    let mut b = Builder::new();
    b.add(deep_builder, Occur::Filter)?;
    b.build()
  };

  let expected_query: Query =
    BoostQuery::new(ConstantScoreQuery::new(expected_query_builder.build()), 0.0)?.into();

  let rewritten = searcher.rewrite(bq)?;
  assert_eq!(expected_query, rewritten);

  // the SHOULD clauses cause more rewrites because they incrementally change to `MUST` and then
  // `FILTER`, plus the flattening of required clauses
  assert_eq!(depth as usize * 2, rc_.load(Ordering::Relaxed));

  Ok(())
}
#[test]
fn test_deeply_nested_boolean_rewrite() -> Result<()> {
  let mut random = random();

  // Java: newSearcher(new MultiReader())
  let reader = MultiReader::empty()?;
  let searcher = new_searcher_with_reader(reader)?;
  let depth = random.random_range(10..=30);
  let expected_rc = Arc::new(AtomicUsize::new(0));
  let rewrite_query_expected = TestRewriteQuery::new(expected_rc.clone());

  let rc_ = Arc::new(AtomicUsize::new(0));
  let rewrite_query = TestRewriteQuery::new(rc_.clone());

  let mut expected_query_builder = Builder::new();
  expected_query_builder.add(rewrite_query_expected, Occur::Filter)?;

  let mut deep_builder = {
    let mut b = Builder::new();
    b.add(rewrite_query.clone(), Occur::Must)?;
    b.build()
  };

  for i in (1..=depth).rev() {
    let tq = TermQuery::new(Term::from_text(format!("layer[{}]", i), "foo"));

    let mut bq = Builder::new();
    bq.add(tq.clone(), Occur::Must)?;
    bq.add(deep_builder, Occur::Must)?;
    deep_builder = bq.build();

    expected_query_builder.add(tq, Occur::Filter)?;
    if i == depth {
      expected_query_builder.add(rewrite_query.clone(), Occur::Filter)?;
    }
  }

  let bq = {
    let mut b = Builder::new();
    b.add(deep_builder, Occur::Filter)?;
    b.build()
  };

  let expected_query: Query =
    BoostQuery::new(ConstantScoreQuery::new(expected_query_builder.build()), 0.0)?.into();

  let rewritten = searcher.rewrite(bq)?;
  assert_eq!(expected_query, rewritten);

  // `depth` rewrites because of the flattening
  assert_eq!(depth as usize, rc_.load(Ordering::Relaxed));

  Ok(())
}
#[test]
fn test_remove_match_all_filter() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();

  assert_eq!(
    Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
    searcher.rewrite(bq)?
  );

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?
    .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();

  let mut expected = Builder::new();
  expected
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let expected = expected.build();

  assert_eq!(Query::from(expected), searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();

  let expected: Query = BoostQuery::new(
    Query::ConstantScore(ConstantScoreQuery::new(TermQuery::new(Term::from_text(
      "foo", "bar",
    )))),
    0.0,
  )?
  .into();

  assert_eq!(expected, searcher.rewrite(bq)?);

  let mut bq = Builder::new();
  bq.add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?
    .add(Query::MatchAllDocs(MatchAllDocsQuery::new()), Occur::Filter)?;
  let bq = bq.build();

  let expected: Query = BoostQuery::new(
    Query::ConstantScore(ConstantScoreQuery::new(Query::MatchAllDocs(
      MatchAllDocsQuery::new(),
    ))),
    0.0,
  )?
  .into();

  assert_eq!(expected, searcher.rewrite(bq)?);

  Ok(())
}

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  let mut f = TextField::from_string("body", "a b c", No)?;
  doc.add(f.clone());
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  f.set_string_value("")?;
  doc.add(f.clone());
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  f.set_string_value("a b")?;
  doc.add(f.clone());
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  f.set_string_value("b c")?;
  doc.add(f.clone());
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  f.set_string_value("a")?;
  doc.add(f.clone());
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  f.set_string_value("c")?;
  doc.add(f.clone());
  writer.add_document(&mut random, doc)?;

  let num_random_docs = at_least(&mut random, 3);
  for _ in 0..num_random_docs {
    let num_terms = random.random_range(0..20);
    let mut text = String::new();
    for _ in 0..num_terms {
      text.push(char::from(b'a' + random.random_range(0..4)));
      text.push(' ');
    }
    doc = Document::new();
    f.set_string_value(text)?;
    doc.add(f.clone());
    writer.add_document(&mut random, doc)?;
  }

  let reader = Arc::new(writer.get_reader(&mut random)?);
  writer.close(&mut random)?;

  let searcher1 = new_searcher_with_reader(reader.clone())?;
  let mut searcher2 = index_searcher::from_reader(reader.clone())?
    .with_hook(IndexSearcherHook::NoRewrite(NoRewriteIndexSearcher));
  searcher2.set_similarity(searcher1.get_similarity());

  let iters = at_least(&mut random, 1000);
  for _iter in 0..iters {
    let query = random_boolean_query(&mut random)?;
    let td1 = searcher1.search(query.clone(), 100)?;
    let td2 = searcher2.search(query.clone(), 100)?;
    let result = catch_unwind(AssertUnwindSafe(|| assert_equals(&td1, &td2)));
    if let Err(payload) = result {
      println!("{}", query.to_string("")?);
      let mut query = query;
      let mut rewritten = query.clone();
      loop {
        query = rewritten;
        rewritten = query.rewrite(&searcher1)?.unwrap_or_else(|| query.clone());
        println!("{}", rewritten.to_string("")?);
        let tdx = searcher2.search(rewritten.clone(), 100)?;
        if td2.total_hits.value() != tdx.total_hits.value() {
          println!("Bad");
        }
        if query.identity() == rewritten.identity() {
          break;
        }
      }
      resume_unwind(payload);
    }
  }

  reader.close()?;
  dir.close()
}

fn random_boolean_query<R>(random: &mut R) -> Result<Query>
where
  R: Rng + ?Sized,
{
  let num_clauses = random.random_range(0..5);
  let mut b = boolean_query::Builder::new();
  let mut num_shoulds = 0;

  for _ in 0..num_clauses {
    let occur = match random.random_range(0..4) {
      0 => Occur::Must,
      1 => Occur::Filter,
      2 => Occur::Should,
      3 => Occur::MustNot,
      _ => unreachable!(),
    };

    if occur == Occur::Should {
      num_shoulds += 1;
    }

    let query = random_query(random)?;
    b.add(query, occur)?;
  }

  b.set_minimum_number_should_match(if random.random_bool(0.5) {
    0
  } else {
    TestUtil::next_int(random, 0, num_shoulds + 1)
  });

  let mut query: Query = b.build().into();

  if random.random_bool(0.5) {
    query = random_wrapper(random, query)?;
  }

  Ok(query)
}
fn random_wrapper<R>(random: &mut R, query: Query) -> Result<Query>
where
  R: Rng + ?Sized,
{
  match random.random_range(0..2) {
    0 => Ok(BoostQuery::new(query, TestUtil::next_int(random, 0, 4) as f32)?.into()),
    1 => Ok(ConstantScoreQuery::new(query).into()),
    _ => unreachable!(""),
  }
}
fn random_query<R>(random: &mut R) -> Result<Query>
where
  R: Rng + ?Sized,
{
  if random.random_range(0..5) == 0 {
    let query = random_query(random)?;
    return random_wrapper(random, query);
  }
  let v = random.random_range(0..6);
  match v {
    0 => Ok(MatchAllDocsQuery::new().into()),
    1 => Ok(TermQuery::new(Term::from_text("body", "a")).into()),
    2 => Ok(TermQuery::new(Term::from_text("body", "b")).into()),
    3 => Ok(TermQuery::new(Term::from_text("body", "c")).into()),
    4 => Ok(TermQuery::new(Term::from_text("body", "d")).into()),
    5 => random_boolean_query(random),
    _ => unreachable!(),
  }
}

fn assert_equals(td1: &TopDocs<ScoreDoc>, td2: &TopDocs<ScoreDoc>) {
  assert_eq!(td1.total_hits.value(), td2.total_hits.value(),);
  assert_eq!(td1.score_docs.len(), td2.score_docs.len(),);

  let expected_scores: HashMap<i32, f32> = td1
    .score_docs
    .iter()
    .map(|score_doc| (score_doc.doc, score_doc.score))
    .collect();
  let actual_result_set: HashSet<i32> = td2
    .score_docs
    .iter()
    .map(|score_doc| score_doc.doc)
    .collect();

  assert_eq!(
    expected_scores.keys().copied().collect::<HashSet<_>>(),
    actual_result_set,
  );

  for score_doc in &td2.score_docs {
    let expected_score = expected_scores[&score_doc.doc];
    let actual_score = score_doc.score;
    let tolerance = expected_score / 100.0;
    assert!((expected_score - actual_score).abs() <= tolerance,);
  }
}
#[test]
fn test_deduplicate_should_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?;
  let query: Query = query.build().into();

  let expected: Query = BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?.into();
  assert_eq!(expected, searcher.rewrite(query.clone())?);

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?,
      Occur::Should,
    )?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let query: Query = query.build().into();

  let mut expected = Builder::new();
  expected
    .add(
      BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 3.0)?,
      Occur::Should,
    )?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let expected: Query = expected.build().into();

  assert_eq!(expected, searcher.rewrite(query.clone())?);

  let mut query = Builder::new();
  query
    .set_minimum_number_should_match(2)
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let query: Query = query.build().into();

  let expected = query.clone();
  assert_eq!(expected, searcher.rewrite(query)?);

  Ok(())
}

#[test]
fn test_deduplicate_must_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?;
  let query: Query = query.build().into();

  let expected: Query = BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?.into();
  assert_eq!(expected, searcher.rewrite(query.clone())?);

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 2.0)?,
      Occur::Must,
    )?
    .add(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?;
  let query: Query = query.build().into();

  let mut expected = Builder::new();
  expected
    .add(
      BoostQuery::new(TermQuery::new(Term::from_text("foo", "bar")), 3.0)?,
      Occur::Must,
    )?
    .add(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?;
  let expected: Query = expected.build().into();

  assert_eq!(expected, searcher.rewrite(query)?);

  Ok(())
}
#[test]
fn test_flatten_inner_disjunctions() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner.clone(), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut query = Builder::new();
  query
    .set_minimum_number_should_match(0)
    .add(inner.clone(), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .set_minimum_number_should_match(0)
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut query = Builder::new();
  query
    .set_minimum_number_should_match(1)
    .add(inner.clone(), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .set_minimum_number_should_match(1)
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut query = Builder::new();
  query
    .set_minimum_number_should_match(2)
    .add(inner.clone(), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let query: Query = query.build().into();

  assert_eq!(
    Query::from(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?
    .set_minimum_number_should_match(2);
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner, Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let query: Query = query.build().into();
  let query_id = query.identity().clone();
  let v = searcher.rewrite(query)?;
  assert_eq!(query_id, v.identity().clone());

  Ok(())
}
#[test]
fn test_flatten_inner_conjunctions() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?;
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner.clone(), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut query = Builder::new();
  query
    .set_minimum_number_should_match(0)
    .add(inner.clone(), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .set_minimum_number_should_match(0)
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut query = Builder::new();
  query.add(inner.clone(), Occur::Must)?.add(
    TermQuery::new(Term::from_text("foo", "baz")),
    Occur::MustNot,
  )?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "quux")), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "baz")),
      Occur::MustNot,
    )?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Filter,
    )?;
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner.clone(), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Filter,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Filter,
    )?;
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner.clone(), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Filter,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::MustNot,
    )?;
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner, Occur::Filter)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::MustNot,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Must)?;
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  Ok(())
}
#[test]
fn test_flatten_disjunction_in_must_clause() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?;
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner, Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
    .set_minimum_number_should_match(1);
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "foo")), Occur::Should)?
    .set_minimum_number_should_match(2);
  let inner: Query = inner.build().into();

  let mut query = Builder::new();
  query
    .add(inner, Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let query: Query = query.build().into();

  let mut expected_rewritten = Builder::new();
  expected_rewritten
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("foo", "quux")),
      Occur::Should,
    )?
    .add(TermQuery::new(Term::from_text("foo", "foo")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?
    .set_minimum_number_should_match(2);
  let expected_rewritten: Query = expected_rewritten.build().into();

  assert_eq!(expected_rewritten, searcher.rewrite(query)?);

  Ok(())
}

#[test]
fn test_discard_should_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("field", "a")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?;
  let inner = inner.build();

  let query1: Query = ConstantScoreQuery::new(inner).into();
  let rewritten1: Query =
    ConstantScoreQuery::new(TermQuery::new(Term::from_text("field", "a"))).into();
  assert_eq!(rewritten1, searcher.rewrite(query1)?);

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("field", "a")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("field", "c")), Occur::Filter)?;
  let inner = inner.build();

  let query2: Query = ConstantScoreQuery::new(inner).into();

  let mut rewritten2_inner = Builder::new();
  rewritten2_inner
    .add(TermQuery::new(Term::from_text("field", "a")), Occur::Filter)?
    .add(TermQuery::new(Term::from_text("field", "c")), Occur::Filter)?;
  let rewritten2_inner = rewritten2_inner.build();
  let rewritten2: Query = ConstantScoreQuery::new(rewritten2_inner).into();

  assert_eq!(rewritten2, searcher.rewrite(query2)?);

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?;
  let inner = inner.build();

  let query3: Query = ConstantScoreQuery::new(inner).into();
  let query3_id = query3.identity().clone();
  let v = searcher.rewrite(query3)?;
  assert_eq!(query3_id, v.identity().clone());

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?
    .add(
      TermQuery::new(Term::from_text("field", "b")),
      Occur::MustNot,
    )?;
  let inner = inner.build();

  let query4: Query = ConstantScoreQuery::new(inner).into();
  let query4_id = query4.identity().clone();
  let v = searcher.rewrite(query4)?;
  assert_eq!(query4_id, v.identity().clone());

  let mut inner = Builder::new();
  inner
    .set_minimum_number_should_match(1)
    .add(TermQuery::new(Term::from_text("field", "a")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("field", "b")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("field", "c")), Occur::Filter)?;
  let inner = inner.build();

  let query5: Query = ConstantScoreQuery::new(inner).into();
  let query5_id = query5.identity().clone();
  let v = searcher.rewrite(query5)?;
  assert_eq!(query5_id, v.identity().clone());

  Ok(())
}

#[test]
fn test_should_match_no_docs_query() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(Query::MatchNoDocs(MatchNoDocsQuery::new()), Occur::Should)?;
  let query = query.build();

  assert_eq!(
    Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
    searcher.rewrite(query)?
  );

  Ok(())
}
#[test]
fn test_must_not_match_no_docs_query() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(Query::MatchNoDocs(MatchNoDocsQuery::new()), Occur::MustNot)?;
  let query = query.build();

  assert_eq!(
    Query::from(TermQuery::new(Term::from_text("foo", "bar"))),
    searcher.rewrite(query)?
  );

  Ok(())
}

#[test]
fn test_must_match_no_docs_query() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(Query::MatchNoDocs(MatchNoDocsQuery::new()), Occur::Must)?;
  let query = query.build();

  assert_eq!(
    Query::from(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  Ok(())
}

#[test]
fn test_filter_match_no_docs_query() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(Query::MatchNoDocs(MatchNoDocsQuery::new()), Occur::Filter)?;
  let query = query.build();

  assert_eq!(
    Query::from(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  Ok(())
}

#[test]
fn test_empty_boolean() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let query: Query = Builder::new().build().into();

  assert_eq!(
    Query::from(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  Ok(())
}

#[test]
fn test_simplify_filter_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query1 = Builder::new();
  query1
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "baz"))),
      Occur::Filter,
    )?;
  let query1 = query1.build();

  let mut expected1 = Builder::new();
  expected1
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Filter)?;
  let expected1: Query = expected1.build().into();

  assert_eq!(expected1, searcher.rewrite(query1)?);

  let mut query2 = Builder::new();
  query2
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Filter)?
    .add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "bar"))),
      Occur::Filter,
    )?;
  let query2 = query2.build();

  let expected2: Query = BoostQuery::new(
    Query::ConstantScore(ConstantScoreQuery::new(TermQuery::new(Term::from_text(
      "foo", "bar",
    )))),
    0.0,
  )?
  .into();

  assert_eq!(expected2, searcher.rewrite(query2)?);

  Ok(())
}
#[test]
fn test_simplify_must_not_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut query = Builder::new();
  query
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "baz"))),
      Occur::MustNot,
    )?;
  let query = query.build();

  let mut expected = Builder::new();
  expected
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Must)?
    .add(
      TermQuery::new(Term::from_text("foo", "baz")),
      Occur::MustNot,
    )?;
  let expected: Query = expected.build().into();

  assert_eq!(expected, searcher.rewrite(query)?);

  Ok(())
}

#[test]
fn test_simplify_non_scoring_should_clauses() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  let mut inner = Builder::new();
  inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(
      ConstantScoreQuery::new(TermQuery::new(Term::from_text("foo", "baz"))),
      Occur::Should,
    )?;
  let inner = inner.build();

  let query: Query = ConstantScoreQuery::new(inner).into();

  let mut expected_inner = Builder::new();
  expected_inner
    .add(TermQuery::new(Term::from_text("foo", "bar")), Occur::Should)?
    .add(TermQuery::new(Term::from_text("foo", "baz")), Occur::Should)?;
  let expected_inner = expected_inner.build();

  let expected: Query = ConstantScoreQuery::new(expected_inner).into();

  assert_eq!(expected, searcher.rewrite(query)?);

  Ok(())
}

#[test]
fn test_should_clauses_less_than_or_equal_to_minimum_number_should_match() -> Result<()> {
  let searcher = new_searcher_with_reader(MultiReader::empty()?)?;

  // The only one SHOULD clause is MatchNoDocsQuery
  let mut b = Builder::new();
  b.add(PhraseQuery::from_terms(0, "field", &[])?, Occur::Should)?;
  b.set_minimum_number_should_match(1);
  let query: Query = b.build().into();
  assert_eq!(
    Query::MatchNoDocs(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  let mut b = Builder::new();
  b.add(PhraseQuery::from_terms(0, "field", &[])?, Occur::Should)?;
  b.set_minimum_number_should_match(0);
  let query: Query = b.build().into();
  assert_eq!(
    Query::MatchNoDocs(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  // Meaningful SHOULD clause count is less than MinimumNumberShouldMatch
  let mut b = Builder::new();
  b.add(PhraseQuery::from_terms(0, "field", &[])?, Occur::Should)?;
  b.add(PhraseQuery::from_terms(0, "field", &["a"])?, Occur::Should)?;
  b.set_minimum_number_should_match(2);
  let query: Query = b.build().into();
  assert_eq!(
    Query::MatchNoDocs(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  // Meaningful SHOULD clause count is equal to MinimumNumberShouldMatch
  let mut b = Builder::new();
  b.add(PhraseQuery::from_terms(0, "field", &["b"])?, Occur::Should)?;
  b.add(
    PhraseQuery::from_terms(0, "field", &["a", "c"])?,
    Occur::Should,
  )?;
  b.set_minimum_number_should_match(2);
  let query: Query = b.build().into();

  let mut eb = Builder::new();
  eb.add(TermQuery::new(Term::from_text("field", "b")), Occur::Must)?;
  eb.add(
    PhraseQuery::from_terms(0, "field", &["a", "c"])?,
    Occur::Must,
  )?;
  let expected: Query = eb.build().into();

  assert_eq!(expected, searcher.rewrite(query)?);

  // Invalid Inner query get removed after rewrite
  let mut ib = Builder::new();
  ib.add(PhraseQuery::from_terms(0, "field", &[])?, Occur::Should)?;
  ib.add(PhraseQuery::from_terms(0, "field", &["a"])?, Occur::Should)?;
  ib.set_minimum_number_should_match(2);
  let inner: Query = ib.build().into();

  let mut b = Builder::new();
  b.add(inner.clone(), Occur::Should)?;
  b.add(PhraseQuery::from_terms(0, "field", &["b"])?, Occur::Should)?;
  b.add(
    PhraseQuery::from_terms(0, "field", &["a", "c"])?,
    Occur::Should,
  )?;
  b.set_minimum_number_should_match(2);
  let query: Query = b.build().into();
  assert_eq!(expected, searcher.rewrite(query)?);

  let mut b = Builder::new();
  b.add(inner, Occur::Should)?;
  b.add(PhraseQuery::from_terms(0, "field", &["b"])?, Occur::Should)?;
  b.set_minimum_number_should_match(2);
  let query: Query = b.build().into();
  assert_eq!(
    Query::MatchNoDocs(MatchNoDocsQuery::new()),
    searcher.rewrite(query)?
  );

  Ok(())
}
