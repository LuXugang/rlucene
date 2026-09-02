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
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::FieldBase;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::binary_doc_values::BinaryDocValues;
use crate::core::index::doc_values_iterator::DocValuesIterator;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, is_night_mode, new_directory_shared, new_index_writer_config,
  new_index_writer_config_with_analyzer, new_log_merge_policy, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestMultiDocValues;

#[test]
fn test_numerics() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut doc = Document::new();

  let mut field = NumericDocValuesField::new("numbers", 0i64);
  doc.add(field.clone());
  let _mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    let value = random.random();
    field.set_long_value(value)?;
    iw.add_document(&mut random, doc.clone())?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }
  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;
  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi = MultiDocValues::get_numeric_values(&ir, "numbers")?.expect("multi should exist");
  let mut single = merged
    .get_numeric_doc_values("numbers")?
    .expect("single dv should exist");

  for i in 0..num_docs {
    assert_eq!(i, multi.next_doc()?);
    assert_eq!(i, single.next_doc()?);
    assert_eq!(single.long_value()?, multi.long_value()?);
  }

  test_random_advance(
    &mut random,
    &mut merged.get_numeric_doc_values("numbers")?.unwrap(),
    &mut MultiDocValues::get_numeric_values(&ir, "numbers")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_numeric_doc_values("numbers")?.unwrap(),
    &mut MultiDocValues::get_numeric_values(&ir, "numbers")?.unwrap(),
    merged.max_doc()?,
  )?;
  Ok(())
}
#[test]
fn test_binary() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut doc = Document::new();

  let mut field = BinaryDocValuesField::new("bytes", BytesRef::new());
  doc.add(field.clone());

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    let s = TestUtil::random_unicode_string(&mut random);
    let bytes = BytesRef::from_string(&s);

    field.set_bytes_value(bytes)?;
    iw.add_document(&mut random, doc.clone())?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }

  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;

  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi = MultiDocValues::get_binary_values(&ir, "bytes")?.expect("multi should exist");
  let mut single = merged
    .get_binary_doc_values("bytes")?
    .expect("single should exist");

  for i in 0..num_docs {
    assert_eq!(i, multi.next_doc()?);
    assert_eq!(i, single.next_doc()?);

    let expected = single.binary_value()?.clone();
    let actual = multi.binary_value()?.clone();

    assert_eq!(expected, actual);
  }

  test_random_advance(
    &mut random,
    &mut merged.get_binary_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_binary_values(&ir, "bytes")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_binary_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_binary_values(&ir, "bytes")?.unwrap(),
    merged.max_doc()?,
  )?;

  Ok(())
}
#[test]
fn test_sorted() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut doc = Document::new();

  let mut field = SortedDocValuesField::new("bytes", BytesRef::new());
  doc.add(field.clone());

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    let s = TestUtil::random_unicode_string(&mut random);
    let r = BytesRef::from_string(s.as_ref());
    field.set_bytes_value(r)?;

    if random.random_range(0..7) == 0 {
      iw.add_document(&mut random, Document::new())?;
    }

    iw.add_document(&mut random, doc.clone())?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }

  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;
  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi = MultiDocValues::get_sorted_values(&ir, "bytes")?.expect("multi should exist");
  let mut single = merged
    .get_sorted_doc_values("bytes")?
    .expect("single dv should exist");

  assert_eq!(single.get_value_count()?, multi.get_value_count()?);

  loop {
    assert_eq!(single.next_doc()?, multi.next_doc()?);
    if single.doc_id() == NO_MORE_DOCS {
      break;
    }

    let single_ord_value = single.ord_value()?;
    let single_ord = single.lookup_ord(single_ord_value)?;
    let expected = BytesRef::deep_copy_of(single_ord.as_ref())?;

    let multi_ord_value = multi.ord_value()?;
    let multi_ord = multi.lookup_ord(multi_ord_value)?;
    let actual = multi_ord.as_ref();
    assert_eq!(&expected, actual);

    // check ord
    assert_eq!(single.ord_value()?, multi.ord_value()?);
  }

  test_random_advance(
    &mut random,
    &mut merged.get_sorted_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_values(&ir, "bytes")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_sorted_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_values(&ir, "bytes")?.unwrap(),
    merged.max_doc()?,
  )?;

  Ok(())
}
// tries to make more dups than testSorted
#[test]
fn test_sorted_with_lots_of_dups() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let mut doc = Document::new();

  let mut field = SortedDocValuesField::new("bytes", BytesRef::new());
  doc.add(field.clone());

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    let s = TestUtil::random_simple_string_with_len(&mut random, 2);
    let r = BytesRef::from_string(s.as_ref());
    field.set_bytes_value(r)?;
    iw.add_document(&mut random, doc.clone())?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }

  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;
  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi = MultiDocValues::get_sorted_values(&ir, "bytes")?.expect("multi should exist");
  let mut single = merged
    .get_sorted_doc_values("bytes")?
    .expect("single dv should exist");

  assert_eq!(single.get_value_count()?, multi.get_value_count()?);

  for i in 0..num_docs {
    assert_eq!(i, multi.next_doc()?);
    assert_eq!(i, single.next_doc()?);

    // check ord
    assert_eq!(single.ord_value()?, multi.ord_value()?);

    // check ord value
    let single_ord_value = single.ord_value()?;
    let single_ord = single.lookup_ord(single_ord_value)?;
    let expected = BytesRef::deep_copy_of(single_ord.as_ref())?;

    let multi_ord_value = multi.ord_value()?;
    let multi_ord = multi.lookup_ord(multi_ord_value)?;
    let actual = multi_ord.as_ref();

    assert_eq!(&expected, actual);
  }

  test_random_advance(
    &mut random,
    &mut merged.get_sorted_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_values(&ir, "bytes")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_sorted_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_values(&ir, "bytes")?.unwrap(),
    merged.max_doc()?,
  )?;

  Ok(())
}
#[test]
fn test_sorted_set() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    let mut doc = Document::new();
    let num_values = random.random_range(0..5);
    for _ in 0..num_values {
      let s = TestUtil::random_unicode_string(&mut random);
      let r = BytesRef::from_string(s.as_ref());
      doc.add(SortedSetDocValuesField::new("bytes", r));
    }

    iw.add_document(&mut random, doc)?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }

  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;
  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi_opt = MultiDocValues::get_sorted_set_values(&ir, "bytes")?;
  let mut single_opt = merged.get_sorted_set_doc_values("bytes")?;

  match (multi_opt.as_mut(), single_opt.as_mut()) {
    (None, None) => {},

    (Some(multi), Some(single)) => {
      assert_eq!(single.get_value_count()?, multi.get_value_count()?);

      let value_count = single.get_value_count()?;
      for i in 0..value_count {
        let expected = BytesRef::deep_copy_of(single.lookup_ord(i)?.as_ref())?;
        let actual = multi.lookup_ord(i)?;
        assert_eq!(&expected, actual.as_ref());
      }

      loop {
        let doc_id = single.next_doc()?;
        assert_eq!(doc_id, multi.next_doc()?);
        if doc_id == NO_MORE_DOCS {
          break;
        }

        assert_eq!(single.doc_value_count()?, multi.doc_value_count()?);
        let cnt = single.doc_value_count()?;
        for _ in 0..cnt {
          assert_eq!(single.next_ord()?, multi.next_ord()?);
        }
      }
    },
    _ => {
      unreachable!(
        "multi and single SortedSetDocValues mismatch: one is None and the other is Some"
      );
    },
  }

  test_random_advance(
    &mut random,
    &mut merged.get_sorted_set_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_set_values(&ir, "bytes")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_sorted_set_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_set_values(&ir, "bytes")?.unwrap(),
    merged.max_doc()?,
  )?;

  Ok(())
}

