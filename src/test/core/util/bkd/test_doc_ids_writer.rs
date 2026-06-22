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
#[cfg(feature = "nightly")]
use crate::core::document::document::Document;
#[cfg(feature = "nightly")]
use crate::core::document::int_point::IntPoint;
#[cfg(feature = "nightly")]
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
#[cfg(feature = "nightly")]
use crate::core::index::index_writer::IndexWriter;
use crate::test::core::util::lucene_test_case::{at_least, new_directory, random};
#[cfg(feature = "nightly")]
use crate::test::core::util::lucene_test_case::{new_directory_shared, new_index_writer_config};

use crate::core::index::point_values::{IntersectVisitor, Relation};
use crate::core::store::directory::Directory;
use crate::core::store::{DataOutput, IOContext, IndexInput, IndexOutput};
use crate::core::util::bkd::doc_ids_writer::DocIdsWriter;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::collections::HashSet;

#[allow(dead_code)] // for quick search
struct TestDocIdsWriter;

#[test]
fn test_random() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 100);
  let dir = new_directory(&mut random)?;
  for _ in 0..num_iters {
    let len = 1 + random.random_range(0..5000);
    let mut doc_ids = vec![0; len];
    let bpv = TestUtil::next_int(&mut random, 1, 32);
    for doc_id in doc_ids.iter_mut().take(len) {
      *doc_id = TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
    }
    test(&mut random, &dir, &doc_ids)?;
  }
  Ok(())
}

#[test]
fn test_sorted() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 100);
  let dir = new_directory(&mut random)?;
  for _ in 0..num_iters {
    let len = 1 + random.random_range(0..5000);
    let mut doc_ids = vec![0; len];
    let bpv = TestUtil::next_int(&mut random, 1, 32);
    for doc_id in doc_ids.iter_mut().take(len) {
      *doc_id = TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
    }
    doc_ids.sort_unstable();
    test(&mut random, &dir, &doc_ids)?;
  }
  Ok(())
}

#[test]
fn test_cluster() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 100);
  let dir = new_directory(&mut random)?;
  for _ in 0..num_iters {
    let len = 1 + random.random_range(0..5000);
    let mut doc_ids = vec![0; len];
    let min = random.random_range(0..1000);
    let bpv = TestUtil::next_int(&mut random, 1, 16);
    for doc_id in doc_ids.iter_mut().take(len) {
      *doc_id = min + TestUtil::next_int(&mut random, 0, (1 << bpv) - 1);
    }
    test(&mut random, &dir, &doc_ids)?;
  }
  Ok(())
}

#[test]
fn test_bit_set() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 100);
  let dir = new_directory(&mut random)?;
  for _ in 0..num_iters {
    let size = 1 + random.random_range(0..5000);
    let mut set = HashSet::with_capacity(size);
    let small = random.random_range(0..1000);
    while set.len() < size {
      set.insert(small + random.random_range(0..(size * 16)) as i32);
    }
    let mut doc_ids: Vec<i32> = set.into_iter().collect();
    doc_ids.sort_unstable();
    test(&mut random, &dir, &doc_ids)?;
  }
  Ok(())
}
#[test]
fn test_continuous_ids() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 100);
  let dir = new_directory(&mut random)?;
  for _ in 0..num_iters {
    let size = 1 + random.random_range(0..5000);
    let mut doc_ids = vec![0; size];
    let start = random.random_range(0..1000000);
    for (i, doc_id) in doc_ids.iter_mut().take(size).enumerate() {
      *doc_id = start + i as i32;
    }
    test(&mut random, &dir, &doc_ids)?;
  }
  Ok(())
}

fn test<R>(random: &mut R, dir: &impl Directory, ints: &[i32]) -> Result<()>
where
  R: Rng + ?Sized,
{
  let len;
  let mut doc_ids_writer = DocIdsWriter::new(ints.len());
  {
    let mut out = dir.create_output("tmp", &IOContext::default_io_context()?)?;
    doc_ids_writer.write_doc_ids(ints, 0, ints.len(), &mut out)?;
    len = out.get_file_pointer();
    if random.random_bool(0.5) {
      out.write_long(0)?;
    }
  }
  {
    let mut input = dir.open_input("tmp", &IOContext::read_once_io_context()?)?;
    let mut read = vec![0; ints.len()];
    doc_ids_writer.read_ints(&mut input, ints.len(), &mut read)?;
    assert_eq!(ints, &read[..]);
    assert_eq!(len, input.get_file_pointer()?);
  }
  {
    let mut input = dir.open_input("tmp", &IOContext::read_once_io_context()?)?;
    let mut read = vec![0; ints.len()];
    let mut visitor = IntersectVisitorMock {
      i: 0,
      read: &mut read,
    };
    doc_ids_writer.read_ints_with_visitor(&mut input, ints.len(), &mut visitor)?;
    assert_eq!(ints, &read[..]);
    assert_eq!(len, input.get_file_pointer()?);
  }
  dir.delete_file("tmp")?;
  Ok(())
}

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_crash() -> Result<()> {
  let mut random = random();
  let itrs = at_least(&mut random, 100);

  for _ in 0..itrs {
    let dir = new_directory_shared(&mut random)?;

    let config = new_index_writer_config(&mut random);
    let iw = IndexWriter::new(dir.clone(), config)?;

    for _d in 0..20_000 {
      let mut doc = Document::new();
      doc.add(IntPoint::new("foo", [0])?);
      doc.add(SortedNumericDocValuesField::new("bar", 0));
      iw.add_document(doc)?;
    }
    iw.close()?;
  }

  Ok(())
}

struct IntersectVisitorMock<'a> {
  i: usize,
  read: &'a mut Vec<i32>,
}
impl IntersectVisitor for IntersectVisitorMock<'_> {
  fn visit(&mut self, doc_id: i32) -> Result<()> {
    self.read[self.i] = doc_id;
    self.i += 1;
    Ok(())
  }

  fn visit_with_packed_value(&mut self, _doc_id: i32, _packed_value: &[u8]) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }

  fn compare(&self, _min_packed_value: &[u8], _max_packed_value: &[u8]) -> Result<Relation> {
    Err(LuceneError::unsupported_operation(""))
  }
}
