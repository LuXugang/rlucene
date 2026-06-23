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
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::multi_phrase_query::{
  Builder as MultiPhraseQueryBuilder, MultiPhraseQuery,
};
use crate::core::search::phrase_query::Builder as PhraseQueryBuilder;
use crate::core::search::query::{Query, QueryBase};
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::term_query::TermQuery;
use crate::core::store::ByteBuffersDirectory;
use crate::core::util::CoreHelper;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::token;
use crate::test::core::analysis::token::Token;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_searcher_with_reader, new_text_field, random,
};
use rand_chacha::rand_core::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestMultiPhraseQuery;

#[test]
fn test_phrase_prefix() -> Result<()> {
  let mut random = random();
  let index_store = new_directory_shared(&mut random)?;

  let mut field_to_type = HashMap::new();
  let config = new_index_writer_config(&mut random)?;
  let writer = RandomIndexWriter::with_config(&mut random, index_store.clone(), config);

  add(&mut random, "blueberry pie", &writer, &mut field_to_type)?;
  add(
    &mut random,
    "blueberry strudel",
    &writer,
    &mut field_to_type,
  )?;
  add(&mut random, "blueberry pizza", &writer, &mut field_to_type)?;
  add(
    &mut random,
    "blueberry chewing gum",
    &writer,
    &mut field_to_type,
  )?;
  add(&mut random, "bluebird pizza", &writer, &mut field_to_type)?;
  add(
    &mut random,
    "bluebird foobar pizza",
    &writer,
    &mut field_to_type,
  )?;
  add(
    &mut random,
    "piccadilly circus",
    &writer,
    &mut field_to_type,
  )?;

  writer.force_merge(&mut random, 1)?;
  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  // search for "blueberry pi*":
  let mut query1builder = MultiPhraseQuery::builder();
  query1builder.add_term(Term::from_text("body", "blueberry"))?;

  // search for "strawberry pi*":
  let mut query2builder = MultiPhraseQuery::builder();
  query2builder.add_term(Term::from_text("body", "strawberry"))?;

  let mut terms_with_prefix: Vec<Term> = Vec::new();

  let prefix = "pi";
  let leaves = searcher.reader_context.leaves()?;
  let r = leaves[0].reader();
  let terms_opt = r.terms("body")?;
  let terms = terms_opt.expect("terms for body should exist");
  let mut te = terms.iterator()?;
  te.seek_ceil(&BytesRef::from_string(prefix))?;
  loop {
    let term_bytes = te.term()?;
    let s = term_bytes.utf8_to_string()?;
    if s.starts_with(prefix) {
      terms_with_prefix.push(Term::from_text("body", &s));
    } else {
      break;
    }
    if te.next()?.is_none() {
      break;
    }
  }

  query1builder.add_terms(&terms_with_prefix)?;
  let query1 = query1builder.build();
  assert_eq!(
    "body:\"blueberry (piccadilly pie pizza)\"",
    query1.to_string("")?
  );
  query2builder.add_terms(&terms_with_prefix)?;
  let query2 = query2builder.build();
  assert_eq!(
    "body:\"strawberry (piccadilly pie pizza)\"",
    query2.to_string("")?
  );

  let result = searcher.search(query1, 1000)?;
  assert_eq!(2, result.total_hits.value());

  let result = searcher.search(query2, 1000)?;
  assert_eq!(0, result.total_hits.value());

  // search for "blue* pizza":
  let mut query3builder = MultiPhraseQuery::builder();
  terms_with_prefix.clear();
  let prefix = "blue";
  let leaves = searcher.reader_context.leaves()?;
  let r = leaves[0].reader();
  let terms_opt = r.terms("body")?;
  let terms = terms_opt.expect("terms for body should exist");
  let mut te = terms.iterator()?;
  te.seek_ceil(&BytesRef::from_string(prefix))?;
  loop {
    let term_bytes = te.term()?;
    let s = term_bytes.utf8_to_string()?;
    if s.starts_with(prefix) {
      terms_with_prefix.push(Term::from_text("body", &s));
    } else {
      break;
    }
    if te.next()?.is_none() {
      break;
    }
  }

  query3builder
    .add_terms(&terms_with_prefix)?
    .add_term(Term::from_text("body", "pizza"))?;
  let query3 = query3builder.build();
  assert_eq!("body:\"(blueberry bluebird) pizza\"", query3.to_string("")?);

  let result = searcher.search(query3, 1000)?;
  assert_eq!(2, result.total_hits.value()); // blueberry pizza, bluebird pizza

  // test slop:
  let mut slop_builder = MultiPhraseQuery::builder();
  slop_builder.set_slop(1)?;
  slop_builder
    .add_terms(&terms_with_prefix)?
    .add_term(Term::from_text("body", "pizza"))?;
  let query3 = slop_builder.build();
  let result = searcher.search(query3, 1000)?;
  assert_eq!(3, result.total_hits.value()); // blueberry pizza, bluebird pizza, bluebird foobar pizza
  let mut query4builder = MultiPhraseQuery::builder();

  let err = query4builder
    .add_term(Term::from_text("field1", "foo"))?
    .add_term(Term::from_text("field2", "foobar"));

  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  Ok(())
}

