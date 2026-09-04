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
use crate::core::index::BytesRef;
use crate::core::index::index_reader::Identity;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::ints_ref::IntsRef;
use bit_set::BitSet;
use num_traits::PrimInt;
use std::cmp::Ordering;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use wide::{f32x8, f64x4, i32x8, u8x32};

#[inline]
fn array_element_equals_f32(a: f32, b: f32) -> bool {
  a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[inline]
fn array_element_equals_f64(a: f64, b: f64) -> bool {
  a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

// Keep the public wrapper small enough to inline its length, short-slice, and first-element
// fast paths. The call overhead is amortized once a slice is large enough to reach this kernel.
#[inline(never)]
fn array_equals_f32_wide(a: &[f32], b: &[f32]) -> bool {
  debug_assert_eq!(a.len(), b.len());
  const LANES: usize = 8;
  const BLOCK_LANES: usize = LANES * 4;
  const ALL_LANES: u32 = (1 << LANES) - 1;
  let mut i = 0;
  // Combine four raw-bit masks before reducing them. This avoids paying for a mask reduction on
  // every vector, while the uncommon mismatch path still canonicalizes NaNs exactly like Java.
  while i + BLOCK_LANES <= a.len() {
    let raw_equal_0 = f32x8::from(&a[i..i + LANES])
      .to_bits()
      .simd_eq(f32x8::from(&b[i..i + LANES]).to_bits());
    let raw_equal_1 = f32x8::from(&a[i + LANES..i + LANES * 2])
      .to_bits()
      .simd_eq(f32x8::from(&b[i + LANES..i + LANES * 2]).to_bits());
    let raw_equal_2 = f32x8::from(&a[i + LANES * 2..i + LANES * 3])
      .to_bits()
      .simd_eq(f32x8::from(&b[i + LANES * 2..i + LANES * 3]).to_bits());
    let raw_equal_3 = f32x8::from(&a[i + LANES * 3..i + BLOCK_LANES])
      .to_bits()
      .simd_eq(f32x8::from(&b[i + LANES * 3..i + BLOCK_LANES]).to_bits());
    if (raw_equal_0 & raw_equal_1 & raw_equal_2 & raw_equal_3).to_bitmask() != ALL_LANES {
      let equal_0 = raw_equal_0
        | (f32x8::from(&a[i..i + LANES]).is_nan() & f32x8::from(&b[i..i + LANES]).is_nan())
          .to_bits();
      let equal_1 = raw_equal_1
        | (f32x8::from(&a[i + LANES..i + LANES * 2]).is_nan()
          & f32x8::from(&b[i + LANES..i + LANES * 2]).is_nan())
        .to_bits();
      let equal_2 = raw_equal_2
        | (f32x8::from(&a[i + LANES * 2..i + LANES * 3]).is_nan()
          & f32x8::from(&b[i + LANES * 2..i + LANES * 3]).is_nan())
        .to_bits();
      let equal_3 = raw_equal_3
        | (f32x8::from(&a[i + LANES * 3..i + BLOCK_LANES]).is_nan()
          & f32x8::from(&b[i + LANES * 3..i + BLOCK_LANES]).is_nan())
        .to_bits();
      if (equal_0 & equal_1 & equal_2 & equal_3).to_bitmask() != ALL_LANES {
        return false;
      }
    }
    i += BLOCK_LANES;
  }
  while i + LANES <= a.len() {
    let a_values = f32x8::from(&a[i..i + LANES]);
    let b_values = f32x8::from(&b[i..i + LANES]);
    let raw_equal = a_values.to_bits().simd_eq(b_values.to_bits());
    if raw_equal.to_bitmask() != ALL_LANES {
      let both_nan = (a_values.is_nan() & b_values.is_nan()).to_bits();
      if (raw_equal | both_nan).to_bitmask() != ALL_LANES {
        return false;
      }
    }
    i += LANES;
  }

  a[i..]
    .iter()
    .zip(&b[i..])
    .all(|(&a, &b)| array_element_equals_f32(a, b))
}

// See `array_equals_f32_wide` for the split fast-path and four-vector reduction rationale.
#[inline(never)]
fn array_equals_f64_wide(a: &[f64], b: &[f64]) -> bool {
  debug_assert_eq!(a.len(), b.len());
  const LANES: usize = 4;
  const BLOCK_LANES: usize = LANES * 4;
  const ALL_LANES: u32 = (1 << LANES) - 1;
  let mut i = 0;
  while i + BLOCK_LANES <= a.len() {
    let raw_equal_0 = f64x4::from(&a[i..i + LANES])
      .to_bits()
      .simd_eq(f64x4::from(&b[i..i + LANES]).to_bits());
    let raw_equal_1 = f64x4::from(&a[i + LANES..i + LANES * 2])
      .to_bits()
      .simd_eq(f64x4::from(&b[i + LANES..i + LANES * 2]).to_bits());
    let raw_equal_2 = f64x4::from(&a[i + LANES * 2..i + LANES * 3])
      .to_bits()
      .simd_eq(f64x4::from(&b[i + LANES * 2..i + LANES * 3]).to_bits());
    let raw_equal_3 = f64x4::from(&a[i + LANES * 3..i + BLOCK_LANES])
      .to_bits()
      .simd_eq(f64x4::from(&b[i + LANES * 3..i + BLOCK_LANES]).to_bits());
    if (raw_equal_0 & raw_equal_1 & raw_equal_2 & raw_equal_3).to_bitmask() != ALL_LANES {
      let equal_0 = raw_equal_0
        | (f64x4::from(&a[i..i + LANES]).is_nan() & f64x4::from(&b[i..i + LANES]).is_nan())
          .to_bits();
      let equal_1 = raw_equal_1
        | (f64x4::from(&a[i + LANES..i + LANES * 2]).is_nan()
          & f64x4::from(&b[i + LANES..i + LANES * 2]).is_nan())
        .to_bits();
      let equal_2 = raw_equal_2
        | (f64x4::from(&a[i + LANES * 2..i + LANES * 3]).is_nan()
          & f64x4::from(&b[i + LANES * 2..i + LANES * 3]).is_nan())
        .to_bits();
      let equal_3 = raw_equal_3
        | (f64x4::from(&a[i + LANES * 3..i + BLOCK_LANES]).is_nan()
          & f64x4::from(&b[i + LANES * 3..i + BLOCK_LANES]).is_nan())
        .to_bits();
      if (equal_0 & equal_1 & equal_2 & equal_3).to_bitmask() != ALL_LANES {
        return false;
      }
    }
    i += BLOCK_LANES;
  }
  while i + LANES <= a.len() {
    let a_values = f64x4::from(&a[i..i + LANES]);
    let b_values = f64x4::from(&b[i..i + LANES]);
    let raw_equal = a_values.to_bits().simd_eq(b_values.to_bits());
    if raw_equal.to_bitmask() != ALL_LANES {
      let both_nan = (a_values.is_nan() & b_values.is_nan()).to_bits();
      if (raw_equal | both_nan).to_bitmask() != ALL_LANES {
        return false;
      }
    }
    i += LANES;
  }

  a[i..]
    .iter()
    .zip(&b[i..])
    .all(|(&a, &b)| array_element_equals_f64(a, b))
}

pub struct CoreHelper;
impl CoreHelper {
  /// Compares like Java's `Float.compare`: all NaNs are equal and greater than
  /// positive infinity, and negative zero sorts before positive zero.
  #[inline]
  pub fn compare_f32(a: f32, b: f32) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
      (true, true) => Ordering::Equal,
      (true, false) => Ordering::Greater,
      (false, true) => Ordering::Less,
      (false, false) => a.total_cmp(&b),
    }
  }

  /// Compares like Java's `Double.compare`: all NaNs are equal and greater than
  /// positive infinity, and negative zero sorts before positive zero.
  #[inline]
  pub fn compare_f64(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
      (true, true) => Ordering::Equal,
      (true, false) => Ordering::Greater,
      (false, true) => Ordering::Less,
      (false, false) => a.total_cmp(&b),
    }
  }

  /// Returns the greater value with Java `Math.max(float, float)` semantics.
  #[inline]
  pub fn max_f32(a: f32, b: f32) -> f32 {
    if a.is_nan() {
      a
    } else if b.is_nan() {
      b
    } else if a == 0.0 && b == 0.0 {
      if a.is_sign_positive() { a } else { b }
    } else if a >= b {
      a
    } else {
      b
    }
  }

  /// Returns the lesser value with Java `Math.min(float, float)` semantics.
  #[inline]
  pub fn min_f32(a: f32, b: f32) -> f32 {
    if a.is_nan() {
      a
    } else if b.is_nan() {
      b
    } else if a == 0.0 && b == 0.0 {
      if a.is_sign_positive() { b } else { a }
    } else if a <= b {
      a
    } else {
      b
    }
  }

  /// Returns the greater value with Java `Math.max(double, double)` semantics.
  #[inline]
  pub fn max_f64(a: f64, b: f64) -> f64 {
    if a.is_nan() {
      a
    } else if b.is_nan() {
      b
    } else if a == 0.0 && b == 0.0 {
      if a.is_sign_positive() { a } else { b }
    } else if a >= b {
      a
    } else {
      b
    }
  }

  /// Returns the lesser value with Java `Math.min(double, double)` semantics.
  #[inline]
  pub fn min_f64(a: f64, b: f64) -> f64 {
    if a.is_nan() {
      a
    } else if b.is_nan() {
      b
    } else if a == 0.0 && b == 0.0 {
      if a.is_sign_positive() { b } else { a }
    } else if a <= b {
      a
    } else {
      b
    }
  }

  /// Returns canonical bits suitable for hashing a float whose equality uses
  /// Java primitive `==`: signed zeros are equal, so they share one hash.
  #[inline]
  pub fn hash_bits_f32_for_primitive_eq(value: f32) -> u32 {
    BitUtil::float_to_int_bits(if value == 0.0 { 0.0 } else { value }) as u32
  }

  /// Returns canonical bits suitable for hashing a double whose equality uses
  /// Java primitive `==`: signed zeros are equal, so they share one hash.
  #[inline]
  pub fn hash_bits_f64_for_primitive_eq(value: f64) -> u64 {
    BitUtil::double_to_long_bits(if value == 0.0 { 0.0 } else { value }) as u64
  }

  /// Compares slices like Java's `Arrays.equals(float[], float[])`.
  #[inline]
  pub fn array_equals_f32(a: &[f32], b: &[f32]) -> bool {
    if a.len() != b.len() {
      return false;
    }
    if a.is_empty() {
      return true;
    }
    if !array_element_equals_f32(a[0], b[0]) {
      return false;
    }
    if std::ptr::eq(a, b) {
      return true;
    }
    if a.len() < 32 {
      return a[1..]
        .iter()
        .zip(&b[1..])
        .all(|(&a, &b)| array_element_equals_f32(a, b));
    }
    array_equals_f32_wide(a, b)
  }

  /// Compares slices like Java's `Arrays.equals(double[], double[])`.
  #[inline]
  pub fn array_equals_f64(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() {
      return false;
    }
    if a.is_empty() {
      return true;
    }
    if !array_element_equals_f64(a[0], b[0]) {
      return false;
    }
    if std::ptr::eq(a, b) {
      return true;
    }
    if a.len() < 16 {
      return a[1..]
        .iter()
        .zip(&b[1..])
        .all(|(&a, &b)| array_element_equals_f64(a, b));
    }
    array_equals_f64_wide(a, b)
  }

  pub const CLONE_WARRING: &'static str = "does not implement the Clone logic.
