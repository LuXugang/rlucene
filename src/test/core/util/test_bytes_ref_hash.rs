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
// Migrated from src/core/util/bytes_ref_hash.rs

use crate::test_framework::core::util::lucene_test_case::{at_least, random};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

use rand::Rng;
use rand::RngExt;

use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::allocator_byte::DirectAllocatorByte;
use crate::core::util::bytes_ref_hash::{BytesRefHash, DirectBytesRefHash, DirectBytesStartArray};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{BYTE_BLOCK_SIZE, ByteBlockPool};
use crate::test_framework::core::index::test_concurrent_merge_scheduler::CountDownLatch;
use crate::test_framework::core::util::test_util::TestUtil;

#[allow(dead_code)] // for quick search
pub struct TestBytesRefHash;

fn new_pool() -> ByteBlockPool {
  let allocator = DirectAllocatorByte::new();
  ByteBlockPool::new(allocator)
}
fn new_hash<R>(random: &mut R) -> Result<DirectBytesRefHash>
where
  R: Rng + ?Sized,
{
  let init_size = 2 << (1 + random.random_range(0..5));
  if random.random_bool(0.5) {
    BytesRefHash::new()
  } else {
    BytesRefHash::from_bytes_start_array(init_size, DirectBytesStartArray::new(init_size as usize))
  }
}
#[test]
fn test_size() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mod_val = random.random_range(1..40);
    for i in 0..797 {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }
      ref_builder.copy_chars_from_string(&str_value)?;
      let count = hash.size();
      let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

      if key < 0 {
        assert_eq!(hash.size(), count,);
      } else {
        assert_eq!(hash.size(), count + 1);
      }

      if i % mod_val == 0 {
        hash.clear(&mut byte_block_pool);
        assert_eq!(hash.size(), 0);
        hash.reinit()?;
      }
    }
  }
  Ok(())
}
#[test]
fn test_get() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();
  let mut scratch = BytesRef::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mut strings: HashMap<String, i32> = HashMap::new();
    let mut unique_count = 0;

    for _ in 0..797 {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }

      ref_builder.copy_chars_from_string(&str_value)?;
      let count = hash.size();
      let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

      if key >= 0 {
        assert!(strings.insert(str_value.clone(), key).is_none());
        assert_eq!(unique_count, key);
        unique_count += 1;
        assert_eq!(hash.size(), count + 1);
      } else {
        assert!((-key - 1) < count);
        assert_eq!(hash.size(), count);
      }
    }

    for (key, value) in &strings {
      ref_builder.copy_chars_from_string(key)?;
      hash.get(*value, &mut scratch, &byte_block_pool)?;
      assert_eq!(*ref_builder.get_bytes_mut_ref(), scratch);
    }

    hash.clear(&mut byte_block_pool);
    assert_eq!(hash.size(), 0);
    hash.reinit()?;
  }
  Ok(())
}
#[test]
fn test_compact() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mut num_entries = 0;
    let size = 797;
    let mut bits = bit_set::BitSet::new();

    for _ in 0..size {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }

      ref_builder.copy_chars_from_string(&str_value)?;
      let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

      if key < 0 {
        assert!(bits.contains(((-key) - 1) as usize));
      } else {
        assert!(!bits.contains(key as usize));
        bits.insert(key as usize);
        num_entries += 1;
      }
    }
    assert_eq!(hash.size() as usize, bits.count());
    assert_eq!(num_entries as usize, bits.count());
    assert_eq!(num_entries, hash.size());

    let compact = hash.compact()?;
    assert!(num_entries < compact.len() as i32);

    for &id in compact {
      bits.remove(id as usize);
    }

    assert_eq!(bits.count(), 0);

    hash.clear(&mut byte_block_pool);
    assert_eq!(hash.size(), 0);
    hash.reinit()?;
  }
  Ok(())
}
#[test]
fn test_sort() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mut strings = std::collections::BTreeSet::new();

    for _ in 0..797 {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }

      ref_builder.copy_chars_from_string(&str_value)?;
      hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;
      strings.insert(str_value);
    }

    for _ in 0..3 {
      hash.sort(&byte_block_pool)?;
      let len = hash.ids.len();
      assert!(strings.len() < len);
      let mut scratch = BytesRef::new();
      for (i, string) in strings.iter().enumerate() {
        ref_builder.copy_chars_from_string(string)?;
        let bytes_id = hash.ids[i];
        hash.get(bytes_id, &mut scratch, &byte_block_pool)?;
        let sorted_ref = scratch.clone();
        assert_eq!(
          *ref_builder.get_bytes_mut_ref(),
          sorted_ref,
          "Sorted value mismatch at index {}",
          i
        );
      }
    }

    hash.clear(&mut byte_block_pool);
    assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
    hash.reinit()?;
  }
  Ok(())
}

