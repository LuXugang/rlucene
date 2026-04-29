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
use crate::core::document::binary_point::BinaryPoint;
use crate::core::document::document::Document;
use crate::core::document::double_point::DoublePoint;
use crate::core::document::field::Store::No;
use crate::core::document::field_type::FieldType;
use crate::core::document::float_point::FloatPoint;
use crate::core::document::int_point::IntPoint;
use crate::core::document::long_point::LongPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::point_values::{MAX_INDEX_DIMENSIONS, MAX_NUM_BYTES};
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::point_range_query::{PointRangeBase, PointRangeQuery};
use crate::core::search::query::Query;
use crate::core::search::score_mode::ScoreMode::CompleteNoScores;
use crate::core::util::bits::Bits;
use crate::core::util::bkd::bkd_config::BKDConfig;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::numeric_utils::NumericUtils;
use crate::core::util::{CoreHelper, SliceCopyOps};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::fixed_bit_set_collector::FixedBitSetCollector;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, create_temp_dir_with_prefix, new_directory_shared, new_fs_directory,
  new_index_writer_config, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_searcher_with_wrap, new_string_field, random, random_from_seed,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::vec;

#[allow(dead_code)] // for quick search
pub struct TestPointQueries;

#[test]
fn test_basic_ints() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(IntPoint::new("point", [-7])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(IntPoint::new("point", [0])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(IntPoint::new("point", [3])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = IndexSearcher::from_cr(r)?;

  assert_eq!(
    2,
    searcher.count(IntPoint::new_range_query("point", -8, 1)?)?
  );

  assert_eq!(
    3,
    searcher.count(IntPoint::new_range_query("point", -7, 3)?)?
  );

  assert_eq!(1, searcher.count(IntPoint::new_exact_query("point", -7)?)?);

  assert_eq!(0, searcher.count(IntPoint::new_exact_query("point", -6)?)?);
  w.close()?;
  Ok(())
}
#[test]
fn test_basic_floats() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [-7.0f32])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [0.0f32])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [3.0f32])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = IndexSearcher::from_cr(r)?;

  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query("point", -8.0f32, 1.0f32)?)?
  );

  assert_eq!(
    3,
    searcher.count(FloatPoint::new_range_query("point", -7.0f32, 3.0f32)?)?
  );

  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", -7.0f32)?)?
  );

  assert_eq!(
    0,
    searcher.count(FloatPoint::new_exact_query("point", -6.0f32)?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_basic_longs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("point", [-7i64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("point", [0i64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("point", [3i64])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = IndexSearcher::from_cr(r)?;

  assert_eq!(
    2,
    searcher.count(LongPoint::new_range_query("point", -8i64, 1i64)?)?
  );

  assert_eq!(
    3,
    searcher.count(LongPoint::new_range_query("point", -7i64, 3i64)?)?
  );

  assert_eq!(
    1,
    searcher.count(LongPoint::new_exact_query("point", -7i64)?)?
  );

  assert_eq!(
    0,
    searcher.count(LongPoint::new_exact_query("point", -6i64)?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_basic_doubles() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [-7.0f64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [0.0f64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [3.0f64])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = IndexSearcher::from_cr(r)?;

  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query("point", -8.0f64, 1.0f64)?)?
  );

  assert_eq!(
    3,
    searcher.count(DoublePoint::new_range_query("point", -7.0f64, 3.0f64)?)?
  );

  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", -7.0f64)?)?
  );

  assert_eq!(
    0,
    searcher.count(DoublePoint::new_exact_query("point", -6.0f64)?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_crazy_doubles() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [f64::NEG_INFINITY])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [-0.0f64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [0.0f64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [f64::MIN_POSITIVE])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [f64::MAX])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [f64::INFINITY])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("point", [f64::NAN])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = IndexSearcher::from_cr(r)?;

  // exact queries
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", f64::NEG_INFINITY)?)?
  );
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", -0.0f64)?)?
  );
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", 0.0f64)?)?
  );
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", f64::MIN_POSITIVE)?)?
  );
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", f64::MAX)?)?
  );
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", f64::INFINITY)?)?
  );
  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("point", f64::NAN)?)?
  );

  // set query
  let _set = [
    f64::MAX,
    f64::NAN,
    0.0f64,
    f64::NEG_INFINITY,
    f64::MIN_POSITIVE,
    -0.0f64,
    f64::INFINITY,
  ];
  // TODO IMPORTANT new_set_query未实现
  // assert_eq!(
  //     7,
  //     searcher.count(DoublePoint::new_set_query("point", &set)?)?
  // );

  // ranges
  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query(
      "point",
      f64::NEG_INFINITY,
      -0.0f64
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query("point", -0.0f64, 0.0f64)?)?
  );

  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query(
      "point",
      0.0f64,
      f64::MIN_POSITIVE
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query(
      "point",
      f64::MIN_POSITIVE,
      f64::MAX
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query(
      "point",
      f64::MAX,
      f64::INFINITY
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(DoublePoint::new_range_query(
      "point",
      f64::INFINITY,
      f64::NAN
    )?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_crazy_floats() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [f32::NEG_INFINITY])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [-0.0f32])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [0.0f32])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [f32::MIN_POSITIVE])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [f32::MAX])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [f32::INFINITY])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("point", [f32::NAN])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = IndexSearcher::from_cr(r)?;

  // exact queries
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", f32::NEG_INFINITY)?)?
  );
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", -0.0f32)?)?
  );
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", 0.0f32)?)?
  );
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", f32::MIN_POSITIVE)?)?
  );
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", f32::MAX)?)?
  );
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", f32::INFINITY)?)?
  );
  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("point", f32::NAN)?)?
  );

  // set query
  let _set = [
    f32::MAX,
    f32::NAN,
    0.0f32,
    f32::NEG_INFINITY,
    f32::MIN_POSITIVE,
    -0.0f32,
    f32::INFINITY,
  ];
  // TODO IMPORTANT new_set_query 未实现
  // assert_eq!(
  //     7,
  //     searcher.count(FloatPoint::new_set_query("point", &set)?)?
  // );

  // ranges
  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query(
      "point",
      f32::NEG_INFINITY,
      -0.0f32
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query("point", -0.0f32, 0.0f32)?)?
  );

  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query(
      "point",
      0.0f32,
      f32::MIN_POSITIVE
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query(
      "point",
      f32::MIN_POSITIVE,
      f32::MAX
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query(
      "point",
      f32::MAX,
      f32::INFINITY
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(FloatPoint::new_range_query(
      "point",
      f32::INFINITY,
      f32::NAN
    )?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_all_equal() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_random_longs_tiny() -> Result<()> {
  let mut random = random();
  do_test_random_longs(&mut random, 10)
}
#[test]
fn test_random_longs_medium() -> Result<()> {
  let mut random = random();
  do_test_random_longs(&mut random, 1000)
}

#[test]
fn test_random_longs_big() -> Result<()> {
  let mut random = random();
  do_test_random_longs(&mut random, 20_000)
}

fn do_test_random_longs<R>(random: &mut R, count: i32) -> Result<()>
where
  R: Rng + ?Sized,
{
  let num_values = TestUtil::next_int(random, count, count * 2) as usize;
  let mut values = vec![0i64; num_values];
  let mut ids = vec![0; num_values];
  let single_valued = random.random_bool(0.5);
  let same_value_pct = random.random_range(0..100);

  let mut id = 0;
  for ord in 0..num_values {
    if ord > 0 && random.random_range(0..100) < same_value_pct {
      values[ord] = values[random.random_range(0..ord)];
    } else {
      values[ord] = random_value(random);
    }

    ids[ord] = id;
    if single_valued || random.random_range(0..2) == 1 {
      id += 1;
    }
  }

  verify_longs(random, &values, &ids)
}
#[test]
fn test_long_encode() -> Result<()> {
  let mut random = random();

  for _ in 0..10_000 {
    let v: i64 = random.random();
    let mut tmp = [0u8; 8];

    NumericUtils::long_to_sortable_bytes(v, &mut tmp, 0);
    let v2 = NumericUtils::sortable_bytes_to_long(&tmp, 0);

    assert_eq!(v, v2, "got bytes={:?}", tmp);
  }

  Ok(())
}
fn verify_longs<R>(random: &mut R, values: &[i64], ids: &[i32]) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut iwc = new_index_writer_config(random);

  let mbd = iwc.get_max_buffered_docs();
  if mbd != -1 && mbd < (values.len() / 100) as i32 {
    iwc.set_max_buffered_docs((values.len() / 100) as i32);
  }

  let dir = if values.len() > 100000 {
    new_fs_directory(
      random,
      create_temp_dir_with_prefix("test_random_longs_big")?,
    )?
  } else {
    new_directory_shared(random)?
  };

  let missing_pct = if random.random_bool(0.5) {
    0
  } else {
    random.random_range(0..100)
  };
  let deleted_pct = random.random_range(0..100);

  let mut missing = bit_set::BitSet::new();
  let mut deleted = bit_set::BitSet::new();
  let mut doc: Option<Document> = None;
  let mut last_id = -1;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let w = IndexWriter::new(dir.clone(), iwc)?;
  #[allow(clippy::needless_range_loop)]
  for ord in 0..values.len() {
    let id = ids[ord];
    let id_index = id as usize;

    if id != last_id {
      if random.random_range(0..100) < missing_pct {
        missing.insert(id_index);
      }

      if let Some(doc) = doc.take() {
        w.add_document(doc)?;
        if random.random_range(0..100) < deleted_pct {
          let id_to_delete = random.random_range(0..id) as usize;
          w.delete_documents_with_terms(vec![Term::from_text("id", id_to_delete.to_string())])?;
          deleted.insert(id_to_delete);
        }
      }

      let mut new_doc = Document::new();
      new_doc.add(new_string_field(
        random,
        "id",
        id.to_string(),
        No,
        &mut field_to_type,
      )?);
      new_doc.add(NumericDocValuesField::new("id", id as i64));
      doc = Some(new_doc);
      last_id = id;
    }

    if !missing.contains(id_index) {
      let value = values[id_index];
      doc
        .as_mut()
        .expect("document should be initialized before adding point values")
        .add(LongPoint::new("sn_value", [value])?);

      let mut bytes = vec![0u8; 8];
      NumericUtils::long_to_sortable_bytes(value, &mut bytes, 0);
      doc
        .as_mut()
        .expect("document should be initialized before adding point values")
        .add(BinaryPoint::new("ss_value", [bytes])?);
    }
  }

  if let Some(doc) = doc {
    w.add_document(doc)?;
  }

  if random.random_bool(0.5) {
    w.force_merge(1)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let max_doc = r.max_doc()?;
  w.close()?;

  let num_threads = TestUtil::next_int(random, 2, 5);
  let iters = at_least(random, 100);
  let failed = AtomicBool::new(false);

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();

    for _ in 0..num_threads {
      let dir = dir.clone();
      let failed = &failed;
      let missing = &missing;
      let deleted = &deleted;
      let seed = random.next_u64();

      handles.push(scope.spawn(move || -> Result<()> {
        let mut random = random_from_seed(seed);
        let r = Arc::new(directory_reader::open(dir)?);
        let searcher = new_searcher_with_reader(r.clone())?;

        for _ in 0..iters {
          if failed.load(Ordering::Relaxed) {
            break;
          }

          let mut lower = random_value(&mut random);
          let mut upper = random_value(&mut random);
          if upper < lower {
            std::mem::swap(&mut lower, &mut upper);
          }

          let query: Query = if random.random_bool(0.5) {
            LongPoint::new_range_query("sn_value", lower, upper)?.into()
          } else {
            let mut lower_bytes = vec![0u8; 8];
            let mut upper_bytes = vec![0u8; 8];
            NumericUtils::long_to_sortable_bytes(lower, &mut lower_bytes, 0);
            NumericUtils::long_to_sortable_bytes(upper, &mut upper_bytes, 0);
            BinaryPoint::new_range_query("ss_value", lower_bytes, upper_bytes)?.into()
          };

          let hits = searcher
            .search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))?;

          let mut doc_id_to_id = MultiDocValues::get_numeric_values(r.clone(), "id")?
            .ok_or_else(|| LuceneError::illegal_state("missing id numeric doc values"))?;

          loop {
            let doc_id = doc_id_to_id.next_doc()?;
            if doc_id == NO_MORE_DOCS {
              break;
            }
            let id = doc_id_to_id.long_value()? as usize;
            let expected = !missing.contains(id)
              && !deleted.contains(id)
              && values[id] >= lower
              && values[id] <= upper;

            if hits.get(doc_id as usize)? != expected {
              failed.store(true, Ordering::Relaxed);
              return Err(LuceneError::illegal_state(format!(
                "id={} docID={} value={} range={} TO {} expected {} but got {}",
                id,
                doc_id,
                values[id],
                lower,
                upper,
                expected,
                hits.get(doc_id as usize)?
              )));
            }
          }
        }

        Ok(())
      }));
    }

    for handle in handles {
      handle
        .join()
        .map_err(|_| LuceneError::illegal_state("verify_longs query thread panicked"))??;
    }

    Ok(())
  })?;

  Ok(())
}
#[test]
fn test_random_binary_tiny() -> Result<()> {
  let mut random = random();
  do_test_random_binary(&mut random, 10)
}