#[test]
fn test_sorted_set_with_dups() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mut iwc = new_index_writer_config(&mut random)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);
  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    // tries to make more dups than test_sorted_set
    let mut doc = Document::new();
    let num_values = random.random_range(0..5);
    for _ in 0..num_values {
      let s = TestUtil::random_simple_string_with_len(&mut random, 2);
      let r = BytesRef::from_string(s.as_ref());
      doc.add(SortedSetDocValuesField::new("bytes", r));
    }

    iw.add_document(&mut random, doc)?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }

  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;
  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi_opt = MultiDocValues::get_sorted_set_values(&ir, "bytes")?;
  let mut single_opt = merged.get_sorted_set_doc_values("bytes")?;

  match (multi_opt.as_mut(), single_opt.as_mut()) {
    (None, None) => {},

    (Some(multi), Some(single)) => {
      assert_eq!(single.get_value_count()?, multi.get_value_count()?);

      // check values
      let value_count = single.get_value_count()?;
      for i in 0..value_count {
        let expected = BytesRef::deep_copy_of(single.lookup_ord(i)?.as_ref())?;
        let actual = multi.lookup_ord(i)?;
        assert_eq!(&expected, actual.as_ref());
      }

      // check ord list
      loop {
        let doc_id = single.next_doc()?;
        assert_eq!(doc_id, multi.next_doc()?);
        if doc_id == NO_MORE_DOCS {
          break;
        }

        assert_eq!(single.doc_value_count()?, multi.doc_value_count()?);
        let cnt = single.doc_value_count()?;
        for _ in 0..cnt {
          assert_eq!(single.next_ord()?, multi.next_ord()?);
        }
      }
    },

    _ => {
      unreachable!(
        "multi and single SortedSetDocValues mismatch: one is None and the other is Some"
      );
    },
  }

  test_random_advance(
    &mut random,
    &mut merged.get_sorted_set_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_set_values(&ir, "bytes")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_sorted_set_doc_values("bytes")?.unwrap(),
    &mut MultiDocValues::get_sorted_set_values(&ir, "bytes")?.unwrap(),
    merged.max_doc()?,
  )?;

  Ok(())
}

