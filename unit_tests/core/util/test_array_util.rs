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
// Migrated from src/core/util/array_util.rs

use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use std::cmp::Ordering;
use std::fmt::Display;

use rand::Rng;
use rand::RngExt;

use crate::core::util::array_util::{
  ArrayUtil, ByteArrayComparator, U32byteArrayComparator, U64byteArrayComparator,
};
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{NaturalOrder, ReverseOrder, SliceCopyOps, ToInt};
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
pub struct TestArrayUtil;
#[test]
fn test_growth() -> Result<()> {
  let mut current_size = 0;
  let mut copy_cost = 0;

  while current_size != ArrayUtil::MAX_ARRAY_LENGTH {
    let next_size = ArrayUtil::oversize(1 + current_size, size_of::<usize>())?;
    assert!(next_size > current_size);

    if current_size > 0 {
      copy_cost += current_size;
      let copy_cost_per_element = copy_cost as f64 / current_size as f64;
      assert!(
        copy_cost_per_element < 10.0,
        "cost {}",
        copy_cost_per_element
      );
    }

    current_size = next_size;
  }
  Ok(())
}

#[test]
fn test_max_size() -> Result<()> {
  for elem_size in 0..10 {
    assert_eq!(
      ArrayUtil::MAX_ARRAY_LENGTH,
      ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH, elem_size)?
    );
    assert_eq!(
      ArrayUtil::MAX_ARRAY_LENGTH,
      ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH - 1, elem_size)?
    );
  }
  Ok(())
}

#[test]
fn test_too_big() {
  let result = ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH + 1, 1);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
}

#[test]
fn test_exact_limit() -> Result<()> {
  assert_eq!(
    ArrayUtil::MAX_ARRAY_LENGTH,
    ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH, 1)?
  );
  Ok(())
}
#[test]
fn test_invalid_element_sizes() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 10000);
  for _ in 0..num {
    let min_target_size = random.random_range(0..ArrayUtil::MAX_ARRAY_LENGTH);
    let elem_size = random.random_range(0..11);
    let v = ArrayUtil::oversize(min_target_size, elem_size)?;
    assert!(v >= min_target_size);
  }
  Ok(())
}

fn parse_int<R>(random: &mut R, s: &str) -> Result<i32>
where
  R: Rng + ?Sized,
{
  let start = random.random_range(0..5);
  let extra_length = random.random_range(0..4);
  let mut chars: Vec<char> = vec![' '; s.len() + start + extra_length];
  let s_chars: Vec<char> = s.chars().collect();
  chars.copy_from(&s_chars, start);
  ArrayUtil::parse_int_default(&chars, start as i32, s.len() as i32)
}
#[test]
fn test_parse_int() -> Result<()> {
  let mut random = random();
  let result = parse_int(&mut random, "");
  assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

  let result = parse_int(&mut random, "foo");
  assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

  let result = parse_int(&mut random, &i64::MAX.to_string());
  assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

  let result = parse_int(&mut random, "0.34");
  assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

  let result = parse_int(&mut random, "1");
  assert!(result.is_ok());
  let value = result?;
  assert_eq!(value, 1, "{} does not equal: 1", value);

  let result = parse_int(&mut random, "-10000");
  assert!(result.is_ok());
  let value = result?;
  assert_eq!(value, -10000, "{} does not equal: -10000", value);

  let result = parse_int(&mut random, "1923");
  assert!(result.is_ok());
  let value = result?;
  assert_eq!(value, 1923, "{} does not equal: 1923", value);

  let result = parse_int(&mut random, "-1");
  assert!(result.is_ok());
  let value = result?;
  assert_eq!(value, -1, "{} does not equal: -1", value);

  let result = ArrayUtil::parse_int_default(&"foo 1923 bar".chars().collect::<Vec<char>>(), 4, 4);
  assert!(result.is_ok());
  let value = result?;
  assert_eq!(value, 1923, "{} does not equal: 1923", value);
  Ok(())
}
fn create_random_array<R>(random: &mut R, max_size: i32) -> Vec<i32>
where
  R: Rng + ?Sized,
{
  let size = random.random_range(1..=max_size);
  let mut array = Vec::with_capacity(size as usize);

  for _ in 0..size {
    array.push(random.random_range(0..size));
  }
  array
}
#[test]
fn test_intro_sort() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 50);
  for _ in 0..num {
    let mut a1 = create_random_array(&mut random, 2000);
    let mut a2 = a1.clone();

    ArrayUtil::intro_sort(&mut a1)?;
    a2.sort();
    assert_eq!(a1, a2);

    a1 = create_random_array(&mut random, 2000);
    a2 = a1.clone();
    ArrayUtil::intro_sort_with_comparator(&mut a1, ReverseOrder::new())?;
    a2.sort_by(|x, y| y.cmp(x)); // reverse order
    assert_eq!(a1, a2);

    ArrayUtil::intro_sort(&mut a1)?;
    a2.sort();
    assert_eq!(a1, a2);
  }
  Ok(())
}
fn create_sparse_random_array<R>(random: &mut R, max_size: i32) -> Vec<i32>
where
  R: Rng + ?Sized,
{
  let size = random.random_range(0..=max_size);
  let mut array = Vec::with_capacity(size as usize);

  for _ in 0..size {
    array.push(random.random_range(0..2));
  }
  array
}
// This is a test for LUCENE-3054 (which fails without the merge sort fall
// back with stack overflow in most cases)
#[test]
fn test_quick_to_heap_sort_fallback() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 10);
  for _ in 0..num {
    let mut a1 = create_sparse_random_array(&mut random, 40_000);
    let mut a2 = a1.clone();
    ArrayUtil::intro_sort(&mut a1)?;
    a2.sort();
    assert_eq!(a1, a2);
  }
  Ok(())
}
#[test]
fn test_tim_sort() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 50);

  for _ in 0..num {
    let mut a1 = create_random_array(&mut random, 2000);
    let mut a2 = a1.clone();

    ArrayUtil::tim_sort(&mut a1)?;
    a2.sort();
    assert_eq!(a1, a2);

    a1 = create_random_array(&mut random, 2000);
    a2 = a1.clone();
    ArrayUtil::tim_sort_with_comparator(&mut a1, ReverseOrder::new())?;
    a2.sort_by(|a, b| b.cmp(a));
    assert_eq!(a1, a2);
    // reverse back, so we can test that completely backwards sorted
    // array (worst case) is working:
    ArrayUtil::tim_sort(&mut a1)?;
    a2.sort();
    assert_eq!(a1, a2);
  }
  Ok(())
}
#[derive(Debug, Clone, Default, Copy)]
struct Item {
  val: i32,
  order: i32,
}

