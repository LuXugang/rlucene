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
use crate::core::codecs::competitive_impact_accumulator::CompetitiveImpactAccumulator;
use crate::core::index::impact::Impact;
#[allow(dead_code)] // for quick search
struct TestCompetitiveImpactAccumulator;
#[test]
fn test_basics() {
  let mut acc = CompetitiveImpactAccumulator::new();

  acc.add(3, 5);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![Impact::new(3, 5)]
  );
  acc.add(6, 11);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![Impact::new(3, 5), Impact::new(6, 11)]
  );
  acc.add(10, 13);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![Impact::new(3, 5), Impact::new(6, 11), Impact::new(10, 13)]
  );
  acc.add(1, 2);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![
      Impact::new(1, 2),
      Impact::new(3, 5),
      Impact::new(6, 11),
      Impact::new(10, 13)
    ]
  );

  acc.add(7, 9);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![
      Impact::new(1, 2),
      Impact::new(3, 5),
      Impact::new(7, 9),
      Impact::new(10, 13)
    ]
  );

  acc.add(8, 2);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![Impact::new(8, 2), Impact::new(10, 13)]
  );
}
#[test]
fn test_extreme_norms() {
  let mut acc = CompetitiveImpactAccumulator::new();
  let mut expected = Vec::new();

  acc.add(3, 5);
  expected.push(Impact::new(3, 5));
  assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

  acc.add(10, 10000);
  expected.push(Impact::new(10, 10000));
  assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

  acc.add(5, 200);
  expected.insert(1, Impact::new(5, 200));
  assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

  acc.add(20, -100);
  expected.push(Impact::new(20, -100));
  assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

  acc.add(30, -3);
  expected.push(Impact::new(30, -3));
  assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);
}

#[test]
fn test_copy_and_merge() {
  let mut acc = CompetitiveImpactAccumulator::new();
  let mut copied_acc = CompetitiveImpactAccumulator::new();
  let mut merged_acc = CompetitiveImpactAccumulator::new();

  acc.add(3, 5);
  copied_acc.copy_from(&acc);
  assert_eq!(
    copied_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  merged_acc.add_all(&acc);
  assert_eq!(
    merged_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  acc.add(10, 10000);
  copied_acc.copy_from(&acc);
  assert_eq!(
    copied_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  merged_acc.clear();
  merged_acc.add_all(&acc);
  assert_eq!(
    merged_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  acc.add(5, 200);
  copied_acc.copy_from(&acc);
  assert_eq!(
    copied_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  merged_acc.clear();
  merged_acc.add_all(&acc);
  assert_eq!(
    merged_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  acc.add(20, -100);
  copied_acc.copy_from(&acc);
  assert_eq!(
    copied_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  merged_acc.clear();
  merged_acc.add_all(&acc);
  assert_eq!(
    merged_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  acc.add(30, -3);
  copied_acc.copy_from(&acc);
  assert_eq!(
    copied_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );

  merged_acc.clear();
  merged_acc.add_all(&acc);
  assert_eq!(
    merged_acc.get_competitive_freq_norm_pairs(),
    acc.get_competitive_freq_norm_pairs()
  );
}

#[test]
fn test_omit_freqs() {
  let mut acc = CompetitiveImpactAccumulator::new();
  acc.add(1, 5);
  acc.add(1, 7);
  acc.add(1, 4);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![Impact::new(1, 4)]
  );
}

#[test]
fn test_omit_norms() {
  let mut acc = CompetitiveImpactAccumulator::new();
  acc.add(5, 1);
  acc.add(7, 1);
  acc.add(4, 1);
  assert_eq!(
    acc.get_competitive_freq_norm_pairs(),
    vec![Impact::new(7, 1)]
  );
}