#[test]
fn test_tall() -> Result<()> {
  let mut random = random();
  let index_store = new_directory_shared(&mut random)?;

  let mut field_to_type = HashMap::new();
  let config = new_index_writer_config(&mut random)?;
  let writer = RandomIndexWriter::with_config(&mut random, index_store.clone(), config);

  add(
    &mut random,
    "blueberry chocolate pie",
    &writer,
    &mut field_to_type,
  )?;
  add(
    &mut random,
    "blueberry chocolate tart",
    &writer,
    &mut field_to_type,
  )?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_term(Term::from_text("body", "blueberry"))?
    .add_term(Term::from_text("body", "chocolate"))?
    .add_terms(&[
      Term::from_text("body", "pie"),
      Term::from_text("body", "tart"),
    ])?;
  assert_eq!(2, searcher.count(qb.build())?);
  Ok(())
}

/// Tests in Java could not pass
fn test_multi_sloppy_with_repeats() -> Result<()> {
  let mut random = random();
  let index_store = new_directory_shared(&mut random)?;

  let mut field_to_type = HashMap::new();
  let config = new_index_writer_config(&mut random)?;
  let writer = RandomIndexWriter::with_config(&mut random, index_store.clone(), config);

  add(
    &mut random,
    "a b c d e f g h i k",
    &writer,
    &mut field_to_type,
  )?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut qb = MultiPhraseQuery::builder();
  qb.set_slop(6)?;
  // this will fail, when the scorer would propagate [a] rather than [a,b],
  qb.add_terms(&[Term::from_text("body", "a"), Term::from_text("body", "b")])?
    .add_terms(&[Term::from_text("body", "a")])?;
  assert_eq!(1, searcher.count(qb.build())?); // should match on "a b"
  Ok(())
}

/// Tests exact MPQ with repeated terms
#[test]
fn test_multi_exact_with_repeats() -> Result<()> {
  let mut random = random();
  let index_store = new_directory_shared(&mut random)?;

  let mut field_to_type = HashMap::new();
  let config = new_index_writer_config(&mut random)?;
  let writer = RandomIndexWriter::with_config(&mut random, index_store.clone(), config);

  add(
    &mut random,
    "a b c d e f g h i k",
    &writer,
    &mut field_to_type,
  )?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;
  let mut qb = MultiPhraseQuery::builder();
  qb.add_terms_with_position(
    &[Term::from_text("body", "a"), Term::from_text("body", "d")],
    0,
  )?
  .add_terms_with_position(
    &[Term::from_text("body", "a"), Term::from_text("body", "f")],
    2,
  )?;
  assert_eq!(1, searcher.count(qb.build())?); // should match on "a b"
  Ok(())
}