impl Item {
  fn new(val: i32, order: i32) -> Self {
    Item { val, order }
  }
}
impl Display for Item {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Item {{ val: {}, order: {} }}", self.val, self.order)
  }
}

impl Eq for Item {}

impl PartialEq for Item {
  fn eq(&self, other: &Self) -> bool {
    self.order == other.order
  }
}

impl PartialOrd<Self> for Item {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for Item {
  fn cmp(&self, other: &Self) -> Ordering {
    self.order.cmp(&other.order)
  }
}

#[test]
fn test_merge_sort_stability() -> Result<()> {
  let mut random = random();
  let mut items = Vec::with_capacity(100);

  for i in 0..100 {
    // half of the items have value but same order. The value of this
    // items is sorted, so they should always be in order
    // after sorting. The other half has defined order, but
    // no (-1) value (they should appear after all above,
    // when sorted).
    let equal = random.random_bool(0.5);
    if equal {
      items.push(Item::new(i + 1, 0));
    } else {
      items.push(Item::new(-1, random.random_range(1..=1000)));
    }
  }
  if cfg!(feature = "test_log_verbose") {
    println!("Before: {:?}", items);
  }
  // if you replace this with ArrayUtil.quickSort(), test should fail:
  ArrayUtil::tim_sort(&mut items)?;

  if cfg!(feature = "test_log_verbose") {
    println!("Sorted: {:?}", items);
  }

  let mut last = &items[0];

  for item in &items[1..] {
    if item.order == 0 {
      assert!(item.val > last.val);
    }
    assert!(item.order >= last.order);

    last = item;
  }
  Ok(())
}
#[test]
fn test_tim_sort_stability() -> Result<()> {
  let mut random = rand::rng();
  let mut items = Vec::with_capacity(100);

  for i in 0..100 {
    // half of the items have value but same order. The value of this
    // items is sorted, so they should always be in order
    // after sorting. The other half has defined order, but
    // no (-1) value (they should appear after all above,
    // when sorted).
    let equal = random.random_bool(0.5);
    if equal {
      items.push(Item::new(i + 1, 0)); // "equal" items
    } else {
      items.push(Item::new(-1, random.random_range(1..=1000))); // Items with defined order
    }
  }

  if cfg!(feature = "test_log_verbose") {
    println!("Before: {:?}", items);
  }
  ArrayUtil::tim_sort(&mut items)?;

  if cfg!(feature = "test_log_verbose") {
    println!("Sorted: {:?}", items);
  }

  let mut last = &items[0];

  for item in &items[1..] {
    if item.order == 0 {
      // order of "equal" items should be not mixed up
      assert!(item.val > last.val, "Expected sorted value for equal items");
    }
    assert!(item.order >= last.order, "Expected sorted order");
    last = item;
  }
  Ok(())
}
// should produce no errors
#[test]
fn test_empty_array_sort() -> Result<()> {
  let mut a: Vec<i32> = Vec::new();
  ArrayUtil::intro_sort(&mut a)?;
  ArrayUtil::tim_sort(&mut a)?;
  ArrayUtil::intro_sort_with_comparator(&mut a, ReverseOrder::new())?;
  ArrayUtil::tim_sort_with_comparator(&mut a, ReverseOrder::new())?;
  Ok(())
}

#[test]
fn test_select() -> Result<()> {
  let mut random = random();
  for _ in 0..100 {
    do_test_select(&mut random)?
  }
  Ok(())
}

fn do_test_select<R>(random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
{
  let from = random.random_range(0..5) as usize;
  let to = from + TestUtil::next_usize(random, 1, 10_000);
  let max = if random.random_bool(0.5) {
    random.random_range(0..100)
  } else {
    random.random_range(0..100_000)
  };

  let arr: Vec<i32> = (0..from + to + random.random_range(0..5))
    .map(|_| TestUtil::next_int(random, 0, max))
    .collect();

  let k = TestUtil::next_int(random, from as i32, (to - 1) as i32) as usize;

  let mut expected = arr.clone();
  expected[from..to].sort();

  let mut actual = arr.clone();
  ArrayUtil::select(&mut actual, from, to, k, &NaturalOrder::new())?;

  assert_eq!(expected[k], actual[k]);

  for (i, &value) in actual.iter().enumerate() {
    if i < from || i >= to {
      assert_eq!(arr[i], value);
    } else if i <= k {
      assert!(value <= actual[k]);
    } else {
      assert!(value >= actual[k]);
    }
  }
  Ok(())
}

#[test]
fn test_grow_exact() -> Result<()> {
  let mut random = random();
  let mut arr: Vec<i16> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert_eq!(arr, vec![1, 2, 3, 0]);
  let mut arr: Vec<i16> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert_eq!(arr, vec![1, 2, 3, 0, 0]);
  let mut arr: Vec<i16> = vec![1, 2, 3];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<i32> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert_eq!(arr, vec![1, 2, 3, 0]);
  let mut arr: Vec<i32> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert_eq!(arr, vec![1, 2, 3, 0, 0]);
  let mut arr: Vec<i32> = vec![1, 2, 3];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<i64> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert_eq!(arr, vec![1, 2, 3, 0]);
  let mut arr: Vec<i64> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert_eq!(arr, vec![1, 2, 3, 0, 0]);
  let mut arr: Vec<i64> = vec![1, 2, 3];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<f32> = vec![0.1, 0.2, 0.3];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert!((arr[3] - 0.0).abs() < 0.001);
  let mut arr: Vec<f32> = vec![0.1, 0.2, 0.3];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert!((arr[3] - 0.0).abs() < 0.001);
  assert!((arr[4] - 0.0).abs() < 0.001);
  let mut arr: Vec<f32> = vec![1.0, 2.0, 3.0];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<f64> = vec![0.1, 0.2, 0.3];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert!((arr[3] - 0.0).abs() < 0.001);
  let mut arr: Vec<f64> = vec![0.1, 0.2, 0.3];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert!((arr[3] - 0.0).abs() < 0.001);
  assert!((arr[4] - 0.0).abs() < 0.001);
  let mut arr: Vec<f64> = vec![0.1, 0.2, 0.3];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<i8> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert_eq!(arr, vec![1, 2, 3, 0]);
  let mut arr: Vec<i8> = vec![1, 2, 3];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert_eq!(arr, vec![1, 2, 3, 0, 0]);
  let mut arr: Vec<i8> = vec![1, 2, 3];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<char> = vec!['a', 'b', 'c'];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert_eq!(arr, vec!['a', 'b', 'c', '\0']);
  let mut arr: Vec<char> = vec!['a', 'b', 'c'];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert_eq!(arr, vec!['a', 'b', 'c', '\0', '\0']);
  let mut arr: Vec<char> = vec!['a', 'b', 'c'];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  let mut arr: Vec<Option<String>> = vec![
    Some("a1".to_string()),
    Some("b2".to_string()),
    Some("c3".to_string()),
  ];
  ArrayUtil::grow_exact(&mut arr, 4)?;
  assert_eq!(
    arr,
    vec![
      Some("a1".to_string()),
      Some("b2".to_string()),
      Some("c3".to_string()),
      None
    ]
  );
  let mut arr: Vec<Option<String>> = vec![
    Some("a1".to_string()),
    Some("b2".to_string()),
    Some("c3".to_string()),
  ];
  ArrayUtil::grow_exact(&mut arr, 5)?;
  assert_eq!(
    arr,
    vec![
      Some("a1".to_string()),
      Some("b2".to_string()),
      Some("c3".to_string()),
      None,
      None
    ]
  );
  let mut arr: Vec<Option<String>> = vec![
    Some("a".to_string()),
    Some("b".to_string()),
    Some("c".to_string()),
  ];
  let result = ArrayUtil::grow_exact(&mut arr, random.random_range(0..3));
  assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

  Ok(())
}

#[test]
fn test_grow_in_range() -> Result<()> {
  let mut array: Vec<i32> = vec![1, 2, 3];
  // If minLength > maxLength, we return an error
  let result = ArrayUtil::grow_in_range(&mut array, 1, 0);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  let result = ArrayUtil::grow_in_range(&mut array, 4, 3);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
  let result = ArrayUtil::grow_in_range(&mut array, 5, 4);
  assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));