#[test]
fn test_sorted_numeric() -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;

  let mock = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, mock)?;
  iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

  let iw = RandomIndexWriter::with_config(&mut random, dir.clone(), iwc);

  let num_docs = if is_night_mode() {
    at_least(&mut random, 500)
  } else {
    at_least(&mut random, 50)
  };

  for _ in 0..num_docs {
    let mut doc = Document::new();
    let num_values = random.random_range(0..5);

    for _ in 0..num_values {
      let v = TestUtil::next_long(&mut random, i64::MIN, i64::MAX);
      doc.add(SortedNumericDocValuesField::new("nums", v));
    }

    iw.add_document(&mut random, doc)?;

    if random.random_range(0..17) == 0 {
      iw.commit(&mut random)?;
    }
  }

  iw.commit(&mut random)?;

  let ir = iw.get_reader(&mut random)?;
  iw.force_merge(&mut random, 1)?;
  let ir2 = iw.get_reader(&mut random)?;
  let merged = get_only_leaf_reader(&ir2)?;
  iw.close(&mut random)?;

  let mut multi_opt = MultiDocValues::get_sorted_numeric_values(&ir, "nums")?;
  let mut single_opt = merged.get_sorted_numeric_doc_values("nums")?;

  match (multi_opt.as_mut(), single_opt.as_mut()) {
    (None, None) => {
      // pass
    },
    (Some(multi), Some(single)) => {
      for i in 0..num_docs {
        if i > single.doc_id() {
          assert_eq!(single.next_doc()?, multi.next_doc()?);
        }

        if i == single.doc_id() {
          let single_count = single.doc_value_count()?;
          let multi_count = multi.doc_value_count()?;
          assert_eq!(single_count, multi_count);

          for _ in 0..single_count {
            let sv = single.next_value()?;
            let mv = multi.next_value()?;
            assert_eq!(sv, mv);
          }
        }
      }
    },
    _ => {
      unreachable!(
        "multi and single SortedNumericDocValues mismatch: one is None and the other is Some"
      );
    },
  }

  test_random_advance(
    &mut random,
    &mut merged.get_sorted_numeric_doc_values("nums")?.unwrap(),
    &mut MultiDocValues::get_sorted_numeric_values(&ir, "nums")?.unwrap(),
  )?;

  test_random_advance_exact(
    &mut random,
    &mut merged.get_sorted_numeric_doc_values("nums")?.unwrap(),
    &mut MultiDocValues::get_sorted_numeric_values(&ir, "nums")?.unwrap(),
    merged.max_doc()?,
  )?;

  Ok(())
}
fn test_random_advance<I1, I2, R>(random: &mut R, iter1: &mut I1, iter2: &mut I2) -> Result<()>
where
  R: Rng + ?Sized,
  I1: DocIdSetIterator,
  I2: DocIdSetIterator,
{
  assert_eq!(iter1.doc_id(), -1);
  assert_eq!(iter2.doc_id(), -1);

  while iter1.doc_id() != NO_MORE_DOCS {
    if random.random_bool(0.5) {
      let v1 = iter1.next_doc()?;
      let v2 = iter2.next_doc()?;
      assert_eq!(v1, v2);
    } else {
      let target = iter1.doc_id() + TestUtil::next_int(random, 1, 100);
      let v1 = iter1.advance(target)?;
      let v2 = iter2.advance(target)?;
      assert_eq!(v1, v2);
    }
  }

  Ok(())
}
fn test_random_advance_exact<I1, I2, R>(
  random: &mut R,
  iter1: &mut I1,
  iter2: &mut I2,
  max_doc: i32,
) -> Result<()>
where
  R: Rng + ?Sized,
  I1: DocValuesIterator,
  I2: DocValuesIterator,
{
  let mut target = TestUtil::next_int(random, 0, max_doc.min(10));

  while target < max_doc {
    let exists1 = iter1.advance_exact(target)?;
    let exists2 = iter2.advance_exact(target)?;
    assert_eq!(exists1, exists2);

    target += TestUtil::next_int(random, 0, 10);
  }

  Ok(())
}