#[test]
fn test_random_binary_medium() -> Result<()> {
  let mut random = random();
  do_test_random_binary(&mut random, 1000)
}

fn do_test_random_binary<R>(random: &mut R, count: i32) -> Result<()>
where
  R: Rng + ?Sized,
{
  let num_values = TestUtil::next_int(random, count, count * 2) as usize;
  let num_bytes_per_dim = TestUtil::next_int(random, 2, MAX_NUM_BYTES as i32) as usize;
  let num_dims = TestUtil::next_int(random, 1, MAX_INDEX_DIMENSIONS as i32) as usize;
  let same_value_pct = random.random_range(0..100);

  let mut doc_values: Vec<Vec<Vec<u8>>> = Vec::with_capacity(num_values);
  let single_valued = random.random_bool(0.5);
  let mut ids = vec![0; num_values];

  let mut id = 0;
  for ord in 0..num_values {
    if ord > 0 && random.random_range(0..100) < same_value_pct {
      doc_values.push(doc_values[random.random_range(0..ord)].clone());
    } else {
      let mut values = Vec::with_capacity(num_dims);
      for _ in 0..num_dims {
        let mut value = vec![0u8; num_bytes_per_dim];
        random.fill_bytes(&mut value);
        values.push(value);
      }
      doc_values.push(values);
    }

    ids[ord] = id;
    if single_valued || random.random_range(0..2) == 1 {
      id += 1;
    }
  }

  verify_binary(random, &doc_values, &ids, num_bytes_per_dim)
}

