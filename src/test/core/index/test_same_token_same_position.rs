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
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::TextField;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, random,
};
#[allow(dead_code)] // for quick search
pub struct TestSameTokenSamePosition;

/// Attempt to reproduce an assertion error that happens only with the trunk version around April
/// 2011.
#[test]
fn test() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let riw = RandomIndexWriter::new(&mut random, dir);
  let mut doc = Document::new();
  doc.add(TextField::from_token_stream(
    "eng",
    FieldTokenStreamEnum::custom(BugReproTokenStream::new()),
  )?);
  riw.add_document(doc)?;
  riw.close()?;
  Ok(())
}

/// Same as the above, but with more docs.
#[test]
fn test_more_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let riw = RandomIndexWriter::new(&mut random, dir);
  for _ in 0..100 {
    let mut doc = Document::new();
    doc.add(TextField::from_token_stream(
      "eng",
      FieldTokenStreamEnum::custom(BugReproTokenStream::new()),
    )?);
    riw.add_document(doc)?;
  }
  riw.close()?;
  Ok(())
}

struct BugReproTokenStream {
  attrs: Attributes,
  next_token_index: usize,
}

impl BugReproTokenStream {
  const TOKEN_COUNT: usize = 4;
  const TERMS: [&'static str; Self::TOKEN_COUNT] = ["six", "six", "drunken", "drunken"];
  const STARTS: [i32; Self::TOKEN_COUNT] = [0, 0, 4, 4];
  const ENDS: [i32; Self::TOKEN_COUNT] = [3, 3, 11, 11];
  const INCS: [i32; Self::TOKEN_COUNT] = [1, 0, 1, 0];

  fn new() -> Self {
    Self {
      attrs: Attributes::default(),
      next_token_index: 0,
    }
  }
}

impl TokenStream for BugReproTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.next_token_index < Self::TOKEN_COUNT {
      self.attrs.clear_attributes();
      self
        .attrs
        .append_str(Some(Self::TERMS[self.next_token_index]))?;
      self.attrs.set_offset(
        Self::STARTS[self.next_token_index],
        Self::ENDS[self.next_token_index],
      )?;
      self
        .attrs
        .set_position_increment(Self::INCS[self.next_token_index])?;
      self.next_token_index += 1;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn end(&mut self) -> Result<()> {
    self.default_end()
  }

  fn reset(&mut self) -> Result<()> {
    self.next_token_index = 0;
    Ok(())
  }

  fn get_attribute_source(&self) -> &Attributes {
    &self.attrs
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.attrs
  }
}