The purpose of implementing the Clone trait is to make it could be used with Cow";
  pub fn check_from_index_size(from_index: usize, size: usize, length: usize) -> Result<usize> {
    if from_index > length || size > length - from_index {
      Err(LuceneError::array_index_out_of_bounds(format!(
        "size: {size} is too large, from_index: {from_index}, length: {length}"
      )))
    } else {
      Ok(from_index)
    }
  }
  pub fn check_from_to_index(from_index: usize, to_index: usize, length: usize) -> Result<usize> {
    if from_index > to_index || to_index > length {
      return Err(LuceneError::array_index_out_of_bounds(format!(
        "index out of bounds: from_index={from_index} to_index={to_index} length={length}"
      )));
    }
    Ok(from_index)
  }
  pub fn check_index<I>(index: I, length: I) -> Result<I>
  where
    I: PrimInt + std::fmt::Display,
  {
    if index < I::zero() || length < I::zero() || index >= length {
      return Err(LuceneError::array_index_out_of_bounds(format!(
        "index out of bounds: index={index} length={length}"
      )));
    }
    Ok(index)
  }
  pub fn miss_match_u8(a: &[u8], b: &[u8]) -> i32 {
    let common_len = a.len().min(b.len());
    let mut i = 0;
    while i + 32 <= common_len {
      let equal = u8x32::from(&a[i..i + 32]).simd_eq(u8x32::from(&b[i..i + 32]));
      let mismatch = !equal.to_bitmask();
      if mismatch != 0 {
        return (i + mismatch.trailing_zeros() as usize) as i32;
      }
      i += 32;
    }

    while i < common_len && a[i] == b[i] {
      i += 1;
    }
    if i < common_len || a.len() != b.len() {
      i as i32
    } else {
      -1
    }
  }

  pub fn miss_match_i32(a: &[i32], b: &[i32]) -> i32 {
    let common_len = a.len().min(b.len());
    let mut i = 0;
    while i + 8 <= common_len {
      let equal = i32x8::from(&a[i..i + 8]).simd_eq(i32x8::from(&b[i..i + 8]));
      let mismatch = (!equal.to_bitmask()) & 0xff;
      if mismatch != 0 {
        return (i + mismatch.trailing_zeros() as usize) as i32;
      }
      i += 8;
    }

    while i < common_len && a[i] == b[i] {
      i += 1;
    }
    if i < common_len || a.len() != b.len() {
      i as i32
    } else {
      -1
    }
  }

  pub fn take_and_reset<T, F>(target: &mut T, reset_fn: F) -> T
  where
    T: Default,
    F: FnOnce(&T) -> T,
  {
    let old = std::mem::take(target);
    *target = reset_fn(&old);
    old
  }
  pub fn calculate_hash<T>(value: &T) -> u64
  where
    T: Hash + ?Sized,
  {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
  }
}

