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
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Math utility methods.
pub struct MathUtil;

impl MathUtil {
  /// Returns `x <= 0 ? 0 : floor(log(x) / log(base))`
  ///
  /// # Parameters
  /// - `x`: The number to compute the logarithm for.
  /// - `base`: The logarithm base, must be greater than 1.
  ///
  /// # Returns
  /// - The integer part of the logarithm of `x` in the given `base`.
  ///
  /// # Panics
  /// - If `base <= 1`, it will panic.
  pub fn log(mut x: i64, base: i32) -> Result<i32> {
    if base == 2 {
      // This specialized method is significantly faster.
      return if x <= 0 {
        Ok(0)
      } else {
        Ok((63 - x.leading_zeros()) as i32)
      };
    } else if base <= 1 {
      return Err(LuceneError::illegal_argument("base must be > 1"));
    }

    let mut ret = 0;
    while x >= base as i64 {
      x /= base as i64;
      ret += 1;
    }
    Ok(ret)
  }

  /// Calculates logarithm in a given base with floating-point numbers.
  ///
  /// # Parameters
  /// - `base`: The logarithm base.
  /// - `x`: The number to compute the logarithm for.
  ///
  /// # Returns
  /// - The logarithm of `x` in the given `base`.
  pub fn log_f64(base: f64, x: f64) -> f64 {
    x.ln() / base.ln()
  }
  /// Returns the greatest common divisor (GCD) of `a` and `b`,
  ///
  /// # Notes
  /// - A GCD must be positive, but `2^64` cannot be expressed as an `i64`,
  ///   although it is the GCD of `i64::MIN` and `0`, as well as `i64::MIN`
  ///   and `i64::MIN`. In these two cases, this method returns `i64::MIN`.
  pub fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.wrapping_abs();
    b = b.wrapping_abs();
    if a == 0 {
      return b;
    } else if b == 0 {
      return a;
    }

    let common_trailing_zeros = (a | b).trailing_zeros();
    a = (a as u64 >> a.trailing_zeros()) as i64;
    while b != 0 {
      b = (b as u64 >> b.trailing_zeros()) as i64;
      if a == b {
        break;
      } else if a > b || a == i64::MIN {
        std::mem::swap(&mut a, &mut b);
      }
      if a == 1 {
        break;
      }
      b = b.wrapping_sub(a);
    }
    a << common_trailing_zeros
  }

  /// Calculates the inverse hyperbolic sine (`asinh`) of a `f64` value.
  pub fn asinh(a: f64) -> f64 {
    // check the sign bit of the raw representation to handle -0
    let sign = if (a.to_bits() as i64) < 0 { -1.0 } else { 1.0 };
    sign * (f64::sqrt(a.abs() * a.abs() + 1.0) + a.abs()).ln()
  }

  /// Calculates the inverse hyperbolic cosine (`acosh`) of a `f64` value.
  pub fn acosh(a: f64) -> f64 {
    f64::ln(f64::sqrt(a * a - 1.0) + a)
  }

  /// Calculates the inverse hyperbolic tangent (`atanh`) of a `f64` value.
  pub fn atanh(a: f64) -> f64 {
    // check the sign bit of the raw representation to handle -0
    let mult = if (a.to_bits() as i64) < 0 { -0.5 } else { 0.5 };
    mult * ((1.0 + a.abs()) / (1.0 - a.abs())).ln()
  }

  /// Returns a relative error bound for the sum of `num_values` positive
  /// doubles computed using recursive summation.
  ///
  /// # Notes
  /// - This only works if all values are positive.
  /// - Uses formula 3.5 from Higham (1993), "The accuracy of floating point
  ///   summation".
  pub fn sum_relative_error_bound(num_values: i32) -> f64 {
    if num_values <= 1 {
      return 0.0;
    }
    // Machine epsilon (unit roundoff)
    let u = f64::from_bits(0x3CA0000000000000); // 2^-52
    (num_values - 1) as f64 * u
  }

  /// Returns the maximum possible sum across `num_values` non-negative
  /// doubles, assuming one sum yielded `sum`.
  pub fn sum_upper_bound(sum: f64, num_values: i32) -> f64 {
    if num_values <= 2 {
      return sum;
    }

    let b = MathUtil::sum_relative_error_bound(num_values);
    (1.0 + 2.0 * b) * sum
  }
}