fn verify_binary<R>(
  random: &mut R,
  doc_values: &[Vec<Vec<u8>>],
  ids: &[i32],
  num_bytes_per_dim: usize,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let mut iwc = new_index_writer_config(random);

  let num_dims = doc_values[0].len();
  let bytes_per_dim = doc_values[0][0].len();

  let mbd = iwc.get_max_buffered_docs();
  if mbd != -1 && mbd < (doc_values.len() / 100) as i32 {
    iwc.set_max_buffered_docs((doc_values.len() / 100) as i32);
  }
  // TODO setCodec未实现
  let dir = if doc_values.len() > 100000 {
    new_fs_directory(
      random,
      create_temp_dir_with_prefix("test_random_binary_big")?,
    )?
  } else {
    new_directory_shared(random)?
  };
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let num_values = doc_values.len();
  let missing_pct = random.random_range(0..100);
  let deleted_pct = random.random_range(0..100);

  let mut missing = bit_set::BitSet::new();
  let mut deleted = bit_set::BitSet::new();

  let mut doc: Option<Document> = None;
  let mut last_id = -1;
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  for ord in 0..num_values {
    let id = ids[ord];
    let id_index = id as usize;

    if id != last_id {
      if random.random_range(0..100) < missing_pct {
        missing.insert(id_index);
      }

      if let Some(doc) = doc.take() {
        w.add_document(doc)?;
        if random.random_range(0..100) < deleted_pct {
          let id_to_delete = random.random_range(0..id) as usize;
          w.delete_documents_with_terms(vec![Term::from_text("id", id_to_delete.to_string())])?;
          deleted.insert(id_to_delete);
        }
      }

      let mut new_doc = Document::new();
      new_doc.add(new_string_field(
        random,
        "id",
        id.to_string(),
        No,
        &mut field_to_type,
      )?);
      new_doc.add(NumericDocValuesField::new("id", id as i64));
      doc = Some(new_doc);
      last_id = id;
    }

    if !missing.contains(id as usize) {
      doc
        .as_mut()
        .expect("document should be initialized before adding point values")
        .add(BinaryPoint::new("value", doc_values[ord].clone())?);
    }
  }

  if let Some(doc) = doc {
    w.add_document(doc)?;
  }

  if random.random_bool(0.5) {
    w.force_merge(1)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let max_doc = r.max_doc()?;
  w.close()?;

  let num_threads = TestUtil::next_int(random, 2, 5);
  let iters = at_least(random, 100);

  let failed = AtomicBool::new(false);

  thread::scope(|scope| -> Result<()> {
    let mut handles = Vec::new();

    for _ in 0..num_threads {
      let dir = dir.clone();
      let failed = &failed;
      let missing = &missing;
      let deleted = &deleted;
      let seed = random.next_u64();

      handles.push(scope.spawn(move || -> Result<()> {
        let mut random = random_from_seed(seed);
        let r = Arc::new(directory_reader::open(dir)?);
        let searcher = new_searcher_with_reader(r.clone())?;

        for _ in 0..iters {
          if failed.load(Ordering::Relaxed) {
            break;
          }

          let mut lower = vec![vec![0u8; bytes_per_dim]; num_dims];
          let mut upper = vec![vec![0u8; bytes_per_dim]; num_dims];

          for dim in 0..num_dims {
            random.fill_bytes(&mut lower[dim]);
            random.fill_bytes(&mut upper[dim]);

            if lower[dim].as_slice() > upper[dim].as_slice() {
              std::mem::swap(&mut lower[dim], &mut upper[dim]);
            }
          }

          let query = BinaryPoint::new_range_query_multi_dim("value", &lower, &upper)?;
          let hits = searcher
            .search_with_collector_manager(query, &FixedBitSetCollector::create_manager(max_doc))?;

          let mut expected = bit_set::BitSet::new();
          for ord in 0..num_values {
            let id = ids[ord] as usize;
            if !missing.contains(id)
              && !deleted.contains(id)
              && matches(num_bytes_per_dim, &lower, &upper, &doc_values[ord])
            {
              expected.insert(id);
            }
          }

          let mut doc_id_to_id = MultiDocValues::get_numeric_values(r.clone(), "id")?
            .ok_or_else(|| LuceneError::illegal_state("missing id numeric doc values"))?;

          let mut fail_count = 0;
          loop {
            let doc_id = doc_id_to_id.next_doc()?;
            if doc_id == NO_MORE_DOCS {
              break;
            }
            let id = doc_id_to_id.long_value()? as usize;
            if hits.get(doc_id as usize)? != expected.contains(id) {
              fail_count += 1;
            }
          }

          if fail_count != 0 {
            failed.store(true, Ordering::Relaxed);
            return Err(LuceneError::illegal_state(format!(
              "{} hits were wrong",
              fail_count
            )));
          }
        }

        Ok(())
      }));
    }

    for handle in handles {
      handle
        .join()
        .map_err(|_| LuceneError::illegal_state("verify_binary query thread panicked"))??;
    }

    Ok(())
  })?;

  Ok(())
}
fn matches(bytes_per_dim: usize, lower: &[Vec<u8>], upper: &[Vec<u8>], value: &[Vec<u8>]) -> bool {
  for dim in 0..value.len() {
    if value[dim][0..bytes_per_dim] < lower[dim][0..bytes_per_dim]
      || value[dim][0..bytes_per_dim] > upper[dim][0..bytes_per_dim]
    {
      return false;
    }
  }
  true
}

fn random_value<R>(random: &mut R) -> i64
where
  R: Rng + ?Sized,
{
  match random.random_range(0..10) {
    0 => i64::MIN,
    1 => i64::MAX,
    2 => 0,
    3 => -1,
    4 => 1,
    _ => random.random(),
  }
}
#[test]
fn test_min_max_long() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MIN])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MAX])?);
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    1,
    searcher.count(LongPoint::new_range_query("value", i64::MIN, 0i64)?)?
  );

  assert_eq!(
    1,
    searcher.count(LongPoint::new_range_query("value", 0i64, i64::MAX)?)?
  );

  assert_eq!(
    2,
    searcher.count(LongPoint::new_range_query("value", i64::MIN, i64::MAX)?)?
  );
  Ok(())
}
fn to_utf8(s: &str) -> Vec<u8> {
  s.as_bytes().to_vec()
}
// Right zero pads
fn to_utf8_padded(s: &str, length: usize) -> Result<Vec<u8>> {
  let bytes = s.as_bytes();

  if length < bytes.len() {
    return Err(LuceneError::illegal_argument(format!(
      "length={} but string's UTF8 bytes has length={}",
      length,
      bytes.len()
    )));
  }

  let mut result = vec![0u8; length];
  result.copy_from(&bytes[0..bytes.len()], 0);
  Ok(result)
}

