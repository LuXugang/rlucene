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
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{BytesRefComparator, ToInt};

pub trait Comparator<T> {
    /// A static string that identifies the type of comparator.
    const TYPE: &'static str;

    /// Compares two values and returns the result as `Result<i32>`.
    ///
    /// This method is fallible to support cases where comparison may fail,
    /// such as dynamic comparator logic or I/O-dependent comparisons.
    /// For most simple comparators (e.g., numerical or lexical), this
    /// will always return `Ok(result)`.
    fn compare(&self, a: &T, b: &T) -> Result<i32>;

    /// Unwraps the result of `compare` and panics if an error occurs.
    ///
    /// This is a convenience method for use cases where failure is not
    /// expected, which is the common case for most statically defined
    /// comparators.
    ///
    /// # Panics
    ///
    /// Panics if [`compare`](Self::compare) returns `Err`. Only use this when you are sure
    /// the comparison cannot fail.
    ///
    /// # Why this method exists
    ///
    /// Most comparator implementations are infallible. However, to support
    /// advanced use cases (e.g. pluggable or script-based comparators),
    /// the main [`compare`](Self::compare) method returns a `Result<i32>`.
    /// This method provides a cleaner, ergonomic way to call the comparator
    /// in contexts where no error is expected.
    fn compare_unchecked(&self, a: &T, b: &T) -> i32 {
        self.compare(a, b).expect("Comparator failed unexpectedly")
    }
}

pub struct NaturalOrder<T>
where
    T: Ord,
{
    _t: std::marker::PhantomData<T>,
}

impl<T> Default for NaturalOrder<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NaturalOrder<T>
where
    T: Ord,
{
    pub fn new() -> NaturalOrder<T> {
        NaturalOrder {
            _t: std::marker::PhantomData,
        }
    }
}
impl<T> Comparator<T> for NaturalOrder<T>
where
    T: Ord,
{
    const TYPE: &'static str = COMPARATOR_TYPE;

    fn compare(&self, a: &T, b: &T) -> Result<i32> {
        Ok(a.cmp(b).to_int())
    }
}

pub struct ReverseOrder<T>
where
    T: Ord,
{
    comparator: NaturalOrder<T>,
}

impl<T> Default for ReverseOrder<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ReverseOrder<T>
where
    T: Ord,
{
    pub fn new() -> ReverseOrder<T> {
        ReverseOrder {
            comparator: NaturalOrder::new(),
        }
    }
}

impl<T> Comparator<T> for ReverseOrder<T>
where
    T: Ord,
{
    const TYPE: &'static str = "ReverseOrder";

    fn compare(&self, a: &T, b: &T) -> Result<i32> {
        Ok(-self.comparator.compare(a, b)?)
    }
}

/// # NOTE
/// The purpose of implementing BytesRefComparator is to
/// allow it to be passed as the same parameter alongside other types
/// that also implement BytesRefComparator, distinguishing its type by the TYPE
/// constant.
impl BytesRefComparator for NaturalOrder<BytesRef<Vec<u8>>> {}

pub const COMPARATOR_TYPE: &str = "Comparator";
