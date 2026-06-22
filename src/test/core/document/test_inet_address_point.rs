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
use crate::core::document::inet_address_point::InetAddressPoint;
use crate::core::index::index_reader::IndexReader;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  new_directory_shared, new_searcher_with_reader, random,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;

#[allow(dead_code)] // for quick search
struct TestInetAddressPoint;

/** Add a single address and search for it */
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();
  let mut dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());

  // add a doc with an address
  let mut document = Document::new();
  let address = "1.2.3.4".parse::<IpAddr>().expect("valid IP literal");
  document.add(InetAddressPoint::new("field", address)?);
  writer.add_document(&mut random, document)?;

  // search and verify we found our doc
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_exact_query("field", address)?)?
  );
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_prefix_query("field", address, 24)?)?
  );
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_range_query(
      "field",
      "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.5".parse::<IpAddr>().expect("valid IP literal")
    )?)?
  );
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_set_query(
      "field",
      ["1.2.3.4".parse::<IpAddr>().expect("valid IP literal")]
    )?)?
  );
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_set_query(
      "field",
      [
        "1.2.3.4".parse::<IpAddr>().expect("valid IP literal"),
        "1.2.3.5".parse::<IpAddr>().expect("valid IP literal")
      ]
    )?)?
  );
  assert_eq!(
    0,
    searcher.count(InetAddressPoint::new_set_query(
      "field",
      ["1.2.3.3".parse::<IpAddr>().expect("valid IP literal")]
    )?)?
  );
  assert_eq!(
    0,
    searcher.count(InetAddressPoint::new_set_query(
      "field",
      Vec::<IpAddr>::new()
    )?)?
  );

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()?;
  Ok(())
}

/** Add a single address and search for it */
#[test]
fn test_basics_v6() -> Result<()> {
  let mut random = random();
  let mut dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());

  // add a doc with an address
  let mut document = Document::new();
  let address = "fec0::f66d".parse::<IpAddr>().expect("valid IP literal");
  document.add(InetAddressPoint::new("field", address)?);
  writer.add_document(&mut random, document)?;

  // search and verify we found our doc
  let reader = writer.get_reader(&mut random)?;
  let searcher = new_searcher_with_reader(reader)?;
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_exact_query("field", address)?)?
  );
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_prefix_query("field", address, 64)?)?
  );
  assert_eq!(
    1,
    searcher.count(InetAddressPoint::new_range_query(
      "field",
      "fec0::f66c".parse::<IpAddr>().expect("valid IP literal"),
      "fec0::f66e".parse::<IpAddr>().expect("valid IP literal")
    )?)?
  );

  searcher.get_index_reader().close()?;
  writer.close(&mut random)?;
  dir.close()?;
  Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
  assert_eq!(
    "InetAddressPoint <field:1.2.3.4>",
    InetAddressPoint::new(
      "field",
      "1.2.3.4".parse::<IpAddr>().expect("valid IP literal")
    )?
    .to_string()
  );
  assert_eq!(
    "InetAddressPoint <field:1.2.3.4>",
    InetAddressPoint::new(
      "field",
      "::FFFF:1.2.3.4"
        .parse::<IpAddr>()
        .expect("valid IP literal")
    )?
    .to_string()
  );
  assert_eq!(
    "InetAddressPoint <field:[fdc8:57ed:f042:ad1:f66d:4ff:fe90:ce0c]>",
    InetAddressPoint::new(
      "field",
      "fdc8:57ed:f042:0ad1:f66d:4ff:fe90:ce0c"
        .parse::<IpAddr>()
        .expect("valid IP literal")
    )?
    .to_string()
  );

  assert_eq!(
    "field:[1.2.3.4 TO 1.2.3.4]",
    InetAddressPoint::new_exact_query(
      "field",
      "1.2.3.4".parse::<IpAddr>().expect("valid IP literal")
    )?
    .to_string("")?
  );
  assert_eq!(
    "field:[0:0:0:0:0:0:0:1 TO 0:0:0:0:0:0:0:1]",
    InetAddressPoint::new_exact_query("field", "::1".parse::<IpAddr>().expect("valid IP literal"))?
      .to_string("")?
  );

  assert_eq!(
    "field:[1.2.3.0 TO 1.2.3.255]",
    InetAddressPoint::new_prefix_query(
      "field",
      "1.2.3.4".parse::<IpAddr>().expect("valid IP literal"),
      24
    )?
    .to_string("")?
  );
  assert_eq!(
    "field:[fdc8:57ed:f042:ad1:0:0:0:0 TO fdc8:57ed:f042:ad1:ffff:ffff:ffff:ffff]",
    InetAddressPoint::new_prefix_query(
      "field",
      "fdc8:57ed:f042:0ad1:f66d:4ff:fe90:ce0c"
        .parse::<IpAddr>()
        .expect("valid IP literal"),
      64
    )?
    .to_string("")?
  );
  assert_eq!(
    "field:{fdc8:57ed:f042:ad1:f66d:4ff:fe90:ce0c}",
    InetAddressPoint::new_set_query(
      "field",
      ["fdc8:57ed:f042:0ad1:f66d:4ff:fe90:ce0c"
        .parse::<IpAddr>()
        .expect("valid IP literal")]
    )?
    .to_string("")?
  );
  Ok(())
}

