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
/// Bit mixing utilities. The purpose of these methods is to evenly distribute
/// key space over the `i32` range.
///
/// Forked from `com.carrotsearch.hppc.BitMixer`
///
/// GitHub: <https://github.com/carrotsearch/hppc>
/// Release: 0.10.0
pub struct BitMixer;
impl BitMixer {
  pub const PHI_C32: u32 = 0x9e3779b9;
  pub const PHI_C64: u64 = 0x9e3779b97f4a7c15;
  /// Mix a `u8` key using simple multiplication by PHI_C32.
  pub fn mix_u8(key: u8) -> u32 {
    (key as u32).wrapping_mul(Self::PHI_C32)
  }

  /// Mix an `i8` key using phi strategy.
  pub fn mix_i8(key: i8) -> u32 {
    Self::mix_phi_i8(key)
  }

  /// Mix a `u16` key using phi strategy.
  pub fn mix_u16(key: u16) -> u32 {
    Self::mix_phi_u16(key)
  }

  /// Mix a `char` (Rust `char` is u32) key using phi strategy.
  pub fn mix_char(key: char) -> u32 {
    Self::mix_phi_i32(key as i32)
  }

  /// Better mix for larger key domains: mix an `i32` key using mix32.
  pub fn mix_i32(key: i32) -> u32 {
    Self::mix32(key as u32)
  }

  /// Mix an `f32` key using mix32 on its bit representation.
  pub fn mix_f32(key: f32) -> u32 {
    Self::mix32(key.to_bits())
  }

  /// Mix an `f64` key using mix64 on its bit representation, returning lower
  /// 32 bits.
  pub fn mix_f64(key: f64) -> u32 {
    (Self::mix64(key.to_bits()) & 0xFFFF_FFFF) as u32
  }

  /// Mix an `i64` key using mix64, returning lower 32 bits.
  pub fn mix_i64(key: i64) -> u32 {
    (Self::mix64(key as u64) & 0xFFFF_FFFF) as u32
  }
  /// MH3's finalization step (32-bit variant).
  pub fn mix32(mut k: u32) -> u32 {
    k ^= k >> 16;
    k = k.wrapping_mul(0x85eb_ca6b);
    k ^= k >> 13;
    k = k.wrapping_mul(0xc2b2_ae35);
    k ^ (k >> 16)
  }

  /// David Stafford variant 9 of 64-bit mixing function.
  /// Good distribution and efficient in hardware.
  pub fn mix64(mut z: u64) -> u64 {
    z ^= z >> 32;
    z = z.wrapping_mul(0x4cd6_944c_5cc2_0b6d);
    z ^= z >> 29;
    z = z.wrapping_mul(0xfc12_c5b1_9d32_59e9);
    z ^ (z >> 32)
  }

  /// Mix using golden ratio (φ) strategy, for small key types.
  pub fn mix_phi_u8(k: u8) -> u32 {
    let h = (k as u32).wrapping_mul(BitMixer::PHI_C32);
    h ^ (h >> 16)
  }

  pub fn mix_phi_i8(k: i8) -> u32 {
    let h = (k as i32 as u32).wrapping_mul(BitMixer::PHI_C32);
    h ^ (h >> 16)
  }

  pub fn mix_phi_u16(k: u16) -> u32 {
    let h = (k as u32).wrapping_mul(BitMixer::PHI_C32);
    h ^ (h >> 16)
  }

  pub fn mix_phi_i16(k: i16) -> u32 {
    let h = (k as i32 as u32).wrapping_mul(BitMixer::PHI_C32);
    h ^ (h >> 16)
  }

  pub fn mix_phi_i32(k: i32) -> u32 {
    let h = (k as u32).wrapping_mul(BitMixer::PHI_C32);
    h ^ (h >> 16)
  }

  pub fn mix_phi_f32(k: f32) -> u32 {
    let bits = k.to_bits();
    let h = bits.wrapping_mul(BitMixer::PHI_C32);
    h ^ (h >> 16)
  }

  pub fn mix_phi_f64(k: f64) -> u32 {
    let bits = k.to_bits();
    let h = bits.wrapping_mul(BitMixer::PHI_C64);
    ((h ^ (h >> 32)) & 0xFFFF_FFFF) as u32
  }

  pub fn mix_phi_i64(k: i64) -> u32 {
    let h = (k as u64).wrapping_mul(BitMixer::PHI_C64);
    ((h ^ (h >> 32)) & 0xFFFF_FFFF) as u32
  }
}
