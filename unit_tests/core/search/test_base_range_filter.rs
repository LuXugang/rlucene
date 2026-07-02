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
use crate::core::document::double_point::DoublePoint;
use crate::core::document::field::Store;
use crate::core::document::float_point::FloatPoint;
use crate::core::document::int_point::IntPoint;
use crate::core::document::long_point::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::index_writer_config::OpenMode;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::standard_directory_reader::StandardDirectoryReaderType;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::DefaultCRReader;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_string_field,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestBaseRangeFilter;

struct TestIndex {
  max_r: i32,
  min_r: i32,
  allow_negative_random_ints: bool,
  index: Arc<DirEnum>,
}

impl TestIndex {
  fn new<R>(
    random: &mut R,
    min_r: i32,
    max_r: i32,
    allow_negative_random_ints: bool,
  ) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    Ok(Self {
      min_r,
      max_r,
      allow_negative_random_ints,
      index: new_directory_shared(random)?,
    })
  }
}
pub fn pad(n: i32) -> String {
  let mut b = String::with_capacity(40);
  let mut p = "0";
  let mut n = n;

  if n < 0 {
    p = "-";
    n = i32::MAX + n + 1;
  }

  b.push_str(p);

  let s = n.to_string();
  for _ in s.len()..=i32::MAX.to_string().len() {
    b.push('0');
  }
  b.push_str(&s);

  b
}
pub fn set_up<R>(random: &mut R) -> Result<(i32, i32, i32, i32, DefaultCRReader, DefaultCRReader)>
where
  R: Rng + ?Sized,
{
  let min_id = 0;
  let max_id = at_least(random, 500);
  let mut signed_index_dir = TestIndex::new(random, i32::MAX, i32::MIN, true)?;
  let mut unsigned_index_dir = TestIndex::new(random, i32::MAX, 0, false)?;

  let signed_index_reader = build(random, &mut signed_index_dir, min_id, max_id)?;
  let unsigned_index_reader = build(random, &mut unsigned_index_dir, min_id, max_id)?;

  Ok((
    min_id,
    max_id,
    signed_index_dir.min_r,
    signed_index_dir.max_r,
    signed_index_reader,
    unsigned_index_reader,
  ))
}
fn build<R>(
  random: &mut R,
  index: &mut TestIndex,
  min_id: i32,
  max_id: i32,
) -> Result<StandardDirectoryReaderType<DirEnum>>
where
  R: Rng + ?Sized,
{
  loop {
    let analyzer = MockAnalyzer::new(random);
    let mut config = new_index_writer_config_with_analyzer(random, analyzer)?;
    config
      .set_open_mode(OpenMode::Create)
      .set_max_buffered_docs(TestUtil::next_int(random, 50, 1000))
      .set_merge_policy(new_log_merge_policy(random)?);
    let writer = RandomIndexWriter::with_config(random, index.index.clone(), config);
    TestUtil::reduce_open_files(&writer.w)?;
    let mut field_to_type = HashMap::new();

    let mut min_count = 0;
    let mut max_count = 0;

    for d in min_id..=max_id {
      let id = pad(d);
      let r = if index.allow_negative_random_ints {
        random.random::<i32>()
      } else {
        random.random_range(0..i32::MAX)
      };
      let rand = pad(r);

      if index.max_r < r {
        index.max_r = r;
        max_count = 1;
      } else if index.max_r == r {
        max_count += 1;
      }

      if r < index.min_r {
        index.min_r = r;
        min_count = 1;
      } else if r == index.min_r {
        min_count += 1;
      }

      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "id",
        id.clone(),
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(SortedDocValuesField::new(
        "id",
        BytesRef::from_bytes(id.into_bytes()),
      ));
      doc.add(IntPoint::new("id_int", [d])?);
      doc.add(NumericDocValuesField::new("id_int", d as i64));
      doc.add(FloatPoint::new("id_float", [d as f32])?);
      doc.add(NumericDocValuesField::new(
        "id_float",
        (d as f32).to_bits() as i32 as i64,
      ));
      doc.add(LongPoint::new("id_long", [d as i64])?);
      doc.add(NumericDocValuesField::new("id_long", d as i64));
      doc.add(DoublePoint::new("id_double", [d as f64])?);
      doc.add(NumericDocValuesField::new(
        "id_double",
        (d as f64).to_bits() as i64,
      ));

      doc.add(new_string_field(
        random,
        "rand",
        rand.clone(),
        Store::Yes,
        &mut field_to_type,
      )?);
      doc.add(SortedDocValuesField::new(
        "rand",
        BytesRef::from_bytes(rand.into_bytes()),
      ));
      doc.add(new_string_field(
        random,
        "body",
        "body",
        Store::No,
        &mut field_to_type,
      )?);
      doc.add(SortedDocValuesField::new(
        "body",
        BytesRef::from_bytes(b"body".to_vec()),
      ));
      writer.add_document(random, doc)?;
    }

    if min_count == 1 && max_count == 1 {
      let reader = writer.get_reader(random)?;
      writer.close(random)?;
      return Ok(reader);
    }
    // TODO IMPORTANT delete_all实现后 这里的 loop 需要调整
    writer.w.delete_all()?;
  }
}
#[test]
fn test_pad() {
  let tests = [
    -9_999_999,
    -99_560,
    -100,
    -3,
    -1,
    0,
    3,
    9,
    10,
    1000,
    999_999_999,
  ];

  for i in 0..tests.len() - 1 {
    let a = tests[i];
    let b = tests[i + 1];
    let aa = pad(a);
    let bb = pad(b);
    let label = format!("{a}:{aa} vs {b}:{bb}");

    assert_eq!(aa.len(), bb.len(), "length of {label}");
    assert!(aa < bb, "compare less than {label}");
  }
}