#[test]
fn test_basic_sorted_set() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(BinaryPoint::new("value", [to_utf8("abc")])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(BinaryPoint::new("value", [to_utf8("def")])?);
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    1,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8("aaa"),
      to_utf8("bbb")
    )?)?
  );

  assert_eq!(
    1,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8_padded("c", 3)?,
      to_utf8_padded("e", 3)?
    )?)?
  );

  assert_eq!(
    2,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8_padded("a", 3)?,
      to_utf8_padded("z", 3)?
    )?)?
  );

  assert_eq!(
    1,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8_padded("", 3)?,
      to_utf8("abc")
    )?)?
  );

  assert_eq!(
    1,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8_padded("a", 3)?,
      to_utf8("abc")
    )?)?
  );

  assert_eq!(
    0,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8_padded("a", 3)?,
      to_utf8("abb")
    )?)?
  );

  assert_eq!(
    1,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8("def"),
      to_utf8("zzz")
    )?)?
  );

  assert_eq!(
    1,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8("def"),
      to_utf8_padded("z", 3)?
    )?)?
  );

  assert_eq!(
    0,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8("deg"),
      to_utf8_padded("z", 3)?
    )?)?
  );
  w.close()?;
  Ok(())
}

#[test]
fn test_long_min_max_numeric() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MIN])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MAX])?);
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    2,
    searcher.count(LongPoint::new_range_query("value", i64::MIN, i64::MAX)?)?
  );

  assert_eq!(
    1,
    searcher.count(LongPoint::new_range_query("value", i64::MIN, i64::MAX - 1)?)?
  );

  assert_eq!(
    1,
    searcher.count(LongPoint::new_range_query("value", i64::MIN + 1, i64::MAX)?)?
  );

  assert_eq!(
    0,
    searcher.count(LongPoint::new_range_query(
      "value",
      i64::MIN + 1,
      i64::MAX - 1
    )?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_long_min_max_sorted_set() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MIN])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MAX])?);
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    2,
    searcher.count(LongPoint::new_range_query("value", i64::MIN, i64::MAX)?)?
  );

  assert_eq!(
    1,
    searcher.count(LongPoint::new_range_query("value", i64::MIN, i64::MAX - 1)?)?
  );

  assert_eq!(
    1,
    searcher.count(LongPoint::new_range_query("value", i64::MIN + 1, i64::MAX)?)?
  );

  assert_eq!(
    0,
    searcher.count(LongPoint::new_range_query(
      "value",
      i64::MIN + 1,
      i64::MAX - 1
    )?)?
  );

  w.close()?;
  Ok(())
}

#[test]
fn test_sorted_set_no_ords_match() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(BinaryPoint::new("value", [to_utf8("a")])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(BinaryPoint::new("value", [to_utf8("z")])?);
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    0,
    searcher.count(BinaryPoint::new_range_query(
      "value",
      to_utf8("m"),
      to_utf8("m")
    )?)?
  );

  w.close()?;
  Ok(())
}

