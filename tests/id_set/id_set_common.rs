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
// todo: should with mask
pub fn flip_bit_range(bitset: &mut bit_set::BitSet, start: usize, end: usize) {
    for i in start..end {
        flip_bit(bitset, i);
    }
}

// todo: should with mask
pub fn clear_range(bitset: &mut bit_set::BitSet, start: usize, end: usize) {
    for i in start..end {
        bitset.remove(i);
    }
}
// todo: should with mask
pub fn set_range(bitset: &mut bit_set::BitSet, start: usize, end: usize) {
    for i in start..end {
        bitset.insert(i);
    }
}
pub fn flip_bit(bitset: &mut bit_set::BitSet, index: usize) {
    if bitset.contains(index) {
        bitset.remove(index);
    } else {
        bitset.insert(index);
    }
}
