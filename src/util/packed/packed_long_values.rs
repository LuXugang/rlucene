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
pub const DEFAULT_PAGE_SIZE: u32 = 256;
const MIN_PAGE_SIZE: u32 = 64;
const MAX_PAGE_SIZE: u32 = 1 << 20;
pub struct PackedLongValues;

// impl PackedLongValues {
//     pub fn packed_long_values_builder(page_size: u32, acceptable_overhead_ratio:f32) -> PackedLongValuesBuilder {
//         todo!()
//     }
// }
//
// pub struct PackedLongValuesBuilder {
//
//     // Fields
//     page_shift: u32,
//     page_mask: u32,
//     acceptable_overhead_ratio: f32,
//     pending: Vec<i64>,
//     size: u64,
//     values: Vec<PackedIntsReader>,
//     ram_bytes_used: usize,
//     values_off: usize,
//     pending_off: usize,
// }
//
// impl PackedLongValues {
//     const INITIAL_PAGE_COUNT: usize = 16;
//     // TODO
//     const BASE_RAM_BYTES_USED: u64= 0;
//     /// Constructor for Builder
//     pub fn new(page_size: usize, acceptable_overhead_ratio: f32) -> Self {
//         let page_shift = Self::check_block_size(page_size, MIN_PAGE_SIZE, MAX_PAGE_SIZE);
//         let page_mask = (page_size - 1) as u32;
//         let pending = vec![0; page_size];
//         let values = Vec::with_capacity(Self::INITIAL_PAGE_COUNT);
//
//         let base_ram_bytes_used = std::mem::size_of::<Self>();
//         let pending_ram_bytes = pending.len() * std::mem::size_of::<i64>();
//         let values_ram_bytes = values.capacity() * std::mem::size_of::<PackedIntsReader>();
//
//         Self {
//             page_shift,
//             page_mask,
//             acceptable_overhead_ratio,
//             pending,
//             size: 0,
//             values,
//             ram_bytes_used: base_ram_bytes_used + pending_ram_bytes + values_ram_bytes,
//             values_off: 0,
//             pending_off: 0,
//         }
//     }
//
//     /// Validates block size
//     fn check_block_size(page_size: usize, min_page_size: usize, max_page_size: usize) -> u32 {
//         if page_size < min_page_size || page_size > max_page_size || !page_size.is_power_of_two() {
//             panic!("Page size must be a power of 2 between {} and {}", min_page_size, max_page_size);
//         }
//         page_size.trailing_zeros()
//     }
// }