#[test]
fn test_query_equals() -> Result<()> {
  let mut q1 = InetAddressPoint::new_range_query(
    "a",
    "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
    "1.2.3.5".parse::<IpAddr>().expect("valid IP literal"),
  )?;
  let mut q2 = InetAddressPoint::new_range_query(
    "a",
    "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
    "1.2.3.5".parse::<IpAddr>().expect("valid IP literal"),
  )?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(
    q1,
    InetAddressPoint::new_range_query(
      "a",
      "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.7".parse::<IpAddr>().expect("valid IP literal")
    )?
  );
  assert_ne!(
    q1,
    InetAddressPoint::new_range_query(
      "b",
      "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.5".parse::<IpAddr>().expect("valid IP literal")
    )?
  );

  q1 = InetAddressPoint::new_prefix_query(
    "a",
    "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
    16,
  )?;
  q2 = InetAddressPoint::new_prefix_query(
    "a",
    "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
    16,
  )?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(
    q1,
    InetAddressPoint::new_prefix_query(
      "a",
      "1.1.3.5".parse::<IpAddr>().expect("valid IP literal"),
      16
    )?
  );
  assert_ne!(
    q1,
    InetAddressPoint::new_prefix_query(
      "a",
      "1.2.3.5".parse::<IpAddr>().expect("valid IP literal"),
      24
    )?
  );

  q1 =
    InetAddressPoint::new_exact_query("a", "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"))?;
  q2 =
    InetAddressPoint::new_exact_query("a", "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"))?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(
    q1,
    InetAddressPoint::new_exact_query("a", "1.2.3.5".parse::<IpAddr>().expect("valid IP literal"))?
  );
  let q1 = InetAddressPoint::new_set_query(
    "a",
    [
      "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.5".parse::<IpAddr>().expect("valid IP literal"),
    ],
  )?;
  let q2 = InetAddressPoint::new_set_query(
    "a",
    [
      "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.5".parse::<IpAddr>().expect("valid IP literal"),
    ],
  )?;
  assert_eq!(q1, q2);
  let mut h1 = DefaultHasher::new();
  q1.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  q2.hash(&mut h2);
  assert_eq!(h1.finish(), h2.finish());
  assert_ne!(
    q1,
    InetAddressPoint::new_set_query(
      "a",
      [
        "1.2.3.3".parse::<IpAddr>().expect("valid IP literal"),
        "1.2.3.7".parse::<IpAddr>().expect("valid IP literal")
      ]
    )?
  );
  Ok(())
}

