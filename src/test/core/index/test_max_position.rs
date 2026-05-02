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
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::core::document::document::Document;
use crate::core::document::fields::FieldTokenStreamEnum;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::MAX_POSITION;
use crate::core::index::multi_terms::get_term_postings_enum;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::canned_token_stream::CannedTokenStream;
use crate::test::core::analysis::token;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, random,
};
use rand::RngExt;
#[allow(dead_code)] // for quick search
pub struct TestMaxPosition;

#[test]
fn test_too_big_position() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir);

  let mut doc = Document::new();
  // This is at position 1:
  let mut t1 = token::with_pos_inc("foo", 2, 0, 3)?;
  if random.random_bool(0.5) {
    t1.sub
      .token
      .set_payload(Some(BytesRef::from_bytes(vec![0x1])));
  }

  let mut t2 = token::with_range(Some("foo"), 4, 7)?;
  // This should overflow max:
  t2.sub.set_position_increment(MAX_POSITION)?;
  if random.random_bool(0.5) {
    t2.sub
      .token
      .set_payload(Some(BytesRef::from_bytes(vec![0x1])));
  }

  doc.add(TextField::from_token_stream(
    "foo",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![t1, t2])),
  )?);
  assert!(iw.add_document(doc).is_err());

  // Document should not be visible:
  let r = iw.get_reader()?;
  assert_eq!(0, r.num_docs()?);
  iw.close()?;

  Ok(())
}

#[test]
fn test_max_position() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir);

  let mut doc = Document::new();
  // This is at position 0:
  let mut t1 = token::with_range(Some("foo"), 0, 3)?;
  if random.random_bool(0.5) {
    t1.sub
      .token
      .set_payload(Some(BytesRef::from_bytes(vec![0x1])));
  }

  let mut t2 = token::with_range(Some("foo"), 4, 7)?;
  t2.sub.set_position_increment(MAX_POSITION)?;
  if random.random_bool(0.5) {
    t2.sub
      .token
      .set_payload(Some(BytesRef::from_bytes(vec![0x1])));
  }

  doc.add(TextField::from_token_stream(
    "foo",
    FieldTokenStreamEnum::custom(CannedTokenStream::new(vec![t1, t2])),
  )?);
  iw.add_document(doc)?;

  // Document should be visible:
  let r = iw.get_reader()?;
  assert_eq!(1, r.num_docs()?);
  let mut postings = get_term_postings_enum(&r, "foo", &BytesRef::from_string("foo"))?
    .expect("postings enum for term 'foo' must exist");

  // "foo" appears in docID=0
  assert_eq!(0, postings.next_doc()?);

  // "foo" appears 2 times in the doc
  assert_eq!(2, postings.freq()?);

  // first at pos=0
  assert_eq!(0, postings.next_position()?);

  // next at pos=MAX
  assert_eq!(MAX_POSITION, postings.next_position()?);

  iw.close()?;

  Ok(())
}