#[test]
fn test_numeric_no_values_match() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(SortedNumericDocValuesField::new("value", 17));
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(SortedNumericDocValuesField::new("value", 22));
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;
  let searcher = IndexSearcher::from_cr(r)?;

  assert_eq!(
    0,
    searcher.count(LongPoint::new_range_query("value", 17i64, 13i64)?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_no_docs() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  w.add_document(Document::new())?;

  let r = w.get_reader()?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    0,
    searcher.count(LongPoint::new_range_query("value", 17i64, 13i64)?)?
  );

  w.close()?;
  Ok(())
}

#[test]
fn test_wrong_num_dims() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("value", [i64::MIN])?);
    w.add_document(doc)?;
  }

  let r = w.get_reader()?;

  // no wrapping, else the exc might happen in executor thread:
  let searcher = IndexSearcher::from_cr(r)?;

  let point = [vec![0u8; 8], vec![0u8; 8]];

  let err = searcher.count(BinaryPoint::new_range_query_multi_dim(
    "value",
    point.as_ref(),
    point.as_ref(),
  )?);

  assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
  if let Err(LuceneError::IllegalArgument(msg)) = err {
    assert_eq!(
      "field=\"value\" was indexed with numIndexDimensions=1 but this query has numDims=2",
      msg.to_string()
    );
  }
  w.close()?;
  Ok(())
}