pub trait ToInt {
  fn to_int(&self) -> i32;
}

impl ToInt for Ordering {
  fn to_int(&self) -> i32 {
    match self {
      Ordering::Less => -1,
      Ordering::Equal => 0,
      Ordering::Greater => 1,
    }
  }
}

/// Extension trait for `Option<T>` that provides a convenient way to
/// temporarily take ownership of the inner value, operate on it, and
/// then restore it back into the `Option`.
///
/// This is useful for patterns where you need a mutable borrow of the
/// contents, perform some fallible operation, and then put the value
/// back—without panicking or manually handling `take()` / `replace()`.
pub trait OptionTakeExt<T> {
  /// Takes the inner `T` out of the `Option`, leaving `None` in its place,
  /// then calls the provided closure `f` on a mutable reference to that `T`.
  ///
  /// - If the `Option` was `Some(val)`, runs `f(&mut val)`, restores
  ///   `Some(val)`, and returns the closure’s `Result<R>`.
  /// - If the `Option` was `None`, returns an `Err` with a
  ///   `LuceneError::illegal_state("Option was None".to_string())`.
  ///
  /// # Errors
  ///
  /// Returns `Err(LuceneError::illegal_state)` if the `Option` is empty,
  /// or propagates any `Err` returned by the closure.
  fn take_do_return<R>(&mut self, f: impl FnOnce(&mut T) -> Result<R>) -> Result<R>;
}