#[test]
fn test_prefix_query() -> Result<()> {
  assert_eq!(
    InetAddressPoint::new_range_query(
      "a",
      "1.2.3.0".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.255".parse::<IpAddr>().expect("valid IP literal")
    )?,
    InetAddressPoint::new_prefix_query(
      "a",
      "1.2.3.127".parse::<IpAddr>().expect("valid IP literal"),
      24
    )?
  );
  assert_eq!(
    InetAddressPoint::new_range_query(
      "a",
      "1.2.3.128".parse::<IpAddr>().expect("valid IP literal"),
      "1.2.3.255".parse::<IpAddr>().expect("valid IP literal")
    )?,
    InetAddressPoint::new_prefix_query(
      "a",
      "1.2.3.213".parse::<IpAddr>().expect("valid IP literal"),
      25
    )?
  );
  assert_eq!(
    InetAddressPoint::new_range_query(
      "a",
      "2001::a000:0".parse::<IpAddr>().expect("valid IP literal"),
      "2001::afff:ffff"
        .parse::<IpAddr>()
        .expect("valid IP literal")
    )?,
    InetAddressPoint::new_prefix_query(
      "a",
      "2001::a6bd:fc80"
        .parse::<IpAddr>()
        .expect("valid IP literal"),
      100
    )?
  );
  Ok(())
}

#[test]
fn test_next_up() -> Result<()> {
  assert_eq!(
    "::1".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_up("::".parse::<IpAddr>().expect("valid IP literal"))?
  );

  assert_eq!(
    "::1:0".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_up("::ffff".parse::<IpAddr>().expect("valid IP literal"))?
  );

  assert_eq!(
    "1.2.4.0".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_up("1.2.3.255".parse::<IpAddr>().expect("valid IP literal"))?
  );

  assert_eq!(
    "0.0.0.0".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_up(
      "::fffe:ffff:ffff"
        .parse::<IpAddr>()
        .expect("valid IP literal")
    )?
  );

  assert_eq!(
    "::1:0:0:0".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_up(
      "255.255.255.255"
        .parse::<IpAddr>()
        .expect("valid IP literal")
    )?
  );

  let e = InetAddressPoint::next_up(
    "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"
      .parse::<IpAddr>()
      .expect("valid IP literal"),
  )
  .unwrap_err();
  assert_eq!(
    "Overflow: there is no greater InetAddress than ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
    e.to_string()
  );
  Ok(())
}

#[test]
fn test_next_down() -> Result<()> {
  assert_eq!(
    "ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe"
      .parse::<IpAddr>()
      .expect("valid IP literal"),
    InetAddressPoint::next_down(
      "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"
        .parse::<IpAddr>()
        .expect("valid IP literal")
    )?
  );

  assert_eq!(
    "::ffff".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_down("::1:0".parse::<IpAddr>().expect("valid IP literal"))?
  );

  assert_eq!(
    "1.2.3.255".parse::<IpAddr>().expect("valid IP literal"),
    InetAddressPoint::next_down("1.2.4.0".parse::<IpAddr>().expect("valid IP literal"))?
  );

  assert_eq!(
    "::fffe:ffff:ffff"
      .parse::<IpAddr>()
      .expect("valid IP literal"),
    InetAddressPoint::next_down("0.0.0.0".parse::<IpAddr>().expect("valid IP literal"))?
  );

  assert_eq!(
    "255.255.255.255"
      .parse::<IpAddr>()
      .expect("valid IP literal"),
    InetAddressPoint::next_down("::1:0:0:0".parse::<IpAddr>().expect("valid IP literal"))?
  );

  let e =
    InetAddressPoint::next_down("::".parse::<IpAddr>().expect("valid IP literal")).unwrap_err();
  assert_eq!(
    "Underflow: there is no smaller InetAddress than 0:0:0:0:0:0:0:0",
    e.to_string()
  );
  Ok(())
}