fn add<R, D>(
  random: &mut R,
  s: &str,
  writer: &RandomIndexWriter<D>,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: crate::core::store::directory::Directory + 'static,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "body",
    s,
    Store::Yes,
    field_to_type,
  )?);
  writer.add_document(random, doc)?;
  Ok(())
}
#[test]
fn test_boolean_query_containing_single_term_prefix_query() -> Result<()> {
  // this tests against bug 33161 (now fixed)
  // In order to cause the bug, the outer query must have more than one term
  // and all terms required.
  // The contained PhraseMultiQuery must contain exactly one term array.

  let mut random = random();

  let index_store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, index_store.clone())?;

  let mut field_to_type = HashMap::new();
  add(&mut random, "blueberry pie", &writer, &mut field_to_type)?;
  add(
    &mut random,
    "blueberry chewing gum",
    &writer,
    &mut field_to_type,
  )?;
  add(
    &mut random,
    "blue raspberry pie",
    &writer,
    &mut field_to_type,
  )?;

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  // This query will be equivalent to +body:pie +body:"blue*"
  let mut q = Builder::new();

  q.add(TermQuery::new(Term::from_text("body", "pie")), Occur::Must)?;

  let mut trouble_builder = MultiPhraseQuery::builder();
  trouble_builder.add_terms(&[
    Term::from_text("body", "blueberry"),
    Term::from_text("body", "blue"),
  ])?;

  q.add(trouble_builder.build(), Occur::Must)?;

  // error will be returned here without fix
  let query = q.build();
  let hits = searcher.search(query.clone(), 1000)?.score_docs;

  assert_eq!(2, hits.len(), "Wrong number of hits");

  // just make sure no exc:
  searcher.explain(query, 0)?;

  writer.close(&mut random)?;

  Ok(())
}

#[test]
fn test_phrase_prefix_with_boolean_query() -> Result<()> {
  let mut random = random();

  let index_store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, index_store.clone())?;

  let mut field_to_type = HashMap::new();
  add_with_type(
    &mut random,
    "This is a test",
    "object",
    &writer,
    &mut field_to_type,
  )?;
  add_with_type(&mut random, "a note", "note", &writer, &mut field_to_type)?;

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  // This query will be equivalent to +type:note +body:"a t*"
  let mut q = Builder::new();

  q.add(TermQuery::new(Term::from_text("type", "note")), Occur::Must)?;

  let mut trouble_builder = MultiPhraseQuery::builder();
  trouble_builder.add_term(Term::from_text("body", "a"))?;
  trouble_builder.add_terms(&[
    Term::from_text("body", "test"),
    Term::from_text("body", "this"),
  ])?;

  q.add(trouble_builder.build(), Occur::Must)?;

  // error will be returned here without fix for #35626:
  let hits = searcher.search(q.build(), 1000)?.score_docs;

  assert_eq!(0, hits.len(), "Wrong number of hits");

  writer.close(&mut random)?;

  Ok(())
}
#[test]
fn test_no_docs() -> Result<()> {
  let mut random = random();

  let index_store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, index_store.clone())?;
  let mut field_to_type = HashMap::new();
  add_with_type(&mut random, "a note", "note", &writer, &mut field_to_type)?;

  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let mut qb = MultiPhraseQuery::builder();
  qb.add_term(Term::from_text("body", "a"))?;
  qb.add_terms(&[
    Term::from_text("body", "nope"),
    Term::from_text("body", "nope"),
  ])?;

  let q = qb.build();

  assert_eq!(0, searcher.count(q.clone())?, "Wrong number of hits");

  // just make sure no exc:
  searcher.explain(q, 0)?;

  writer.close(&mut random)?;

  Ok(())
}

#[test]
fn test_hash_code_and_equals() -> Result<()> {
  let query1builder = MultiPhraseQuery::builder();
  let query1 = query1builder.build();

  let query2builder = MultiPhraseQuery::builder();
  let query2 = query2builder.build();

  assert_eq!(
    CoreHelper::calculate_hash(&query1),
    CoreHelper::calculate_hash(&query2)
  );
  assert_eq!(query1, query2);

  let term1 = Term::from_text("someField", "someText");

  let mut query1builder = MultiPhraseQueryBuilder::from_query(&query1);
  query1builder.add_term(term1.clone())?;
  let query1 = query1builder.build();

  let mut query2builder = MultiPhraseQueryBuilder::from_query(&query2);
  query2builder.add_term(term1)?;
  let query2 = query2builder.build();

  assert_eq!(
    CoreHelper::calculate_hash(&query1),
    CoreHelper::calculate_hash(&query2)
  );
  assert_eq!(query1, query2);

  let term2 = Term::from_text("someField", "someMoreText");

  let mut query1builder = MultiPhraseQueryBuilder::from_query(&query1);
  query1builder.add_term(term2.clone())?;
  let query1 = query1builder.build();

  assert_ne!(
    CoreHelper::calculate_hash(&query1),
    CoreHelper::calculate_hash(&query2)
  );
  assert_ne!(query1, query2);

  let mut query2builder = MultiPhraseQueryBuilder::from_query(&query2);
  query2builder.add_term(term2)?;
  let query2 = query2builder.build();

  assert_eq!(
    CoreHelper::calculate_hash(&query1),
    CoreHelper::calculate_hash(&query2)
  );
  assert_eq!(query1, query2);

  Ok(())
}

