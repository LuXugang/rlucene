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
use crate::core::codecs::codec;
use crate::core::index::merge_policy::{DefaultMergeSpecification, MergeStat, OneMerge};
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::tiered_merge_policy::SegmentDocAndID;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{LATEST, StringHelper};
use crate::test_framework::core::util::lucene_test_case::{new_directory_shared, random};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[allow(dead_code)] // for quick search
struct TestMergePolicy;

fn await_merges<D>(ms: &Arc<Mutex<DefaultMergeSpecification<D>>>, timeout: Duration) -> bool
where
  D: Directory,
{
  let merge_stats: Vec<MergeStat> = ms
    .lock()
    .merges
    .iter()
    .map(|merge| merge.stat.clone())
    .collect();
  MergeStat::await_all_with_timeout(&merge_stats, timeout)
}

#[test]
fn test_wait_for_one_merge() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let num_merges = 1 + random.random_range(0..10);
  let ms = Arc::new(Mutex::new(create_random_merge_specification(
    &mut random,
    dir,
    num_merges,
  )?));
  for m in &ms.lock().merges {
    assert!(m.has_completed_successfully().is_none());
  }
  let thread_ms = ms.clone();
  let t = thread::spawn(move || -> Result<()> {
    let mut ms = thread_ms.lock();
    for m in &mut ms.merges {
      m.close_for_test(true, false, |_| Ok(()))?;
    }
    Ok(())
  });
  assert!(await_merges(&ms, Duration::from_secs(100 * 60 * 60)));
  for m in &ms.lock().merges {
    assert!(m.has_completed_successfully().unwrap());
  }
  t.join().unwrap()?;
  Ok(())
}

#[test]
fn test_timeout() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let ms = Arc::new(Mutex::new(create_random_merge_specification(
    &mut random,
    dir,
    3,
  )?));
  for m in &ms.lock().merges {
    assert!(m.has_completed_successfully().is_none());
  }
  let thread_ms = ms.clone();
  let t = thread::spawn(move || -> Result<()> {
    thread_ms.lock().merges[0].close_for_test(true, false, |_| Ok(()))?;
    Ok(())
  });
  assert!(!await_merges(&ms, Duration::from_millis(10)));
  assert!(ms.lock().merges[1].has_completed_successfully().is_none());
  t.join().unwrap()?;
  Ok(())
}

#[test]
fn test_timeout_large_number_of_merges() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let ms = Arc::new(Mutex::new(create_random_merge_specification(
    &mut random,
    dir,
    10000,
  )?));
  for m in &ms.lock().merges {
    assert!(m.has_completed_successfully().is_none());
  }
  let i = Arc::new(AtomicUsize::new(0));
  let stop = Arc::new(AtomicBool::new(false));
  let thread_ms = ms.clone();
  let thread_i = i.clone();
  let thread_stop = stop.clone();
  let t = thread::spawn(move || -> Result<()> {
    while !thread_stop.load(Ordering::SeqCst) {
      let index = thread_i.fetch_add(1, Ordering::SeqCst);
      thread_ms.lock().merges[index].close_for_test(true, false, |_| Ok(()))?;
      thread::sleep(Duration::from_millis(1));
    }
    Ok(())
  });
  assert!(!await_merges(&ms, Duration::from_millis(10)));
  stop.store(true, Ordering::SeqCst);
  t.join().unwrap()?;
  let ms = ms.lock();
  for j in 0..ms.merges.len() {
    if j < i.load(Ordering::SeqCst) {
      assert!(ms.merges[j].has_completed_successfully().unwrap());
    } else {
      assert!(ms.merges[j].has_completed_successfully().is_none());
    }
  }
  Ok(())
}

#[test]
fn test_finish_twice() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mut spec = create_random_merge_specification(&mut random, dir, 1)?;
  let one_merge = &mut spec.merges[0];
  one_merge.close_for_test(true, false, |_| Ok(()))?;
  let err = one_merge.close_for_test(false, false, |_| Ok(()));
  assert!(matches!(
    err,
    Err(error) if error.is_illegal_state_error()
  ));
  Ok(())
}

#[test]
fn test_total_max_doc() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let spec = create_random_merge_specification(&mut random, dir, 1)?;
  let mut docs = 0;
  let one_merge = &spec.merges[0];
  for info in &one_merge.segments {
    docs += info.max_doc;
  }
  assert_eq!(docs, one_merge.total_max_doc);
  Ok(())
}

fn create_random_merge_specification<R, D>(
  random: &mut R,
  dir: Arc<D>,
  num_merges: i32,
) -> Result<DefaultMergeSpecification<D>>
where
  R: Rng + ?Sized,
  D: Directory,
{
  let mut ms = DefaultMergeSpecification::new();
  for _ii in 0..num_merges {
    let id: [u8; StringHelper::ID_LENGTH] = TestUtil::random_simple_string_range(
      random,
      StringHelper::ID_LENGTH,
      StringHelper::ID_LENGTH,
    )
    .into_bytes()
    .try_into()
    .unwrap();
    let si = SegmentInfo::new(
      dir.clone(),
      Some((*LATEST).clone()),
      Some((*LATEST).clone()),
      &TestUtil::random_simple_string(random),
      random.random_range(0..1000),
      random.random_bool(0.5),
      false,
      Some(codec::get_default()),
      HashMap::new(),
      id,
      HashMap::new(),
      None,
    )?;
    let segments = vec![SegmentCommitInfo::new(
      si,
      0,
      0,
      0,
      0,
      0,
      Some(StringHelper::random_id()),
    )];
    let mut merge_segments = Vec::new();
    for info in segments {
      merge_segments.push(SegmentDocAndID::new(
        info.info.get_id_key().to_string(),
        info.info.max_doc()?,
      ));
    }
    ms.add(OneMerge::new(merge_segments)?);
  }
  Ok(ms)
}
