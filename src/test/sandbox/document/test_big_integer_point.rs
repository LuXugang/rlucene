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
use crate::core::index::index_reader::IndexReader;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::sandbox::document::big_integer_point::BigIntegerPoint;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, random,
};
use num_bigint::BigInt;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[allow(dead_code)] // for quick search
struct TestBigIntegerPoint;

/** Add a single 1D point and search for it */
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let mut dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  // add a doc with a large biginteger value
  let mut document = Document::new();
  let large = BigInt::from(i64::MAX) * BigInt::from(64);
  document.add(BigIntegerPoint::new("field", [large.clone()])?);
  writer.add_document(&mut random, document)?;

  // search and verify we found our doc
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(BigIntegerPoint::new_exact_query("field", large.clone())?)?
  );
  assert_eq!(
    1,
    searcher.count(BigIntegerPoint::new_range_query(
      "field",
      &large - BigInt::from(1),
      &large + BigInt::from(1),
    )?)?
  );
  assert_eq!(
    1,
    searcher.count(BigIntegerPoint::new_set_query("field", [large.clone()])?)?
  );
  assert_eq!(
    0,
    searcher.count(BigIntegerPoint::new_set_query(
      "field",
      [&large - BigInt::from(1)]
    )?)?
  );
  assert_eq!(
    0,
    searcher.count(BigIntegerPoint::new_set_query(
      "field",
      Vec::<BigInt>::new()
    )?)?
  );

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()?;
  Ok(())
}

/** Add a negative 1D point and search for it */
#[test]
fn test_negative() -> Result<()> {
  let mut random = random();
  let mut dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone())?;

  // add a doc with a large biginteger value
  let mut document = Document::new();
  let negative = -(BigInt::from(i64::MAX) * BigInt::from(64));
  document.add(BigIntegerPoint::new("field", [negative.clone()])?);
  writer.add_document(&mut random, document)?;

  // search and verify we found our doc
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(BigIntegerPoint::new_exact_query("field", negative.clone())?)?
  );
  assert_eq!(
    1,
    searcher.count(BigIntegerPoint::new_range_query(
      "field",
      &negative - BigInt::from(1),
      &negative + BigInt::from(1),
    )?)?
  );

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()?;
  Ok(())
}

/** Test if we add a too-large value */
#[test]
fn test_too_large() -> Result<()> {
  let too_large = BigInt::from(1) << 128;
  let expected = match BigIntegerPoint::new("field", [too_large]) {
    Ok(_) => unreachable!("expected too-large BigIntegerPoint to fail"),
    Err(e) => e,
  };
  assert!(expected.to_string().contains("requires more than 16 bytes"));
  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  assert_eq!(
    "BigIntegerPoint <field:1>",
    BigIntegerPoint::new("field", [BigInt::from(1)])?.to_string()
  );
  assert_eq!(
    "BigIntegerPoint <field:1,-2>",
    BigIntegerPoint::new("field", [BigInt::from(1), BigInt::from(-2)])?.to_string()
  );
  assert_eq!(
    "field:[1 TO 1]",
    BigIntegerPoint::new_exact_query("field", BigInt::from(1))?.to_string("")?
  );
  assert_eq!(
    "field:[1 TO 17]",
    BigIntegerPoint::new_range_query("field", BigInt::from(1), BigInt::from(17))?.to_string("")?
  );
  assert_eq!(
    "field:[1 TO 17],[0 TO 42]",
    BigIntegerPoint::new_range_query_n(
      "field",
      [BigInt::from(1), BigInt::from(0)],
      [BigInt::from(17), BigInt::from(42)]
    )?
    .to_string("")?
  );
  assert_eq!(
    "field:{1}",
    BigIntegerPoint::new_set_query("field", [BigInt::from(1)])?.to_string("")?
  );
  Ok(())
}

#[test]
fn test_query_equals() -> Result<()> {
  let mut q1 = BigIntegerPoint::new_range_query("a", BigInt::from(0), BigInt::from(1000))?;
  let mut q2 = BigIntegerPoint::new_range_query("a", BigInt::from(0), BigInt::from(1000))?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(
    q1,
    BigIntegerPoint::new_range_query("a", BigInt::from(1), BigInt::from(1000))?
  );
  assert_ne!(
    q1,
    BigIntegerPoint::new_range_query("b", BigInt::from(0), BigInt::from(1000))?
  );

  q1 = BigIntegerPoint::new_exact_query("a", BigInt::from(1000))?;
  q2 = BigIntegerPoint::new_exact_query("a", BigInt::from(1000))?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(q1, BigIntegerPoint::new_exact_query("a", BigInt::from(1))?);

  let q1 =
    BigIntegerPoint::new_set_query("a", [BigInt::from(0), BigInt::from(1000), BigInt::from(17)])?;
  let q2 =
    BigIntegerPoint::new_set_query("a", [BigInt::from(17), BigInt::from(0), BigInt::from(1000)])?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(
    q1,
    BigIntegerPoint::new_set_query("a", [BigInt::from(1), BigInt::from(17), BigInt::from(1000)])?
  );
  Ok(())
}
