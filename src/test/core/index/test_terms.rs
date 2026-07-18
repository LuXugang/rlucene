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
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::multi_terms;
use crate::core::index::terms::Terms;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::canned_binary_token_stream::{
  BinaryToken, CannedBinaryTokenStream,
};
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, at_least_usize, new_directory_shared, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;

#[allow(dead_code)] // for quick search
struct TestTerms;
#[test]
fn test_term_min_max_basic() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir)?;

  let mut doc = Document::new();
  doc.add(TextField::from_string("field", "a b c cc ddd", No)?);
  w.add_document(&mut random, doc)?;

  let r = w.get_reader(&mut random)?;
  let terms = multi_terms::get_terms(&r, "field")?.expect("terms should exist");
  assert_eq!(
    &BytesRef::from_string("a"),
    terms.get_min()?.expect("min term should exist").as_ref()
  );
  assert_eq!(
    &BytesRef::from_string("ddd"),
    terms.get_max()?.expect("max term should exist").as_ref()
  );

  w.close(&mut random)?;
  Ok(())
}

#[test]
fn test_term_min_max_random() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir)?;

  let num_docs = at_least(&mut random, 100);
  let mut min_term: Option<BytesRef<Vec<u8>>> = None;
  let mut max_term: Option<BytesRef<Vec<u8>>> = None;

  for _ in 0..num_docs {
    let mut doc = Document::new();
    let num_tokens = at_least_usize(&mut random, 10);
    let mut tokens = Vec::with_capacity(num_tokens);

    for _ in 0..num_tokens {
      let mut bytes = vec![0u8; TestUtil::next_int(&mut random, 1, 20) as usize];
      random.fill_bytes(&mut bytes);
      let token_bytes = BytesRef::from_bytes(bytes);

      if min_term
        .as_ref()
        .is_none_or(|min_term| token_bytes < *min_term)
      {
        min_term = Some(BytesRef::deep_copy_of(&token_bytes));
      }
      if max_term
        .as_ref()
        .is_none_or(|max_term| token_bytes > *max_term)
      {
        max_term = Some(BytesRef::deep_copy_of(&token_bytes));
      }

      tokens.push(BinaryToken::new(token_bytes));
    }

    doc.add(TextField::from_token_stream(
      "field",
      FieldTokenStreamEnum::custom(CannedBinaryTokenStream::new(tokens)?),
    )?);
    w.add_document(&mut random, doc)?;
  }

  let r = w.get_reader(&mut random)?;
  let terms = multi_terms::get_terms(&r, "field")?.expect("terms should exist");
  assert_eq!(
    min_term.as_ref().expect("min term should exist"),
    terms.get_min()?.expect("min term should exist").as_ref()
  );
  assert_eq!(
    max_term.as_ref().expect("max term should exist"),
    terms.get_max()?.expect("max term should exist").as_ref()
  );

  w.close(&mut random)?;
  Ok(())
}
