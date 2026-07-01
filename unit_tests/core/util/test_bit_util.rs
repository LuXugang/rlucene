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
use crate::core::util::bit_util::BitUtil;

pub struct TestBitUtil;

#[test]
fn test_is_zero_or_power_of_two() {
  assert!(BitUtil::is_zero_or_power_of_two(0));
  for shift in 0..=31 {
    assert!(BitUtil::is_zero_or_power_of_two(1_i32.wrapping_shl(shift)));
  }
  assert!(!BitUtil::is_zero_or_power_of_two(3));
  assert!(!BitUtil::is_zero_or_power_of_two(5));
  assert!(!BitUtil::is_zero_or_power_of_two(6));
  assert!(!BitUtil::is_zero_or_power_of_two(7));
  assert!(!BitUtil::is_zero_or_power_of_two(9));
  assert!(!BitUtil::is_zero_or_power_of_two(i32::MAX));
  assert!(!BitUtil::is_zero_or_power_of_two(i32::MAX.wrapping_add(2)));
  assert!(!BitUtil::is_zero_or_power_of_two(-1));
}