impl<T> OptionTakeExt<T> for Option<T> {
  /// Implementation of `take_do_return` for all `Option<T>`.
  ///
  /// 1. Calls `self.take()` to extract the value (or return an error if
  ///    `None`).
  /// 2. Runs the user-provided closure on a mutable reference to the value.
  /// 3. Restores the value back into `self` regardless of success or failure.
  /// 4. Returns the `Result<R>` produced by the closure.
  fn take_do_return<R>(&mut self, f: impl FnOnce(&mut T) -> Result<R>) -> Result<R> {
    let mut val = self
      .take()
      .ok_or_else(|| LuceneError::illegal_state("Option was None"))?;
    let res = f(&mut val);
    *self = Some(val);
    res
  }
}
pub trait BitSetExt {
  fn next_set_bit(&self, from: usize) -> i32;
}
impl BitSetExt for BitSet {
  // TODO: this method Need optimization
  fn next_set_bit(&self, from: usize) -> i32 {
    match self.iter().find(|&bit| bit >= from) {
      Some(bit) => bit as i32,
      None => -1,
    }
  }
}
pub trait OutputIdentity {
  fn is_same_reference(&self, other: &Self) -> bool;
}
impl OutputIdentity for Arc<i64> {
  fn is_same_reference(&self, other: &Self) -> bool {
    Arc::ptr_eq(self, other)
  }
}
impl OutputIdentity for BytesRef<Arc<Vec<u8>>> {
  fn is_same_reference(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.bytes, &other.bytes)
      && self.offset == other.offset
      && self.length == other.length
  }
}