#[test]
fn test_all_point_docs_were_deleted_and_then_merged_again() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(StringField::from_string("id", "0", No)?);
    doc.add(LongPoint::new("value", [0i64])?);
    w.add_document(doc)?;
  }

  // Add document that won't be deleted to avoid IW dropping
  // segment below since it's 100% deleted:
  w.add_document(Document::new())?;
  w.commit()?;

  // Need another segment so we invoke BKDWriter.merge
  {
    let mut doc = Document::new();
    doc.add(StringField::from_string("id", "0", No)?);
    doc.add(LongPoint::new("value", [0i64])?);
    w.add_document(doc)?;
  }
  w.add_document(Document::new())?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  w.force_merge(1)?;

  {
    let mut doc = Document::new();
    doc.add(StringField::from_string("id", "0", No)?);
    doc.add(LongPoint::new("value", [0i64])?);
    w.add_document(doc)?;
  }
  w.add_document(Document::new())?;

  w.delete_documents_with_terms(vec![Term::from_text("id", "0")])?;
  w.force_merge(1)?;

  w.close()?;
  Ok(())
}
#[test]
fn test_exact_points() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  {
    let mut doc = Document::new();
    doc.add(LongPoint::new("long", [5i64])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(IntPoint::new("int", [42i32])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(FloatPoint::new("float", [2.0f32])?);
    w.add_document(doc)?;
  }

  {
    let mut doc = Document::new();
    doc.add(DoublePoint::new("double", [1.0f64])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(1, searcher.count(IntPoint::new_exact_query("int", 42i32)?)?);
  assert_eq!(0, searcher.count(IntPoint::new_exact_query("int", 41i32)?)?);

  assert_eq!(
    1,
    searcher.count(LongPoint::new_exact_query("long", 5i64)?)?
  );
  assert_eq!(
    0,
    searcher.count(LongPoint::new_exact_query("long", -1i64)?)?
  );

  assert_eq!(
    1,
    searcher.count(FloatPoint::new_exact_query("float", 2.0f32)?)?
  );
  assert_eq!(
    0,
    searcher.count(FloatPoint::new_exact_query("float", 1.0f32)?)?
  );

  assert_eq!(
    1,
    searcher.count(DoublePoint::new_exact_query("double", 1.0f64)?)?
  );
  assert_eq!(
    0,
    searcher.count(DoublePoint::new_exact_query("double", 2.0f64)?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
  // ints
  assert_eq!(
    "field:[1 TO 2]",
    IntPoint::new_range_query("field", 1i32, 2i32)?.to_string("")?
  );
  assert_eq!(
    "field:[-2 TO 1]",
    IntPoint::new_range_query("field", -2i32, 1i32)?.to_string("")?
  );

  // longs
  assert_eq!(
    "field:[1099511627776 TO 2199023255552]",
    LongPoint::new_range_query("field", 1i64 << 40, 1i64 << 41)?.to_string("")?
  );
  assert_eq!(
    "field:[-5 TO 6]",
    LongPoint::new_range_query("field", -5i64, 6i64)?.to_string("")?
  );

  // floats
  assert_eq!(
    "field:[1.3 TO 2.5]",
    FloatPoint::new_range_query("field", 1.3f32, 2.5f32)?.to_string("")?
  );
  assert_eq!(
    "field:[-2.9 TO 1]",
    FloatPoint::new_range_query("field", -2.9f32, 1.0f32)?.to_string("")?
  );

  // doubles
  assert_eq!(
    "field:[1.3 TO 2.5]",
    DoublePoint::new_range_query("field", 1.3f64, 2.5f64)?.to_string("")?
  );
  assert_eq!(
    "field:[-2.9 TO 1]",
    DoublePoint::new_range_query("field", -2.9f64, 1.0f64)?.to_string("")?
  );

  // n-dimensional double
  assert_eq!(
    "field:[1.3 TO 2.5],[-2.9 TO 1]",
    DoublePoint::new_range_query_n("field", &[1.3f64, -2.9f64], &[2.5f64, 1.0f64])?
      .to_string("")?
  );

  Ok(())
}
fn to_array(_values_set: &std::collections::HashSet<i32>) -> Vec<i32> {
  // TODO
  Vec::new()
}

fn random_int_value(_min: Option<i32>, _max: Option<i32>) -> i32 {
  // TODO
  0
}

#[test]
fn test_random_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}
fn new_multi_dim_int_set_query(
  _field: &str,
  _num_dims: usize,
  _values_in: &[i32],
) -> Result<Query> {
  // TODO
  Err(LuceneError::unsupported_operation(
    "new_multi_dim_int_set_query not implement",
  ))
}

#[test]
fn test_basic_multi_dim_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_basic_multi_value_multi_dim_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_many_equal_values_multi_dim_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_invalid_multi_dim_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_basic_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_point_int_set_boxed() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_basic_multi_valued_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_empty_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_point_in_set_query_many_equal_values() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_point_range_query_many_equal_values() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let iwc = new_index_writer_config(&mut random);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let cardinality: i32 = random.random_range(2..20);

  let mut zero_count = 0;
  let mut one_count = 0;

  for _ in 0..10_000 {
    let x: i32 = random.random_range(0..cardinality as usize) as i32;
    if x == 0 {
      zero_count += 1;
    } else if x == 1 {
      one_count += 1;
    }

    let mut doc = Document::new();
    doc.add(IntPoint::new("int", [x])?);
    doc.add(LongPoint::new("long", [x as i64])?);
    doc.add(FloatPoint::new("float", [x as f32])?);
    doc.add(DoublePoint::new("double", [x as f64])?);
    doc.add(BinaryPoint::new("bytes", [vec![x as u8]])?);
    w.add_document(doc)?;
  }

  let r = directory_reader::open_from_writer(&w)?;
  let searcher = new_searcher_with_wrap(&mut random, r, false)?;

  assert_eq!(
    zero_count,
    searcher.count(IntPoint::new_range_query("int", 0, 0)?)?
  );
  assert_eq!(
    one_count,
    searcher.count(IntPoint::new_range_query("int", 1, 1)?)?
  );
  assert_eq!(
    zero_count + one_count,
    searcher.count(IntPoint::new_range_query("int", 0, 1)?)?
  );
  assert_eq!(
    10_000 - zero_count - one_count,
    searcher.count(IntPoint::new_range_query("int", 2, cardinality)?)?
  );

  assert_eq!(
    zero_count,
    searcher.count(LongPoint::new_range_query("long", 0i64, 0i64)?)?
  );
  assert_eq!(
    one_count,
    searcher.count(LongPoint::new_range_query("long", 1i64, 1i64)?)?
  );
  assert_eq!(
    zero_count + one_count,
    searcher.count(LongPoint::new_range_query("long", 0i64, 1i64)?)?
  );
  assert_eq!(
    10_000 - zero_count - one_count,
    searcher.count(LongPoint::new_range_query(
      "long",
      2i64,
      cardinality as i64
    )?)?
  );

  assert_eq!(
    zero_count,
    searcher.count(FloatPoint::new_range_query("float", 0.0f32, 0.0f32)?)?
  );
  assert_eq!(
    one_count,
    searcher.count(FloatPoint::new_range_query("float", 1.0f32, 1.0f32)?)?
  );
  assert_eq!(
    zero_count + one_count,
    searcher.count(FloatPoint::new_range_query("float", 0.0f32, 1.0f32)?)?
  );
  assert_eq!(
    10_000 - zero_count - one_count,
    searcher.count(FloatPoint::new_range_query(
      "float",
      2.0f32,
      cardinality as f32
    )?)?
  );

  assert_eq!(
    zero_count,
    searcher.count(DoublePoint::new_range_query("double", 0.0f64, 0.0f64)?)?
  );
  assert_eq!(
    one_count,
    searcher.count(DoublePoint::new_range_query("double", 1.0f64, 1.0f64)?)?
  );
  assert_eq!(
    zero_count + one_count,
    searcher.count(DoublePoint::new_range_query("double", 0.0f64, 1.0f64)?)?
  );
  assert_eq!(
    10_000 - zero_count - one_count,
    searcher.count(DoublePoint::new_range_query(
      "double",
      2.0f64,
      cardinality as f64
    )?)?
  );

  assert_eq!(
    zero_count,
    searcher.count(BinaryPoint::new_range_query("bytes", vec![0u8], vec![0u8])?)?
  );
  assert_eq!(
    one_count,
    searcher.count(BinaryPoint::new_range_query("bytes", vec![1u8], vec![1u8])?)?
  );
  assert_eq!(
    zero_count + one_count,
    searcher.count(BinaryPoint::new_range_query("bytes", vec![0u8], vec![1u8])?)?
  );
  assert_eq!(
    10_000 - zero_count - one_count,
    searcher.count(BinaryPoint::new_range_query(
      "bytes",
      vec![2u8],
      vec![cardinality as u8]
    )?)?
  );

  w.close()?;
  Ok(())
}
#[test]
fn test_invalid_point_in_set_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_invalid_point_in_set_binary_query() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_point_in_set_query_to_string() -> Result<()> {
  // TODO
  Ok(())
}

#[test]
fn test_point_in_set_query_get_packed_points() -> Result<()> {
  // TODO
  Ok(())
}
#[test]
fn test_range_optimizes_if_all_points_match() -> Result<()> {
  let mut random = random();
  let num_dims: usize = TestUtil::next_usize(&mut random, 1, 3);

  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  // index a single document with an N-dim point
  let mut value = Vec::with_capacity(num_dims);
  for _ in 0..num_dims {
    value.push(TestUtil::next_int(&mut random, 1, 10));
  }

  {
    let mut doc = Document::new();
    doc.add(IntPoint::new("point", &value)?);
    w.add_document(doc)?;
  }

  let query = {
    let reader = w.get_reader()?;
    let mut searcher = IndexSearcher::from_cr(reader)?;
    searcher.set_query_cache(None);

    let mut lower = Vec::with_capacity(num_dims);
    let mut upper = Vec::with_capacity(num_dims);
    #[allow(clippy::needless_range_loop)]
    for i in 0..num_dims {
      lower.push(value[i] - random.random_range(0..1));
      upper.push(value[i] + random.random_range(0..1));
    }

    let query = IntPoint::new_range_query_n("point", &lower, &upper)?;
    let weight = searcher.create_weight(query.clone(), CompleteNoScores, 1.0)?;
    let _scorer = weight
      .scorer(&searcher.get_leaf_contexts()?[0], &searcher)?
      .unwrap();
    query
  };
  // when not all docs have a value, optimization should not apply
  w.add_document(Document::new())?;
  w.force_merge(1)?;
  w.commit()?;

  let reader = w.get_reader()?;
  let mut searcher = IndexSearcher::from_cr(reader)?;
  searcher.set_query_cache(None);

  let weight = searcher.create_weight(query, CompleteNoScores, 1.0)?;
  let _scorer = weight.scorer(&searcher.get_leaf_contexts()?[0], &searcher)?;

  w.close()?;
  Ok(())
}
#[test]
fn test_point_range_weight_count() -> Result<()> {
  // the optimization for Weight::count kicks in only when the number of dimensions is 1
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let w = RandomIndexWriter::new(&mut random, dir.clone());

  let num_points: usize = random.random_range(1..10) as usize;
  let mut points = vec![0i32; num_points];

  let num_queries: usize = random.random_range(1..10) as usize;
  let mut lower_bound = vec![0i32; num_queries];
  let mut upper_bound = vec![0i32; num_queries];
  let mut expected_count = vec![0i32; num_queries];

  // generate random queries
  for i in 0..num_queries {
    lower_bound[i] = random.random_range(1..10);
    // allow malformed ranges where upperBound could be less than lowerBound
    upper_bound[i] = random.random_range(1..10);
  }

  // generate random 1D points
  #[allow(clippy::needless_range_loop)]
  for i in 0..num_points {
    points[i] = random.random_range(1..10);
    if random.random_bool(0.5) {
      // the doc may have at-most 1 point
      let mut doc = Document::new();
      doc.add(IntPoint::new("point", [points[i]])?);
      w.add_document(doc)?;

      for j in 0..num_queries {
        // calculate the number of points that lie within the query range
        if lower_bound[j] <= points[i] && points[i] <= upper_bound[j] {
          expected_count[j] += 1;
        }
      }
    }
  }

  w.commit()?;
  w.force_merge(1)?;

  let reader = w.get_reader()?;
  let searcher = IndexSearcher::from_cr(reader)?;

  // we need at least 1 leaf in the segment
  if !searcher.get_leaf_contexts()?.is_empty() {
    let leaf = &searcher.get_leaf_contexts()?[0];
    #[allow(clippy::needless_range_loop)]
    for i in 0..num_queries {
      let query = IntPoint::new_range_query("point", lower_bound[i], upper_bound[i])?;
      let weight = searcher.create_weight(query, CompleteNoScores, 1.0)?;
      assert_eq!(expected_count[i], weight.count(leaf)?);
    }
  }
  w.close()?;
  Ok(())
}
#[test]
fn test_point_range_equals() -> Result<()> {
  let q1 = IntPoint::new_range_query("a", 0i32, 1000i32)?;
  let q2 = IntPoint::new_range_query("a", 0i32, 1000i32)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, IntPoint::new_range_query("a", 1i32, 1000i32)?);
  assert_ne!(q1, IntPoint::new_range_query("b", 0i32, 1000i32)?);

  let q1 = LongPoint::new_range_query("a", 0i64, 1000i64)?;
  let q2 = LongPoint::new_range_query("a", 0i64, 1000i64)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, LongPoint::new_range_query("a", 1i64, 1000i64)?);

  let q1 = FloatPoint::new_range_query("a", 0.0f32, 1000.0f32)?;
  let q2 = FloatPoint::new_range_query("a", 0.0f32, 1000.0f32)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, FloatPoint::new_range_query("a", 1.0f32, 1000.0f32)?);

  let q1 = DoublePoint::new_range_query("a", 0.0f64, 1000.0f64)?;
  let q2 = DoublePoint::new_range_query("a", 0.0f64, 1000.0f64)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, DoublePoint::new_range_query("a", 1.0f64, 1000.0f64)?);

  let zeros = vec![0u8; 5];
  let ones = vec![0xffu8; 5];

  let q1 = BinaryPoint::new_range_query_multi_dim(
    "a",
    std::slice::from_ref(&zeros),
    std::slice::from_ref(&ones),
  )?;
  let q2 = BinaryPoint::new_range_query_multi_dim(
    "a",
    std::slice::from_ref(&zeros),
    std::slice::from_ref(&ones),
  )?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );

  let mut other = ones.clone();
  other[2] = 5;
  assert_ne!(
    q1,
    BinaryPoint::new_range_query_multi_dim("a", &[zeros], &[other],)?
  );

  Ok(())
}
#[test]
fn test_point_exact_equals() -> Result<()> {
  let q1 = IntPoint::new_exact_query("a", 1000i32)?;
  let q2 = IntPoint::new_exact_query("a", 1000i32)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, IntPoint::new_exact_query("a", 1i32)?);
  assert_ne!(q1, IntPoint::new_exact_query("b", 1000i32)?);

  let q1 = LongPoint::new_exact_query("a", 1000i64)?;
  let q2 = LongPoint::new_exact_query("a", 1000i64)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, LongPoint::new_exact_query("a", 1i64)?);

  assert_eq!(q1.get_lower_point(), q2.get_lower_point());
  assert_eq!(q1.get_upper_point(), q2.get_upper_point());

  let q1 = FloatPoint::new_exact_query("a", 1000.0f32)?;
  let q2 = FloatPoint::new_exact_query("a", 1000.0f32)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, FloatPoint::new_exact_query("a", 1.0f32)?);

  assert_eq!(q1.get_lower_point(), q2.get_lower_point());
  assert_eq!(q1.get_upper_point(), q2.get_upper_point());

  let q1 = DoublePoint::new_exact_query("a", 1000.0f64)?;
  let q2 = DoublePoint::new_exact_query("a", 1000.0f64)?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );
  assert_ne!(q1, DoublePoint::new_exact_query("a", 1.0f64)?);

  assert_eq!(q1.get_lower_point(), q2.get_lower_point());
  assert_eq!(q1.get_upper_point(), q2.get_upper_point());

  let ones = vec![0xffu8; 5];
  let q1 = BinaryPoint::new_exact_query("a", ones.clone())?;
  let q2 = BinaryPoint::new_exact_query("a", ones.clone())?;
  assert_eq!(q1, q2);
  assert_eq!(
    CoreHelper::calculate_hash(&q1),
    CoreHelper::calculate_hash(&q2)
  );

  let mut other = ones.clone();
  other[2] = 5;
  assert_ne!(q1, BinaryPoint::new_exact_query("a", other)?);
  assert_eq!(q1.get_lower_point(), q2.get_lower_point());
  assert_eq!(q1.get_upper_point(), q2.get_upper_point());

  Ok(())
}
#[test]
fn test_point_in_set_equals() -> Result<()> {
  // TODO
  Ok(())
}
#[derive(Debug, Clone)]
pub struct PointRangeQueryBaseImpl;
impl PointRangeBase for PointRangeQueryBaseImpl {
  fn to_string(&self, _dimension: usize, _value: &[u8]) -> Result<String> {
    Ok("foo".to_string())
  }
}
#[test]
fn test_invalid_point_length() -> Result<()> {
  let lower = vec![0u8; 4];
  let upper = vec![0u8; 8];

  let err = PointRangeQuery::new(
    "field".to_string(),
    lower,
    upper,
    1,
    PointRangeQueryBaseImpl,
  )
  .unwrap_err();

  assert!(matches!(err, LuceneError::IllegalArgument(_)));
  if let LuceneError::IllegalArgument(msg) = err {
    assert_eq!(
      "lower_point has length=4 but upper_point has different length=8",
      msg.to_string()
    );
  }

  Ok(())
}

