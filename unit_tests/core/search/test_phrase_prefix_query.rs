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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::index::{BytesRef, multi_terms};
use crate::core::search::multi_phrase_query::MultiPhraseQuery;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, new_text_field, random,
};
use std::collections::{HashMap, LinkedList};

#[allow(dead_code)] // for quick search
struct TestPhrasePrefixQuery;

#[test]
fn test_phrase_prefix() -> Result<()> {
  let mut random = random();

  let index_store = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, index_store.clone())?;

  let mut field_to_type = HashMap::new();

  let mut doc1 = Document::new();
  doc1.add(new_text_field(
    &mut random,
    "body",
    "blueberry pie",
    Store::Yes,
    &mut field_to_type,
  )?);

  let mut doc2 = Document::new();
  doc2.add(new_text_field(
    &mut random,
    "body",
    "blueberry strudel",
    Store::Yes,
    &mut field_to_type,
  )?);

  let mut doc3 = Document::new();
  doc3.add(new_text_field(
    &mut random,
    "body",
    "blueberry pizza",
    Store::Yes,
    &mut field_to_type,
  )?);

  let mut doc4 = Document::new();
  doc4.add(new_text_field(
    &mut random,
    "body",
    "blueberry chewing gum",
    Store::Yes,
    &mut field_to_type,
  )?);

  let mut doc5 = Document::new();
  doc5.add(new_text_field(
    &mut random,
    "body",
    "piccadilly circus",
    Store::Yes,
    &mut field_to_type,
  )?);

  writer.add_document(&mut random, doc1)?;
  writer.add_document(&mut random, doc2)?;
  writer.add_document(&mut random, doc3)?;
  writer.add_document(&mut random, doc4)?;
  writer.add_document(&mut random, doc5)?;

  let reader = writer.get_reader(&mut random)?;
  writer.close(&mut random)?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut query1builder = MultiPhraseQuery::builder();
  let mut query2builder = MultiPhraseQuery::builder();

  query1builder.add_term(Term::from_text("body", "blueberry"))?;
  query2builder.add_term(Term::from_text("body", "strawberry"))?;

  let mut terms_with_prefix = LinkedList::new();

  let terms = multi_terms::get_terms(searcher.reader_context.reader(), "body")?.unwrap();
  let mut te = terms.iterator()?;

  let prefix = "pi";

  te.seek_ceil(&BytesRef::from_string(prefix))?;

  loop {
    let term = te.term()?;
    let s = term.utf8_to_string()?;

    if s.starts_with(prefix) {
      terms_with_prefix.push_back(Term::from_text("body", s));
    } else {
      break;
    }

    if te.next()?.is_none() {
      break;
    }
  }

  let terms: Vec<_> = terms_with_prefix.iter().cloned().collect();

  query1builder.add_terms(&terms)?;
  query2builder.add_terms(&terms)?;

  let result = searcher.search(query1builder.build(), 1000)?.score_docs;
  assert_eq!(2, result.len());

  let result = searcher.search(query2builder.build(), 1000)?.score_docs;
  assert_eq!(0, result.len());

  searcher.reader_context.reader().close()?;

  Ok(())
}