impl OutputIdentity for IntsRef<Arc<Vec<i32>>> {
  fn is_same_reference(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.ints, &other.ints)
      && self.offset == other.offset
      && self.length == other.length
  }
}

pub trait HashCode {
  fn hash_code(&self) -> i32;
}
// i32
impl HashCode for i32 {
  fn hash_code(&self) -> i32 {
    *self
  }
}
// i64
impl HashCode for i64 {
  fn hash_code(&self) -> i32 {
    let value = *self as u64;
    let high = value >> 32;
    let mixed = value ^ high;
    (mixed & 0xFFFF_FFFF) as i32
  }
}
// Arc<i64>
impl HashCode for Arc<i64> {
  fn hash_code(&self) -> i32 {
    (**self).hash_code()
  }
}

#[derive(Clone)]
pub struct IdentityArc<T> {
  pub object: Arc<T>,
}
impl<T> IdentityArc<T> {
  pub fn new(object: Arc<T>) -> Self {
    Self { object }
  }
}
impl<T> PartialEq for IdentityArc<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::as_ptr(&self.object) == Arc::as_ptr(&other.object)
  }
}
impl<T> Eq for IdentityArc<T> {}

impl<T> Hash for IdentityArc<T> {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    Arc::as_ptr(&self.object).hash(state)
  }
}
pub trait TryIntoInt<T> {
  fn try_convert(self) -> Result<T>;
}
macro_rules! impl_try_convert {
  ($src:ty => $dst:ty) => {
    impl TryIntoInt<$dst> for $src {
      #[inline]
      fn try_convert(self) -> Result<$dst> {
        <$dst>::try_from(self).map_err(|_| {
          LuceneError::illegal_state(format!(
            "value {} does not fit into {}",
            self,
            stringify!($dst)
          ))
        })
      }
    }
  };
}
impl_try_convert!(usize => i32);
impl_try_convert!(usize => i64);
impl_try_convert!(usize => u64);
impl_try_convert!(usize => u32);
impl_try_convert!(u64 => i64);
impl_try_convert!(u64 => usize);
impl_try_convert!(i64   => i32);
impl_try_convert!(i64   => usize);
impl_try_convert!(i32   => usize);
impl_try_convert!(u32   => i32);
impl_try_convert!(i64 => u8);

pub trait HasIdentity {
  fn identity(&self) -> &Identity;

  #[inline]
  fn is_same_identity<T>(&self, other: &T) -> bool
  where
    Self: Sized,
    T: HasIdentity + ?Sized,
  {
    self.identity() == other.identity()
  }
}

impl<T> HasIdentity for Arc<T>
where
  T: HasIdentity,
{
  fn identity(&self) -> &Identity {
    (**self).identity()
  }
}
impl<T> HasIdentity for &T
where
  T: HasIdentity,
{
  fn identity(&self) -> &Identity {
    (**self).identity()
  }
}
#[macro_export]
macro_rules! impl_from_for_enum {
    ($enum_ty:ident, $( $src_ty:ty => $variant:ident ),+ $(,)?) => {
        $(
            impl From<$src_ty> for $enum_ty {
                fn from(v: $src_ty) -> Self {
                    $enum_ty::$variant(v)
                }
            }
        )+
    };
}