fn add_with_type<R, D>(
  random: &mut R,
  s: &str,
  typ: &str,
  writer: &RandomIndexWriter<D>,
  field_to_type: &mut HashMap<String, FieldType>,
) -> Result<()>
where
  D: crate::core::store::directory::Directory + 'static,
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  doc.add(new_text_field(
    random,
    "body",
    s,
    Store::Yes,
    field_to_type,
  )?);
  doc.add(StringField::from_string("type", typ, Store::No)?);
  writer.add_document(random, doc)?;
  Ok(())
}
#[test]
fn test_empty_to_string() -> Result<()> {
  let query1builder = MultiPhraseQuery::builder();
  let query1 = query1builder.build();
  let _ = query1.to_string("")?;
  Ok(())
}

#[test]
fn test_zero_pos_incr() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(ByteBuffersDirectory::new());

  let tokens = vec![
    make_token("a", 1)?,
    make_token("b", 0)?,
    make_token("c", 0)?,
  ];

  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(tokens.clone())),
  )?);
  writer.add_document(&mut random, doc)?;

  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(tokens)),
  )?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut mpqb = MultiPhraseQuery::builder();

  mpqb.add_terms_with_position(
    &[Term::from_text("field", "b"), Term::from_text("field", "c")],
    0,
  )?;
  mpqb.add_terms_with_position(&[Term::from_text("field", "a")], 0)?;

  let hits = searcher.search(mpqb.build(), 2)?.score_docs;

  assert_eq!(2, hits.len());
  assert!((hits[0].score() - hits[1].score()).abs() < 1e-5);

  Ok(())
}
fn make_token(text: &str, pos_incr: i32) -> Result<Token> {
  let mut t = token::new()?;
  CharTermAttribute::append_str(&mut t, Some(text));
  t.set_position_increment(pos_incr)?;
  Ok(t)
}
fn incr_0_doc_tokens() -> Result<Vec<Token>> {
  Ok(vec![
    make_token("x", 1)?,
    make_token("a", 1)?,
    make_token("1", 0)?,
    make_token("m", 1)?, // not existing, relying on slop=2
    make_token("b", 1)?,
    make_token("1", 0)?,
    make_token("n", 1)?, // not existing, relying on slop=2
    make_token("c", 1)?,
    make_token("y", 1)?,
  ])
}

fn incr_0_query_tokens_and() -> Result<Vec<Token>> {
  Ok(vec![
    make_token("a", 1)?,
    make_token("1", 0)?,
    make_token("b", 1)?,
    make_token("1", 0)?,
    make_token("c", 1)?,
  ])
}

fn incr_0_query_tokens_and_or_match() -> Result<Vec<Vec<Token>>> {
  Ok(vec![
    vec![make_token("a", 1)?],
    vec![make_token("x", 1)?, make_token("1", 0)?],
    vec![make_token("b", 2)?],
    vec![make_token("x", 2)?, make_token("1", 0)?],
    vec![make_token("c", 3)?],
  ])
}

fn incr_0_query_tokens_and_or_no_match() -> Result<Vec<Vec<Token>>> {
  Ok(vec![
    vec![make_token("x", 1)?],
    vec![make_token("a", 1)?, make_token("1", 0)?],
    vec![make_token("x", 2)?],
    vec![make_token("b", 2)?, make_token("1", 0)?],
    vec![make_token("c", 3)?],
  ])
}
/// using query parser, MPQ will be created, and will not be strict about having all query terms in
/// each position - one of each position is sufficient (OR logic)
#[test]
fn test_zero_pos_incr_sloppy_parsed_and() -> Result<()> {
  let mut qb = MultiPhraseQuery::builder();

  qb.add_terms_with_position(
    &[Term::from_text("field", "a"), Term::from_text("field", "1")],
    -1,
  )?;

  qb.add_terms_with_position(
    &[Term::from_text("field", "b"), Term::from_text("field", "1")],
    0,
  )?;

  qb.add_terms_with_position(&[Term::from_text("field", "c")], 1)?;

  do_test_zero_pos_incr_sloppy(qb.clone().build(), 0)?;

  qb.set_slop(1)?;
  do_test_zero_pos_incr_sloppy(qb.clone().build(), 0)?;

  qb.set_slop(2)?;
  do_test_zero_pos_incr_sloppy(qb.build(), 1)?;

  Ok(())
}
fn do_test_zero_pos_incr_sloppy(query: impl Into<Query>, n_expected: i32) -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let config = new_index_writer_config(&mut random)?;
  let writer = RandomIndexWriter::with_config(&mut random, dir, config);

  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "field",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(incr_0_doc_tokens()?)),
  )?);
  writer.add_document(&mut random, doc)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;

  let hits = searcher.search(query, 1)?;
  assert_eq!(n_expected as usize, hits.total_hits.value());

  Ok(())
}

