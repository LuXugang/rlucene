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
use crate::core::search::total_hits::Relation::{EqualTo, GreaterThanOrEqualTo};
use crate::core::search::total_hits::TotalHits;
use crate::core::util::CoreHelper;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::Rng;
use rand::RngExt;
use rand::prelude::IndexedRandom;
#[allow(dead_code)] // for quick search
struct TestTotalHits;
#[test]
fn test_equals_and_hashcode() {
  let mut random = random();
  let total_hits1 = random_total_hits(&mut random);

  assert_eq!(total_hits1, total_hits1);

  assert_eq!(
    CoreHelper::calculate_hash(&total_hits1),
    CoreHelper::calculate_hash(&total_hits1)
  );

  let total_hits2 = TotalHits::new(total_hits1.value(), total_hits1.relation());

  assert_eq!(total_hits1, total_hits2);
  assert_eq!(total_hits2, total_hits1);
  assert_eq!(
    CoreHelper::calculate_hash(&total_hits1),
    CoreHelper::calculate_hash(&total_hits2)
  );

  let total_hits4 = random_total_hits(&mut random);

  if total_hits4.value() == total_hits1.value() && total_hits4.relation() == total_hits1.relation()
  {
    assert_eq!(total_hits1, total_hits4);
    assert_eq!(total_hits2, total_hits4);
    assert_eq!(
      CoreHelper::calculate_hash(&total_hits1),
      CoreHelper::calculate_hash(&total_hits4)
    );
    assert_eq!(
      CoreHelper::calculate_hash(&total_hits2),
      CoreHelper::calculate_hash(&total_hits4)
    );
  } else {
    assert_ne!(total_hits1, total_hits4);
    assert_ne!(total_hits2, total_hits4);
    assert_ne!(
      CoreHelper::calculate_hash(&total_hits1),
      CoreHelper::calculate_hash(&total_hits4)
    );
    assert_ne!(
      CoreHelper::calculate_hash(&total_hits2),
      CoreHelper::calculate_hash(&total_hits4)
    );
  }
}
fn random_total_hits<R>(random: &mut R) -> TotalHits
where
  R: Rng + ?Sized,
{
  let value = random.random_range(0..=i64::MAX) as usize;
  let relation = *[EqualTo, GreaterThanOrEqualTo].choose(random).unwrap();

  TotalHits::new(value, relation)
}