#[test]
fn test_next_up() -> Result<()> {
  assert_eq!(
    0.0f64.total_cmp(&DoublePoint::next_up(-0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f64::from_bits(1).total_cmp(&DoublePoint::next_up(0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f64::INFINITY.total_cmp(&DoublePoint::next_up(f64::MAX)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f64::INFINITY.total_cmp(&DoublePoint::next_up(f64::INFINITY)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    (-f64::MAX).total_cmp(&DoublePoint::next_up(f64::NEG_INFINITY)),
    std::cmp::Ordering::Equal
  );

  assert_eq!(
    0.0f32.total_cmp(&FloatPoint::next_up(-0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f32::from_bits(1).total_cmp(&FloatPoint::next_up(0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f32::INFINITY.total_cmp(&FloatPoint::next_up(f32::MAX)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f32::INFINITY.total_cmp(&FloatPoint::next_up(f32::INFINITY)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    (-f32::MAX).total_cmp(&FloatPoint::next_up(f32::NEG_INFINITY)),
    std::cmp::Ordering::Equal
  );

  Ok(())
}

#[test]
fn test_next_down() -> Result<()> {
  assert_eq!(
    (-0.0f64).total_cmp(&DoublePoint::next_down(0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    (-f64::from_bits(1)).total_cmp(&DoublePoint::next_down(-0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f64::NEG_INFINITY.total_cmp(&DoublePoint::next_down(-f64::MAX)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f64::NEG_INFINITY.total_cmp(&DoublePoint::next_down(f64::NEG_INFINITY)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f64::MAX.total_cmp(&DoublePoint::next_down(f64::INFINITY)),
    std::cmp::Ordering::Equal
  );

  assert_eq!(
    (-0.0f32).total_cmp(&FloatPoint::next_down(0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    (-f32::from_bits(1)).total_cmp(&FloatPoint::next_down(-0.0)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f32::NEG_INFINITY.total_cmp(&FloatPoint::next_down(-f32::MAX)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f32::NEG_INFINITY.total_cmp(&FloatPoint::next_down(f32::NEG_INFINITY)),
    std::cmp::Ordering::Equal
  );
  assert_eq!(
    f32::MAX.total_cmp(&FloatPoint::next_down(f32::INFINITY)),
    std::cmp::Ordering::Equal
  );

  Ok(())
}
#[ignore]
#[test]
fn test_inverse_point_range() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

  let num_dims = random.random_range(1..=3);
  let num_docs = at_least(
    &mut random,
    (10 * BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE) as i32,
  );

  for i in 0..num_docs {
    let mut doc = Document::new();
    let values = vec![i; num_dims];
    doc.add(IntPoint::new("f", values.as_slice())?);
    w.add_document(doc)?;
  }

  w.force_merge(1)?;

  let reader = directory_reader::open_from_writer(&w)?;
  w.close()?;

  let searcher = new_searcher_with_reader(reader)?;

  let mut low = vec![0i32; num_dims];
  let mut high = vec![0i32; num_dims];

  high.fill(num_docs - 2);
  assert_eq!(
    (high[0] - low[0] + 1),
    searcher.count(IntPoint::new_range_query_n("f", &low, &high)?)?
  );

  low.fill(1);
  assert_eq!(
    (high[0] - low[0] + 1),
    searcher.count(IntPoint::new_range_query_n("f", &low, &high)?)?
  );

  high.fill(num_docs - 1);
  assert_eq!(
    (high[0] - low[0] + 1),
    searcher.count(IntPoint::new_range_query_n("f", &low, &high)?)?
  );

  low.fill((BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE + 1) as i32);
  assert_eq!(
    (high[0] - low[0] + 1),
    searcher.count(IntPoint::new_range_query_n("f", &low, &high)?)?
  );

  high.fill(num_docs - BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE as i32);
  assert_eq!(
    (high[0] - low[0] + 1),
    searcher.count(IntPoint::new_range_query_n("f", &low, &high)?)?
  );

  Ok(())
}

#[test]
fn test_range_query_skips_non_matching_segments() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

  {
    let mut doc = Document::new();
    doc.add(IntPoint::new("field", [2i32])?);
    doc.add(IntPoint::new("field2d", [1i32, 3i32])?);
    w.add_document(doc)?;
  }

  let reader = directory_reader::open_from_writer(&w)?;
  let searcher = new_searcher_with_wrap(&mut random, reader, false)?;

  let query = IntPoint::new_range_query("field", 0i32, 1i32)?;
  let weight = searcher.create_weight(query, CompleteNoScores, 1.0)?;
  assert!(
    weight
      .scorer_supplier(&searcher.get_leaf_contexts()?[0], &searcher)?
      .is_none()
  );

  let query = IntPoint::new_range_query("field", 3i32, 4i32)?;
  let weight = searcher.create_weight(query, CompleteNoScores, 1.0)?;
  assert!(
    weight
      .scorer_supplier(&searcher.get_leaf_contexts()?[0], &searcher)?
      .is_none()
  );

  let query = IntPoint::new_range_query_n("field2d", &[0i32, 0i32], &[2i32, 2i32])?;
  let weight = searcher.create_weight(query, CompleteNoScores, 1.0)?;
  assert!(
    weight
      .scorer_supplier(&searcher.get_leaf_contexts()?[0], &searcher)?
      .is_none()
  );

  let query = IntPoint::new_range_query_n("field2d", &[2i32, 2i32], &[4i32, 4i32])?;
  let weight = searcher.create_weight(query, CompleteNoScores, 1.0)?;
  assert!(
    weight
      .scorer_supplier(&searcher.get_leaf_contexts()?[0], &searcher)?
      .is_none()
  );

  w.close()?;
  Ok(())
}