#[test]
fn test_add() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();
  let mut scratch = BytesRef::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mut strings = HashSet::new();
    let mut unique_count = 0;

    for _ in 0..797 {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }

      ref_builder.copy_chars_from_string(&str_value)?;
      let count = hash.size();
      let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;

      if key >= 0 {
        assert!(strings.insert(str_value.clone()));
        assert_eq!(unique_count, key);
        assert_eq!(hash.size(), count + 1);
        unique_count += 1;
      } else {
        assert!(!strings.insert(str_value.clone()));
        assert!((-key - 1) < count);
        hash.get(-key - 1, &mut scratch, &byte_block_pool)?;
        assert_eq!(str_value, scratch.utf8_to_string()?);
        assert_eq!(count, hash.size());
      }
    }

    assert_all_in(&strings, &mut hash, &mut byte_block_pool)?;
    hash.clear(&mut byte_block_pool);
    assert_eq!(hash.size(), 0);
    hash.reinit()?;
  }
  Ok(())
}
#[test]
fn test_find() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();
  let mut scratch = BytesRef::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mut strings = HashSet::new();
    let mut unique_count = 0;

    for _ in 0..797 {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }

      ref_builder.copy_chars_from_string(&str_value)?;
      let count = hash.size();
      let key = hash.find(ref_builder.get_bytes_mut_ref(), &byte_block_pool)?;

      if key >= 0 {
        assert!(!strings.insert(str_value.clone()));
        assert!(key < count);
        hash.get(key, &mut scratch, &byte_block_pool)?;
        assert_eq!(str_value, scratch.utf8_to_string()?);
        assert_eq!(count, hash.size());
      } else {
        let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut byte_block_pool)?;
        assert!(strings.insert(str_value.clone()));
        assert_eq!(unique_count, key);
        assert_eq!(hash.size(), count + 1);
        unique_count += 1;
      }
    }

    assert_all_in(&strings, &mut hash, &mut byte_block_pool)?;
    hash.clear(&mut byte_block_pool);
    assert_eq!(hash.size(), 0);
    hash.reinit()?;
  }
  Ok(())
}
#[test]
fn test_concurrent_access_to_bytes_ref_hash() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 2);

  for _ in 0..num {
    let num_strings = 797;
    let mut strings = Vec::with_capacity(num_strings);
    let mut byte_block_pool = new_pool();
    let mut hash = new_hash(&mut random)?;

    for _ in 0..num_strings {
      let str_value = TestUtil::random_realistic_unicode_string_range(&mut random, 1, 1000);
      hash.add(&BytesRef::from_string(&str_value), &mut byte_block_pool)?;
      strings.push(str_value);
    }

    let hash_size = hash.size();

    let not_found = AtomicI32::new(0);
    let not_equals = AtomicI32::new(0);
    let wrong_size = AtomicI32::new(0);

    let num_threads = at_least(&mut random, 3);
    let latch = CountDownLatch::new(num_threads as usize);
    thread::scope(|scope| -> Result<()> {
      let mut handles = vec![];
      for _ in 0..num_threads {
        let hash = &hash;
        let strings = &strings;
        let not_found = &not_found;
        let not_equals = &not_equals;
        let wrong_size = &wrong_size;
        let latch = &latch;
        let loops = at_least(&mut random, 100);
        let byte_block_pool = &byte_block_pool;

        handles.push(scope.spawn(move || -> Result<()> {
          let mut scratch = BytesRef::new();
          latch.count_down();
          latch.wait();

          for k in 0..loops {
            let find = BytesRef::from_string(&strings[k as usize % strings.len()]);
            let id = hash.find(&find, byte_block_pool)?;

            if id < 0 {
              not_found.fetch_add(1, Ordering::SeqCst);
            } else {
              hash.get(id, &mut scratch, byte_block_pool)?;
              if scratch != find {
                not_equals.fetch_add(1, Ordering::SeqCst);
              }
            }
            if hash.size() != hash_size {
              wrong_size.fetch_add(1, Ordering::SeqCst);
            }
          }
          Ok(())
        }));
      }
      for handle in handles {
        handle.join().expect("Thread panicked")?;
      }
      Ok(())
    })?;

    assert_eq!(
      not_found.load(Ordering::SeqCst),
      0,
      "No entries should be missing."
    );
    assert_eq!(
      not_equals.load(Ordering::SeqCst),
      0,
      "All entries should match."
    );
    assert_eq!(
      wrong_size.load(Ordering::SeqCst),
      0,
      "Hash size should remain consistent."
    );

    hash.clear(&mut byte_block_pool);
    assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
    hash.reinit()?;
  }

  Ok(())
}
#[test]
fn test_large_value() -> Result<()> {
  let mut random = random();
  let mut byte_block_pool = new_pool();
  let mut hash = new_hash(&mut random)?;

  let sizes = [
    random.random_range(0..5),
    BYTE_BLOCK_SIZE - 33 + random.random_range(0..31),
    BYTE_BLOCK_SIZE - 1 + random.random_range(0..37),
  ];

  for (i, &size) in sizes.iter().enumerate() {
    let mut ref_bytes = BytesRef::new();
    ref_bytes.bytes = vec![0; size as usize];
    ref_bytes.offset = 0;
    ref_bytes.length = size as usize;

    match hash.add(&ref_bytes, &mut byte_block_pool) {
      Ok(key) => {
        assert!(i < sizes.len() - 1, "Expected MaxBytesLengthExceeded");
        assert_eq!(i as i32, key, "Expected index {} but got {}", i, key);
      },
      Err(e) => {
        if i < sizes.len() - 1 {
          unreachable!("Unexpected error at size: {}: {:?}", size, e);
        }
        assert!(matches!(e, LuceneError::MaxBytesLengthExceeded(_)));
      },
    }
  }

  Ok(())
}
#[test]
fn test_add_by_pool_offset() -> Result<()> {
  let mut random = random();
  let mut pool = new_pool();
  let mut hash = new_hash(&mut random)?;
  let mut offset_hash = new_hash(&mut random)?;
  let mut ref_builder = BytesRefBuilder::new();
  let mut scratch = BytesRef::new();

  let num = at_least(&mut random, 2);
  for _ in 0..num {
    let mut strings = HashSet::new();
    let mut unique_count = 0;

    for _ in 0..797 {
      let mut str_value;
      loop {
        str_value = TestUtil::random_realistic_unicode_string_with_len(&mut random, 1000);
        if !str_value.is_empty() {
          break;
        }
      }

      ref_builder.copy_chars_from_string(&str_value)?;
      let count = hash.size();
      let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut pool)?;

      if key >= 0 {
        assert!(strings.insert(str_value.clone()));
        assert_eq!(unique_count, key);
        assert_eq!(hash.size(), count + 1);

        let offset_key = offset_hash.add_by_pool_offset(hash.byte_start(key)?, &mut pool)?;
        assert_eq!(unique_count, offset_key);
        assert_eq!(offset_hash.size(), count + 1);

        unique_count += 1;
      } else {
        assert!(!strings.insert(str_value.clone()));
        assert!((-key - 1) < count);
        hash.get(-key - 1, &mut scratch, &pool)?;
        assert_eq!(str_value, scratch.utf8_to_string()?);
        assert_eq!(count, hash.size());
        let offset_key = offset_hash.add_by_pool_offset(hash.byte_start(-key - 1)?, &mut pool)?;
        assert!((-offset_key - 1) < count);
        hash.get(-offset_key - 1, &mut scratch, &pool)?;
        assert_eq!(str_value, scratch.utf8_to_string()?);
        assert_eq!(count, hash.size());
      }
    }

    assert_all_in(&strings, &mut hash, &mut pool)?;

    for string in &strings {
      ref_builder.copy_chars_from_string(string)?;
      let key = hash.add(ref_builder.get_bytes_mut_ref(), &mut pool)?;
      offset_hash.get(-key - 1, &mut scratch, &pool)?;
      let bytes_ref = scratch.clone();
      assert_eq!(
        *ref_builder.get_bytes_mut_ref(),
        bytes_ref,
        "Values should match."
      );
    }

    hash.clear(&mut pool);
    assert_eq!(hash.size(), 0, "Hash should be empty after clear.");
    offset_hash.clear(&mut pool);
    assert_eq!(
      offset_hash.size(),
      0,
      "Offset hash should be empty after clear."
    );

    hash.reinit()?;
    offset_hash.reinit()?;
  }
  Ok(())
}

fn assert_all_in(
  strings: &HashSet<String>,
  hash: &mut DirectBytesRefHash,
  pool: &mut ByteBlockPool,
) -> Result<()> {
  let mut ref_builder = BytesRefBuilder::new();
  let mut scratch = BytesRef::new();
  let count = hash.size();

  for string in strings {
    ref_builder.copy_chars_from_string(string)?;
    let key = hash.add(ref_builder.get_bytes_mut_ref(), pool)?; // add again to check duplicates
    hash.get((-key) - 1, &mut scratch, pool)?;
    assert_eq!(*string, scratch.utf8_to_string()?);
    assert_eq!(
      count,
      hash.size(),
      "Hash size should remain unchanged after duplicate insertion."
    );
    assert!(
      key < count,
      "Key {} should be less than count {}, string: {}",
      key,
      count,
      string
    );
  }

  Ok(())
}