  // If minLength is sufficient, we return the array
  ArrayUtil::grow_in_range(&mut array, 1, 4)?;
  assert_eq!(array, vec![1, 2, 3]);
  ArrayUtil::grow_in_range(&mut array, 1, 2)?;
  assert_eq!(array, vec![1, 2, 3]);
  ArrayUtil::grow_in_range(&mut array, 1, 1)?;
  assert_eq!(array, vec![1, 2, 3]);

  let min_length = 4;
  let max_length = i32::MAX as usize;

  let mut vec = vec![1, 2, 3];
  ArrayUtil::grow_in_range(&mut vec, min_length, max_length)?;
  assert_eq!(
    ArrayUtil::oversize(min_length, size_of::<i32>())?,
    vec.len()
  );

  // The array grows to maxLength if maxLength is limiting
  let mut vec = vec![1, 2, 3];
  ArrayUtil::grow_in_range(&mut vec, min_length, min_length)?;
  assert_eq!(min_length, vec.len());
  Ok(())
}
#[test]
fn test_copy_of_sub_array() {
  let short_array: Vec<i16> = vec![1, 2, 3];
  assert_eq!(vec![1], ArrayUtil::copy_of_sub_array(&short_array, 0, 1));
  assert_eq!(
    vec![1, 2, 3],
    ArrayUtil::copy_of_sub_array(&short_array, 0, 3)
  );
  assert_eq!(
    Vec::<i16>::new(),
    ArrayUtil::copy_of_sub_array(&short_array, 0, 0)
  );

  let int_array: Vec<i32> = vec![1, 2, 3];
  assert_eq!(vec![1, 2], ArrayUtil::copy_of_sub_array(&int_array, 0, 2));
  assert_eq!(
    vec![1, 2, 3],
    ArrayUtil::copy_of_sub_array(&int_array, 0, 3)
  );
  assert_eq!(
    Vec::<i32>::new(),
    ArrayUtil::copy_of_sub_array(&int_array, 1, 1)
  );

  let long_array: Vec<i64> = vec![1, 2, 3];
  assert_eq!(vec![2], ArrayUtil::copy_of_sub_array(&long_array, 1, 2));
  assert_eq!(
    vec![1, 2, 3],
    ArrayUtil::copy_of_sub_array(&long_array, 0, 3)
  );
  assert_eq!(
    Vec::<i64>::new(),
    ArrayUtil::copy_of_sub_array(&long_array, 2, 2)
  );

  let float_array: Vec<f32> = vec![0.1, 0.2, 0.3];
  assert_eq!(
    vec![0.2, 0.3],
    ArrayUtil::copy_of_sub_array(&float_array, 1, 3)
  );
  assert_eq!(
    vec![0.1, 0.2, 0.3],
    ArrayUtil::copy_of_sub_array(&float_array, 0, 3)
  );
  assert_eq!(
    Vec::<f32>::new(),
    ArrayUtil::copy_of_sub_array(&float_array, 0, 0)
  );

  let double_array: Vec<f64> = vec![0.1, 0.2, 0.3];
  assert_eq!(vec![0.3], ArrayUtil::copy_of_sub_array(&double_array, 2, 3));
  assert_eq!(
    vec![0.1, 0.2, 0.3],
    ArrayUtil::copy_of_sub_array(&double_array, 0, 3)
  );
  assert_eq!(
    Vec::<f64>::new(),
    ArrayUtil::copy_of_sub_array(&double_array, 1, 1)
  );

  let byte_array: Vec<u8> = vec![1, 2, 3];
  assert_eq!(vec![1], ArrayUtil::copy_of_sub_array(&byte_array, 0, 1));
  assert_eq!(
    vec![1, 2, 3],
    ArrayUtil::copy_of_sub_array(&byte_array, 0, 3)
  );
  assert_eq!(
    Vec::<u8>::new(),
    ArrayUtil::copy_of_sub_array(&byte_array, 1, 1)
  );

  let char_array: Vec<char> = vec!['a', 'b', 'c'];
  assert_eq!(
    vec!['a', 'b'],
    ArrayUtil::copy_of_sub_array(&char_array, 0, 2)
  );
  assert_eq!(
    vec!['a', 'b', 'c'],
    ArrayUtil::copy_of_sub_array(&char_array, 0, 3)
  );
  assert_eq!(
    Vec::<char>::new(),
    ArrayUtil::copy_of_sub_array(&char_array, 1, 1)
  );

  let object_array: Vec<String> = vec!["a1".to_string(), "b2".to_string(), "c3".to_string()];
  assert_eq!(
    vec!["a1".to_string()],
    ArrayUtil::copy_of_sub_array(&object_array, 0, 1)
  );
  assert_eq!(
    vec!["a1".to_string(), "b2".to_string(), "c3".to_string()],
    ArrayUtil::copy_of_sub_array(&object_array, 0, 3)
  );
  assert_eq!(
    Vec::<String>::new(),
    ArrayUtil::copy_of_sub_array(&object_array, 1, 1)
  );
}
#[test]
fn test_compare_unsigned4() {
  let mut random = random();
  let a_offset = TestUtil::next_usize(&mut random, 0, 3);
  let mut a = vec![0u8; BitUtil::INT_BYTES + a_offset];
  let b_offset = TestUtil::next_usize(&mut random, 0, 3);
  let mut b = vec![0u8; BitUtil::INT_BYTES + b_offset];
  for i in 0..BitUtil::INT_BYTES {
    a[a_offset + i] = random.random::<u8>();
    loop {
      b[b_offset + i] = random.random::<u8>();
      if b[b_offset + i] != a[a_offset + i] {
        break;
      }
    }
  }

  for i in 0..BitUtil::INT_BYTES {
    let expected = a[a_offset..a_offset + BitUtil::INT_BYTES]
      .cmp(&b[b_offset..b_offset + BitUtil::INT_BYTES])
      .to_int();
    let cmp = U32byteArrayComparator;
    let actual = cmp.compare(&a, a_offset, &b, b_offset);
    assert_eq!(expected.signum(), actual.signum());

    b[b_offset + i] = a[a_offset + i];
  }

  let cmp = U32byteArrayComparator;
  assert_eq!(cmp.compare(&a, a_offset, &b, b_offset), 0);
}

#[test]
fn test_compare_unsigned8() {
  let mut random = random();
  let a_offset = TestUtil::next_usize(&mut random, 0, 7);
  let mut a = vec![0u8; BitUtil::LONG_BYTES + a_offset];
  let b_offset = TestUtil::next_usize(&mut random, 0, 7);
  let mut b = vec![0u8; BitUtil::LONG_BYTES + b_offset];
  for i in 0..BitUtil::LONG_BYTES {
    a[a_offset + i] = random.random::<u8>();
    loop {
      b[b_offset + i] = random.random::<u8>();
      if b[b_offset + i] != a[a_offset + i] {
        break;
      }
    }
  }
  for i in 0..BitUtil::LONG_BYTES {
    let expected = a[a_offset..a_offset + BitUtil::LONG_BYTES]
      .cmp(&b[b_offset..b_offset + BitUtil::LONG_BYTES])
      .to_int();
    let cmp = U64byteArrayComparator;
    let actual = cmp.compare(&a, a_offset, &b, b_offset);
    assert_eq!(expected.signum(), actual.signum());
    b[b_offset + i] = a[a_offset + i];
  }
  let cmp = U64byteArrayComparator;
  assert_eq!(cmp.compare(&a, a_offset, &b, b_offset), 0);
}
