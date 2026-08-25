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
use crate::core::util::packed::abstract_paged_mutable::AbstractPagedMutableBase;
use crate::core::util::packed::mutable_enum::MutableEnum;
use crate::core::util::packed::{Format, FormatAndBits, PackedInts, fastest_format_and_bits};
/// A [`PagedMutable`]. This structure slices data into fixed-size blocks which
/// have the same number of bits per value. It can be a useful replacement for
/// `PackedIntsMutable` to store more than 2 billion values.
///
/// # Lucene Internal
/// This is an internal utility for use within the Lucene system.
#[derive(Default)]
pub struct PagedMutable {
  format: Format,
  bits_per_value: i32,
}
impl PagedMutable {
  pub fn with_overhead_ratio(
    page_size: i32,
    bits_per_value: i32,
    acceptable_overhead_ratio: f32,
  ) -> Self {
    let format_and_bits =
      fastest_format_and_bits(page_size, bits_per_value, acceptable_overhead_ratio);
    Self::with_format_and_bits(format_and_bits)
  }
  fn with_format_and_bits(format_and_bits: FormatAndBits) -> Self {
    Self::with_bits_and_format(format_and_bits.bits_per_value, format_and_bits.format)
  }
  fn with_bits_and_format(bits_per_value: i32, format: Format) -> Self {
    Self {
      format,
      bits_per_value,
    }
  }
}
impl AbstractPagedMutableBase for PagedMutable {
  fn new_mutable(&self, value_count: i32, bits_per_value: i32) -> MutableEnum {
    debug_assert!(self.bits_per_value >= bits_per_value);
    let sub_mutable = PackedInts::get_mutable_impl(value_count, self.bits_per_value, self.format);
    MutableEnum::Packed(sub_mutable)
  }

  fn new_unfilled_copy(&self) -> Self {
    PagedMutable::with_bits_and_format(self.bits_per_value, self.format)
  }

  fn base_ram_bytes_used_base(&self) -> i64 {
    0
  }

  fn fill_pages(&self) -> bool {
    true
  }

  fn bits_per_value(&self) -> i32 {
    self.bits_per_value
  }
}