/// PQ AND Mode - Manually creating a phrase query
#[test]
fn test_zero_pos_incr_sloppy_pq_and() -> Result<()> {
  let mut builder = PhraseQueryBuilder::new();
  let mut pos: i32 = -1;
  for tap in incr_0_query_tokens_and()? {
    pos += tap.get_position_increment()?;
    builder.add(Term::from_text("field", tap.to_string()), pos as usize)?;
  }
  builder.set_slop(0);
  do_test_zero_pos_incr_sloppy(builder.clone().build()?, 0)?;
  builder.set_slop(1);
  do_test_zero_pos_incr_sloppy(builder.clone().build()?, 0)?;
  builder.set_slop(2);
  do_test_zero_pos_incr_sloppy(builder.build()?, 1)?;
  Ok(())
}

/// MPQ AND Mode - Manually creating a multiple phrase query
#[test]
fn test_zero_pos_incr_sloppy_mpq_and() -> Result<()> {
  let mut mpqb = MultiPhraseQuery::builder();

  let mut pos: i32 = -1;
  for tap in incr_0_query_tokens_and()? {
    pos += tap.get_position_increment()?;
    let terms = vec![Term::from_text("field", tap.to_string())];
    mpqb.add_terms_with_position(terms.as_ref(), pos)?; // AND logic
  }

  do_test_zero_pos_incr_sloppy(mpqb.clone().build(), 0)?;

  mpqb.set_slop(1)?;
  do_test_zero_pos_incr_sloppy(mpqb.clone().build(), 0)?;

  mpqb.set_slop(2)?;
  do_test_zero_pos_incr_sloppy(mpqb.build(), 1)?;

  Ok(())
}

#[test]
fn test_zero_pos_incr_sloppy_mpq_and_or_match() -> Result<()> {
  let mut mpqb = MultiPhraseQuery::builder();

  for tap in incr_0_query_tokens_and_or_match()? {
    let terms = tap_terms(&tap);
    let pos = tap[0].get_position_increment()? - 1;
    mpqb.add_terms_with_position(terms.as_ref(), pos)?; // AND logic in pos, OR across lines
  }

  do_test_zero_pos_incr_sloppy(mpqb.clone().build(), 0)?;

  mpqb.set_slop(1)?;
  do_test_zero_pos_incr_sloppy(mpqb.clone().build(), 0)?;

  mpqb.set_slop(2)?;
  do_test_zero_pos_incr_sloppy(mpqb.build(), 1)?;

  Ok(())
}

/// MPQ Combined AND OR Mode - Manually creating a multiple phrase query - with no match
#[test]
fn test_zero_pos_incr_sloppy_mpq_and_or_no_match() -> Result<()> {
  let mut mpqb = MultiPhraseQuery::builder();

  for tap in incr_0_query_tokens_and_or_no_match()? {
    let terms = tap_terms(&tap);
    let pos = tap[0].get_position_increment()? - 1;
    mpqb.add_terms_with_position(terms.as_ref(), pos)?; // AND logic in pos, OR across lines
  }

  do_test_zero_pos_incr_sloppy(mpqb.clone().build(), 0)?;

  mpqb.set_slop(2)?;
  do_test_zero_pos_incr_sloppy(mpqb.build(), 0)?;

  Ok(())
}
fn tap_terms(tap: &[Token]) -> Vec<Term> {
  tap
    .iter()
    .map(|token| Term::from_text("field", token.to_string()))
    .collect()
}
#[test]
fn test_negative_slop() -> Result<()> {
  let mut builder = MultiPhraseQuery::builder();
  let query_builder = builder
    .add_term(Term::from_text("field", "two"))?
    .add_term(Term::from_text("field", "one"))?;

  let err = query_builder.set_slop(-2);

  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));

  Ok(())
}
