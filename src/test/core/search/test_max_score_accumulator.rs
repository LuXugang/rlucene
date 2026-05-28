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
use crate::core::search::max_score_accumulator::MaxScoreAccumulator;

#[allow(dead_code)] // for quick search
struct TestMaxScoreAccumulator;
#[test]
fn test_simple() {
  let acc = MaxScoreAccumulator::new();

  acc.accumulate(0, 0.0);
  assert_eq!(0.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(0, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(10, 0.0);
  assert_eq!(0.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(0, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(100, 1000.0);
  assert_eq!(1000.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(100, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(1000, 5.0);
  assert_eq!(1000.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(100, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(99, 1000.0);
  assert_eq!(1000.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(99, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(1000, 1001.0);
  assert_eq!(1001.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(1000, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(10, 1001.0);
  assert_eq!(1001.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(10, MaxScoreAccumulator::doc_id(acc.get_raw()));

  acc.accumulate(100, 1001.0);
  assert_eq!(1001.0, MaxScoreAccumulator::to_score(acc.get_raw()));
  assert_eq!(10, MaxScoreAccumulator::doc_id(acc.get_raw()));
}
