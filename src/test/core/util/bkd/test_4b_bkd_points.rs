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
use crate::core::index::check_index::VerifyPointsVisitor;
use crate::core::index::point_values::PointValues;
use crate::core::store::IO_CONTEXT_DEFAULT;
use crate::core::store::directory::Directory;
use crate::core::store::{FSDirectories, IndexInput, IndexOutput};
use crate::core::util::TryIntoInt;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::bkd::bkd_config::BKDConfig;
use crate::core::util::bkd::bkd_reader::BKDReader;
use crate::core::util::bkd::bkd_writer::{BKDWriter, DEFAULT_MAX_MB_SORT_IN_HEAP};
use crate::core::util::clone::TryClone;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::core::util::numeric_utils::NumericUtils;
use crate::test_framework::core::util::lucene_test_case::{create_temp_dir_with_prefix, random};
use parking_lot::Mutex;
use rand::RngExt;
use std::sync::Arc;

// For example, run with `cargo test --features monster test_4b_bkd_points -- --ignored
// --nocapture`. These tests take at least 4 hours and consume many GB of temporary disk space.

#[allow(dead_code)] // for quick search
struct Test4BBKDPoints;

#[cfg(feature = "monster")]
#[test]
#[ignore = "monster"]
fn test_1d() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(FSDirectories::open(
    create_temp_dir_with_prefix("4BBKDPoints1D")?.keep(),
  )?);

  let num_docs = (i32::MAX / 13) + 100;

  let config = BKDConfig::new(
    1,
    1,
    BitUtil::LONG_BYTES,
    BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE,
  )?;
  let mut writer = BKDWriter::new(
    num_docs,
    dir.as_ref(),
    "_0",
    config,
    f64::from(DEFAULT_MAX_MB_SORT_IN_HEAP),
    26_i64 * i64::from(num_docs),
  )?;
  let mut counter = 0_i32;
  let mut packed_bytes = vec![0; BitUtil::LONG_BYTES];
  for doc_id in 0..num_docs {
    for _ in 0..26 {
      // first a random int:
      NumericUtils::int_to_sortable_bytes(random.random(), &mut packed_bytes, 0);
      // then our counter, which will overflow a bit in the end:
      NumericUtils::int_to_sortable_bytes(counter, &mut packed_bytes, BitUtil::INT_BYTES);
      writer.add(&packed_bytes, doc_id)?;
      counter = counter.wrapping_add(1);
    }
    if cfg!(feature = "test_log_verbose") && doc_id % 100_000 == 0 {
      println!("{doc_id} of {num_docs}...");
    }
  }
  let mut output =
    dir.create_output("1d.bkd", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
  let finalizer = writer.finish(&mut output)?.expect("points were added");
  let index_fp = output.get_file_pointer()?;
  writer.write_index(&mut output, None, &finalizer)?;
  output.close()?;

  let mut meta_input =
    dir.open_input("1d.bkd", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
  meta_input.seek(index_fp)?;
  let mut index_input = meta_input.try_clone()?;
  let data_input = Arc::new(Mutex::new(meta_input.try_clone()?));
  let values = BKDReader::new(&mut meta_input, &mut index_input, Arc::clone(&data_input))?;
  let mut visitor = VerifyPointsVisitor::new("1d".to_string(), num_docs, &values)?;
  values.intersect(&mut visitor)?;
  let point_count: i64 = values.size()?.try_convert()?;
  assert_eq!(point_count, visitor.get_point_count_seen());
  assert_eq!(
    i64::from(values.get_doc_count()?),
    visitor.get_doc_count_seen()
  );
  drop(values);
  index_input.close()?;
  data_input.lock().close()?;
  meta_input.close()?;
  dir.as_ref().close()
}

#[cfg(feature = "monster")]
#[test]
#[ignore = "monster"]
fn test_2d() -> Result<()> {
  let mut random = random();
  let dir = Arc::new(FSDirectories::open(
    create_temp_dir_with_prefix("4BBKDPoints2D")?.keep(),
  )?);

  let num_docs = (i32::MAX / 13) + 100;

  let config = BKDConfig::new(
    2,
    2,
    BitUtil::LONG_BYTES,
    BKDConfig::DEFAULT_MAX_POINTS_IN_LEAF_NODE,
  )?;
  let mut writer = BKDWriter::new(
    num_docs,
    dir.as_ref(),
    "_0",
    config,
    f64::from(DEFAULT_MAX_MB_SORT_IN_HEAP),
    26_i64 * i64::from(num_docs),
  )?;
  let mut counter = 0_i32;
  let mut packed_bytes = vec![0; 2 * BitUtil::LONG_BYTES];
  for doc_id in 0..num_docs {
    for _ in 0..26 {
      // first a random int:
      NumericUtils::int_to_sortable_bytes(random.random(), &mut packed_bytes, 0);
      // then our counter, which will overflow a bit in the end:
      NumericUtils::int_to_sortable_bytes(counter, &mut packed_bytes, BitUtil::INT_BYTES);
      // then two random ints for the 2nd dimension:
      NumericUtils::int_to_sortable_bytes(random.random(), &mut packed_bytes, BitUtil::LONG_BYTES);
      NumericUtils::int_to_sortable_bytes(
        random.random(),
        &mut packed_bytes,
        BitUtil::LONG_BYTES + BitUtil::INT_BYTES,
      );
      writer.add(&packed_bytes, doc_id)?;
      counter = counter.wrapping_add(1);
    }
    if cfg!(feature = "test_log_verbose") && doc_id % 100_000 == 0 {
      println!("{doc_id} of {num_docs}...");
    }
  }
  let mut output =
    dir.create_output("2d.bkd", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
  let finalizer = writer.finish(&mut output)?.expect("points were added");
  let index_fp = output.get_file_pointer()?;
  writer.write_index(&mut output, None, &finalizer)?;
  output.close()?;

  let mut meta_input =
    dir.open_input("2d.bkd", IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?)?;
  meta_input.seek(index_fp)?;
  let mut index_input = meta_input.try_clone()?;
  let data_input = Arc::new(Mutex::new(meta_input.try_clone()?));
  let values = BKDReader::new(&mut meta_input, &mut index_input, Arc::clone(&data_input))?;
  let mut visitor = VerifyPointsVisitor::new("2d".to_string(), num_docs, &values)?;
  values.intersect(&mut visitor)?;
  let point_count: i64 = values.size()?.try_convert()?;
  assert_eq!(point_count, visitor.get_point_count_seen());
  assert_eq!(
    i64::from(values.get_doc_count()?),
    visitor.get_doc_count_seen()
  );
  drop(values);
  index_input.close()?;
  data_input.lock().close()?;
  meta_input.close()?;
  dir.as_ref().close()
}
