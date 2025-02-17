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
use crate::util::array_tim_sorter::ArrayTimSorter;
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::selector::Selector;
use crate::util::{
    ArrayIntroSorter, Comparator, IntroSelector, IntroSelectorBase, IntroSelectorBaseDefault,
    NaturalOrder, Sorter, TimSorter, VecCopyOps,
};
use std::cmp::Ordering;
use std::mem;

pub struct ArrayUtil;
impl ArrayUtil {
    pub const MAX_ARRAY_LENGTH: i32 = i32::MAX;
    const MIN_RADIX: i32 = 2;
    const MAX_RADIX: i32 = 36;
    /// Parses a char array into an i32 with the default radix of 10.
    ///
    /// # Arguments
    ///
    /// * `chars` - The character array to parse.
    /// * `offset` - The starting offset in the array.
    /// * `len` - The length of the array to parse.
    ///
    /// # Returns
    ///
    /// * `i32` - The parsed integer.
    ///
    /// # Errors
    ///
    /// Returns a `LuceneError::NumberFormat` if it can't parse the chars into an integer.
    pub fn parse_int_default(chars: &[char], offset: i32, len: i32) -> Result<i32, LuceneError> {
        Self::parse_int(chars, offset, len, 10)
    }

    /// Parses the string argument as if it were an `i32` value and returns the result.
    /// Throws a `LuceneError::NumberFormat` if the string does not represent an `i32` quantity.
    /// The second argument specifies the radix to use when parsing the value.
    pub fn parse_int(
        chars: &[char],
        mut offset: i32,
        mut len: i32,
        radix: i32,
    ) -> Result<i32, LuceneError> {
        if !(ArrayUtil::MIN_RADIX..=ArrayUtil::MAX_RADIX).contains(&radix) {
            return Err(LuceneError::number_format("Invalid radix"));
        }
        if len == 0 {
            return Err(LuceneError::number_format("chars length is 0"));
        }
        let mut i = 0;
        let negative = chars[(offset + i) as usize] == '-';
        if negative {
            i += 1;
            if i == len {
                return Err(LuceneError::number_format("chars length is 0"));
            }
        }
        if negative {
            offset += 1;
            len -= 1;
        }
        Self::parse(chars, offset, len, radix, negative)
    }

    pub fn parse(
        chars: &[char],
        offset: i32,
        len: i32,
        radix: i32,
        negative: bool,
    ) -> Result<i32, LuceneError> {
        let max = i32::MIN / radix;
        let mut result = 0;
        for i in 0..len {
            let digit = chars[(offset + i) as usize]
                .to_digit(radix as u32)
                .ok_or_else(|| LuceneError::number_format("Unable to parse"))?;
            if max > result {
                return Err(LuceneError::number_format("Unable to parse"));
            }

            let next = result
                .checked_mul(radix)
                .and_then(|x| x.checked_sub(digit as i32));

            match next {
                Some(next) => {
                    if next > result {
                        return Err(LuceneError::number_format("Unable to parse"));
                    }
                    result = next;
                }
                None => return Err(LuceneError::number_format("Unable to parse")),
            }
        }
        if !negative {
            result = -result;
            if result < 0 {
                return Err(LuceneError::number_format("Unable to parse"));
            }
        }
        Ok(result)
    }
    /// Calculates the new capacity after resizing.
    ///
    /// This method simply doubles the `min_target_size` to achieve the new capacity.
    /// However, this is a basic resizing strategy that may not be suitable for all scenarios.
    ///
    /// Currently, `saturating_mul(2)` is used to avoid overflow, but if the new capacity exceeds `i32::MAX`,
    /// it will return `i32::MAX`.
    ///
    /// In the future, this resizing strategy can be improved to adapt to different element sizes
    /// or use a more intelligent growth factor.
    ///
    /// # Parameters
    /// - `min_target_size`: The minimum desired target capacity.
    /// - `_bytes_per_element`: The number of bytes per element (currently unused, reserved for future improvements).
    ///
    /// # Returns
    /// The new capacity after resizing. If the result exceeds `i32::MAX`, `i32::MAX` is returned.
    pub fn oversize(min_target_size: i32, _bytes_per_element: i32) -> i32 {
        min_target_size.saturating_mul(2)
    }
    pub fn grow_exact<T>(vec: &mut Vec<T>, new_length: i32) -> Result<(), LuceneError>
    where
        T: Default,
    {
        debug_assert!(
            new_length >= 0,
            "size must be positive (got {}): likely integer overflow?",
            new_length
        );
        let current_length = vec.len();
        match (new_length as usize).cmp(&current_length) {
            Ordering::Greater => {
                for _ in 0..(new_length as usize - current_length) {
                    vec.push(T::default());
                }
            }
            Ordering::Equal => {
                return Ok(());
            }
            Ordering::Less => {
                return Err(LuceneError::array_index_out_of_bounds(format!(
                    "new_length: {} is less than current_length: {}",
                    new_length, current_length
                )));
            }
        }
        Ok(())
    }
    pub fn grow_with_len<T>(vec: &mut Vec<T>, min_size: i32) -> Result<(), LuceneError>
    where
        T: Clone + Default,
    {
        debug_assert!(
            min_size >= 0,
            "size must be positive (got {}): likely integer overflow?",
            min_size
        );
        let current_length = vec.len();
        if min_size as usize > current_length {
            let additional = min_size as usize - current_length;
            vec.reserve(additional);
            let capacity = vec.capacity();
            // Fill the new slots with default values.
            // This ensures that even if reserve_exact doesn't add enough space,
            // we will push the necessary default values into the Vec.
            for _ in 0..(capacity - current_length) {
                vec.push(T::default());
            }
        }
        Ok(())
    }
    pub fn grow<T>(vec: &mut Vec<T>) -> Result<(), LuceneError>
    where
        T: Default,
    {
        let bytes_per_element = mem::size_of::<T>();
        debug_assert!(bytes_per_element <= i32::MAX as usize);
        Self::grow_exact(
            vec,
            Self::oversize(vec.len() as i32 + 1, bytes_per_element as i32),
        )
    }
    /// Returns an array whose size is at least {@code minLength}, generally over-allocating
    /// exponentially, but never allocating more than {@code maxLength} elements.
    pub fn grow_in_range<T>(
        vec: &mut Vec<T>,
        min_length: i32,
        max_length: i32,
    ) -> Result<(), LuceneError>
    where
        T: Default,
    {
        debug_assert!(
            min_length >= 0,
            "length must be positive (got {}): likely integer overflow?",
            min_length
        );

        if min_length > max_length {
            return Err(LuceneError::illegal_argument(format!(
                "requested minimum array length {} is larger than requested maximum array length {}",
                min_length, max_length
            )));
        }
        let current_length = vec.len();
        if current_length >= min_length as usize {
            return Ok(());
        }

        let potential_length = Self::oversize(min_length, BitUtil::INT_BYTES as i32);
        let final_length = std::cmp::min(max_length, potential_length);
        Self::grow_exact(vec, final_length)?;

        Ok(())
    }
    /// Returns a vector whose size is at least `min_size`, generally over-allocating
    /// exponentially, but never allocating more than `i32::MAX` elements.
    pub fn grow_i32(vec: &mut Vec<i32>, min_size: i32) -> Result<(), LuceneError> {
        Self::grow_in_range(vec, min_size, i32::MAX)
    }
    /// Returns a vector whose size is at least `min_size`, generally over-allocating
    /// exponentially, and it will not copy the original data to the new vector.
    pub fn grow_no_copy<T>(vec: &[T], min_size: i32) -> Result<Option<Vec<T>>, LuceneError>
    where
        T: Default + Clone,
    {
        debug_assert!(
            min_size >= 0,
            "size must be positive (got {}): likely integer overflow?",
            min_size
        );

        let current_size = vec.len();
        if current_size < min_size as usize {
            let new_size = Self::oversize(min_size, std::mem::size_of::<T>() as i32);
            let new_vec = vec![T::default(); new_size as usize];
            Ok(Option::from(new_vec))
        } else {
            Ok(None)
        }
    }
    /// Returns the hash of chars in the range from `start` (inclusive) to `end` (inclusive).
    #[cfg(feature = "not_required_in_rust_lucene")]
    pub fn hash_code(_array: &[char], _start: usize, _end: usize) -> i32 {
        unimplemented!()
    }

    /// Swaps the values stored in indices `i` and `j` of the given slice.
    #[cfg(feature = "not_required_in_rust_lucene")]
    pub fn swap<T>(_arr: &mut [T], _i: usize, _j: usize) {
        unimplemented!()
    }
    /// Sorts the given slice using the intro sort algorithm,
    /// falling back to insertion sort for small arrays.
    pub fn do_intro_sort<T, C>(
        a: &mut Vec<T>,
        from_index: i32,
        to_index: i32,
        comp: C,
    ) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
        C: Comparator<T>,
    {
        if to_index - from_index <= 1 {
            return Ok(());
        }
        ArrayIntroSorter::new(a, comp).sort(from_index, to_index)
    }
    /// Sorts the given slice using the intro sort algorithm,
    /// falling back to insertion sort for small arrays.
    pub fn intro_sort_with_comparator<T, C>(a: &mut Vec<T>, comp: C) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
        C: Comparator<T>,
    {
        Self::do_intro_sort(a, 0, a.len() as i32, comp)
    }
    /// Sorts the given slice in natural order using the intro sort algorithm,
    /// falling back to insertion sort for small arrays.
    pub fn intro_sort_with_range<T>(
        a: &mut Vec<T>,
        from_index: i32,
        to_index: i32,
    ) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
    {
        if to_index - from_index <= 1 {
            return Ok(());
        }
        Self::do_intro_sort(a, from_index, to_index, NaturalOrder::new())
    }
    /// Sorts the given slice in natural order using the intro sort algorithm,
    /// falling back to insertion sort for small arrays.
    pub fn intro_sort<T>(a: &mut Vec<T>) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
    {
        Self::intro_sort_with_range(a, 0, a.len() as i32)
    }
    /// Sorts the given slice using the Tim sort algorithm
    /// falling back to binary sort for small arrays.
    pub fn do_tim_sort<T, C>(
        a: &mut Vec<T>,
        from_index: i32,
        to_index: i32,
        comp: C,
    ) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
        C: Comparator<T>,
    {
        if to_index - from_index <= 1 {
            return Ok(());
        }
        let max_temp_slots = a.len() / 64;
        debug_assert!(max_temp_slots <= i32::MAX as usize);
        let array_tim_sorter = ArrayTimSorter::new(a, comp, max_temp_slots as i32);
        TimSorter::new(max_temp_slots as i32, array_tim_sorter).sort(from_index, to_index)
    }
    /// Sorts the given slice using the Tim sort algorithm,
    /// falling back to binary sort for small arrays.
    pub fn tim_sort_with_comparator<T, C>(a: &mut Vec<T>, comp: C) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
        C: Comparator<T>,
    {
        Self::do_tim_sort(a, 0, a.len() as i32, comp)
    }
    /// Sorts the given slice in natural order using the Tim sort algorithm,
    /// falling back to binary sort for small arrays.
    pub fn tim_sort_with_range<T>(
        a: &mut Vec<T>,
        from_index: i32,
        to_index: i32,
    ) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
    {
        if to_index - from_index <= 1 {
            return Ok(());
        }
        Self::do_tim_sort(a, from_index, to_index, NaturalOrder::new())
    }
    /// Sorts the given slice in natural order using the Tim sort algorithm,
    /// falling back to binary sort for small arrays.
    pub fn tim_sort<T>(a: &mut Vec<T>) -> Result<(), LuceneError>
    where
        T: Default + Clone + Ord,
    {
        Self::tim_sort_with_range(a, 0, a.len() as i32)
    }

    /// Reorganize the slice `arr[from..to]` so that the element at offset `k` is at the same position
    /// as if `arr[from..to]` were sorted, and all elements to its left are less than or equal to it,
    /// and all elements to its right are greater than or equal to it.
    ///
    /// This runs in linear time on average and in `n*log(n)` time in the worst case.
    ///
    /// # Parameters
    /// - `arr`: The array to be re-organized.
    /// - `from`: The starting index for re-organization. Elements before this index will be left as is.
    /// - `to`: The ending index. Elements after this index will be left as is.
    /// - `k`: The index of the element to sort from. Value must be less than `to` and greater than or
    ///     equal to `from`.
    /// - `comparator`: A comparator to use for sorting.
    pub fn select<T, C>(
        arr: &mut Vec<T>,
        from: i32,
        to: i32,
        k: i32,
        comparator: &mut C,
    ) -> Result<(), LuceneError>
    where
        T: Default + Ord,
        C: Comparator<T>,
    {
        let sub_selector = IntroSelectorImpl::new(arr, comparator);
        let mut selector = IntroSelector::new(sub_selector);
        Selector::select(&mut selector, from, to, k)?;
        Ok(())
    }
    /// Copies a slice into a new vector.
    pub fn copy_array<T>(array: &[T]) -> Vec<T>
    where
        T: Copy + Default,
    {
        Self::copy_of_sub_array(array, 0, array.len() as i32)
    }
    /// Clone a slice into a new vector.
    pub fn clone_array<T>(array: &[T]) -> Vec<T>
    where
        T: Clone + Default,
    {
        Self::clone_of_sub_array(array, 0, array.len() as i32)
    }

    /// Copies the specified range of the given array into a new sub-array.
    ///
    /// This method efficiently copies a slice of the input array into a new `Vec<T>`.
    ///
    /// - For types that implement the `Copy` trait (like `i32`, `i64`, etc.), it performs a
    ///   low-cost, efficient bitwise copy. This is fast and doesn't require heap allocation
    ///   or cloning, making it ideal for simple, stack-based types.
    ///
    /// - For types that do **not** implement `Copy` but implement `Clone`, you should use
    ///   the `clone_of_sub_array` method instead. The `clone_of_sub_array` method will
    ///   perform a deep copy by calling `clone()` on each element, which may involve
    ///   heap allocation or other more complex operations depending on the type.
    ///
    /// For types that implement neither `Copy` nor `Clone`, consider implementing `Clone`
    /// for your type or providing an alternative copying mechanism.
    ///
    /// # Arguments
    ///
    /// * `array` - A slice of the input array to copy from.
    /// * `from` - The initial index (inclusive) of the range to be copied.
    /// * `to` - The final index (exclusive) of the range to be copied.
    ///
    /// # Returns
    /// A new `Vec<T>` containing the specified sub-array of the input array.
    ///
    /// # See Also
    /// `clone_of_sub_array` for deep copy of types that implement `Clone`.
    pub fn copy_of_sub_array<T>(array: &[T], from: i32, to: i32) -> Vec<T>
    where
        T: Copy + Default,
    {
        debug_assert!(from >= 0 && to >= 0 && (to - from) >= 0 && to as usize <= array.len());
        let mut copy = vec![Default::default(); (to - from) as usize];
        copy.copy_from(&array[from as usize..to as usize], 0);
        copy
    }
    /// Clone the specified range of the given array into a new sub-array by cloning each element.
    ///
    /// This method is suitable for types that implement the `Clone` trait, allowing for
    /// a deep copy of the specified range from the input array into a new `Vec<T>`. It
    /// is typically used when the type does not implement the `Copy` trait (e.g., types
    /// that involve heap allocation like `String`, `Vec<T>`, etc.), as `Clone` allows
    /// for the creation of independent copies of each element.
    ///
    /// - For types that implement `Copy` (like `i32`, `i64`, etc.), consider using the
    ///   `copy_of_sub_array` method instead, as it performs a more efficient bitwise copy
    ///   without requiring cloning or heap allocations.
    ///
    /// - For types that implement `Clone`, this method will perform a deep copy by
    ///   calling `clone()` on each element, which may involve heap allocation or more
    ///   complex operations depending on the type.
    ///
    /// # Arguments
    ///
    /// * `array` - A slice of the input array to copy from.
    /// * `from` - The initial index (inclusive) of the range to be copied.
    /// * `to` - The final index (exclusive) of the range to be copied.
    ///
    /// # Returns
    /// A new `Vec<T>` containing the specified sub-array of the input array, with each
    /// element cloned individually.
    pub fn clone_of_sub_array<T>(array: &[T], from: i32, to: i32) -> Vec<T>
    where
        T: Clone + Default,
    {
        debug_assert!(from >= 0 && to >= 0 && (to - from) >= 0 && to as usize <= array.len());
        array[from as usize..to as usize].to_vec()
    }
}

struct IntroSelectorImpl<'a, T, C>
where
    T: Default + Ord,
    C: Comparator<T>,
{
    pivot: i32,
    arr: &'a mut Vec<T>,
    comparator: &'a C,
}
impl<'a, T, C> IntroSelectorImpl<'a, T, C>
where
    T: Default + Ord,
    C: Comparator<T>,
{
    fn new(arr: &'a mut Vec<T>, comparator: &'a C) -> IntroSelectorImpl<'a, T, C> {
        IntroSelectorImpl {
            pivot: 0,
            arr,
            comparator,
        }
    }
}

impl<T, C> IntroSelectorBaseDefault for IntroSelectorImpl<'_, T, C>
where
    C: Comparator<T>,
    T: Default + Ord + PartialEq,
{
    fn set_pivot(&mut self, i: i32) {
        self.pivot = i;
    }

    fn compare_pivot(&self, j: i32) -> i32 {
        self.comparator
            .compare(&self.arr[self.pivot as usize], &self.arr[j as usize])
    }
}

impl<T, C> IntroSelectorBase for IntroSelectorImpl<'_, T, C>
where
    T: Default + Ord,
    C: Comparator<T>,
{
}
impl<T, C> Selector for IntroSelectorImpl<'_, T, C>
where
    T: Default + Ord,
    C: Comparator<T>,
{
    fn swap(&mut self, i: i32, j: i32) -> Result<(), LuceneError> {
        // The data pointed to by the pivot has been swapped.
        // We need to adjust the pivot value to ensure that
        // the value corresponding to the pivot remains unchanged.
        // To avoid Copying the value, we just swap the pivot index.
        if self.pivot == i || self.pivot == j {
            self.pivot = if self.pivot == i { j } else { i };
        }
        self.arr.swap(i as usize, j as usize);
        Ok(())
    }
}
/// Comparator for a fixed number of bytes.
pub trait ByteArrayComparator {
    /// Compare bytes starting from the given offsets.
    ///
    /// The return value has the same contract as [`std::cmp::Ord::cmp`].
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32;
}
/// Returns a comparator for exactly the specified number of bytes.
pub fn get_unsigned_comparator(num_bytes: usize) -> ByteArrayComparatorEnum {
    if num_bytes == BitUtil::LONG_BYTES {
        // Used by LongPoint, DoublePoint
        return ByteArrayComparatorEnum::U64(U64byteArrayComparator);
    } else if num_bytes == BitUtil::INT_BYTES {
        // Used by IntPoint, FloatPoint, LatLonPoint, LatLonShape
        return ByteArrayComparatorEnum::U32(U32byteArrayComparator);
    }
    ByteArrayComparatorEnum::Byte(ByteByteArrayComparator { num_bytes })
}
pub enum ByteArrayComparatorEnum {
    U64(U64byteArrayComparator),
    U32(U32byteArrayComparator),
    Byte(ByteByteArrayComparator),
}
impl ByteArrayComparator for ByteArrayComparatorEnum {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        match self {
            ByteArrayComparatorEnum::U64(c) => c.compare(a, a_i, b, b_i),
            ByteArrayComparatorEnum::U32(c) => c.compare(a, a_i, b, b_i),
            ByteArrayComparatorEnum::Byte(c) => c.compare(a, a_i, b, b_i),
        }
    }
}

pub struct U64byteArrayComparator;
impl ByteArrayComparator for U64byteArrayComparator {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        match (BitUtil::get_i64_be(a, a_i) as u64).cmp(&(BitUtil::get_i64_be(b, b_i) as u64)) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}
pub struct U32byteArrayComparator;
impl ByteArrayComparator for U32byteArrayComparator {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        match (BitUtil::get_i32_be(a, a_i) as u32).cmp(&(BitUtil::get_i32_be(b, b_i) as u32)) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}
pub struct ByteByteArrayComparator {
    num_bytes: usize,
}
impl ByteArrayComparator for ByteByteArrayComparator {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        debug_assert!(a.len() >= a_i + self.num_bytes);
        debug_assert!(b.len() >= b_i + self.num_bytes);
        match &a[a_i..a_i + self.num_bytes].cmp(&b[b_i..b_i + self.num_bytes]) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::util::lucene_test_case::{at_least, random};

    use crate::test::util::test_util::TestUtil;
    use crate::util::array_util::{
        ArrayUtil, ByteArrayComparator, U32byteArrayComparator, U64byteArrayComparator,
    };
    use crate::util::bit_util::BitUtil;
    use crate::util::error::lucene_error::LuceneError;
    use crate::util::{NaturalOrder, ReverseOrder};
    use rand::rngs::StdRng;
    use rand::Rng;
    use std::cmp::Ordering;
    use std::fmt::Display;

    #[allow(dead_code)] // for quick search
    pub struct TestArrayUtil;
    #[test]
    fn test_growth() {
        let mut current_size: i32 = 0;
        let mut copy_cost: i32 = 0;

        while current_size != ArrayUtil::MAX_ARRAY_LENGTH {
            let next_size = ArrayUtil::oversize(1 + current_size, 0);
            assert!(next_size > current_size);

            if current_size > 0 {
                copy_cost += current_size;
                let copy_cost_per_element = copy_cost as f64 / current_size as f64;
                assert!(
                    copy_cost_per_element < 10.0,
                    "cost {}",
                    copy_cost_per_element
                );
            }

            current_size = next_size;
        }
    }
    #[test]
    fn test_max_size() {
        for elem_size in 0..10 {
            assert_eq!(
                ArrayUtil::MAX_ARRAY_LENGTH,
                ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH, elem_size)
            );
            assert_eq!(
                ArrayUtil::MAX_ARRAY_LENGTH,
                ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH - 1, elem_size)
            );
        }
    }

    #[test]
    fn test_too_big() {
        //TODO: The current implementation of oversize is simple and cannot be tested for this functionality.
    }

    #[test]
    fn test_exact_limit() {
        assert_eq!(
            ArrayUtil::MAX_ARRAY_LENGTH,
            ArrayUtil::oversize(ArrayUtil::MAX_ARRAY_LENGTH, 1)
        );
    }
    #[test]
    fn test_invalid_element_sizes() {
        let mut random = random();
        let num = at_least(&mut random, 10000);
        for _ in 0..num {
            let min_target_size = random.gen_range(0..ArrayUtil::MAX_ARRAY_LENGTH);
            let elem_size = random.gen_range(0..11);
            let v = ArrayUtil::oversize(min_target_size, elem_size);
            assert!(v >= min_target_size);
        }
    }
    fn parse_int(random: &mut StdRng, s: &str) -> Result<i32, LuceneError> {
        let start = random.gen_range(0..5);
        let extra_length = random.gen_range(0..4);
        let mut chars: Vec<char> = vec![' '; s.len() + start + extra_length];
        let s_chars: Vec<char> = s.chars().collect();
        chars[start..start + s.len()].copy_from_slice(&s_chars);
        ArrayUtil::parse_int_default(&chars, start as i32, s.len() as i32)
    }
    #[test]
    fn test_parse_int() {
        let mut random = random();
        let result = parse_int(&mut random, "");
        assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

        let result = parse_int(&mut random, "foo");
        assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

        let result = parse_int(&mut random, &i64::MAX.to_string());
        assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

        let result = parse_int(&mut random, "0.34");
        assert!(matches!(result, Err(LuceneError::NumberFormat(_))));

        let result = parse_int(&mut random, "1");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, 1, "{} does not equal: 1", value);

        let result = parse_int(&mut random, "-10000");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, -10000, "{} does not equal: -10000", value);

        let result = parse_int(&mut random, "1923");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, 1923, "{} does not equal: 1923", value);

        let result = parse_int(&mut random, "-1");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, -1, "{} does not equal: -1", value);

        let result =
            ArrayUtil::parse_int_default(&"foo 1923 bar".chars().collect::<Vec<char>>(), 4, 4);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, 1923, "{} does not equal: 1923", value);
    }
    fn create_random_array(random: &mut StdRng, max_size: i32) -> Vec<i32> {
        let size = random.gen_range(1..=max_size);
        let mut array = Vec::with_capacity(size as usize);

        for _ in 0..size {
            array.push(random.gen_range(0..size));
        }
        array
    }
    #[test]
    fn test_intro_sort() -> Result<(), LuceneError> {
        let mut random = random();
        let num = at_least(&mut random, 50);
        for _ in 0..num {
            let mut a1 = create_random_array(&mut random, 2000);
            let mut a2 = a1.clone();

            ArrayUtil::intro_sort(&mut a1)?;
            a2.sort();
            assert_eq!(a1, a2);

            a1 = create_random_array(&mut random, 2000);
            a2 = a1.clone();
            ArrayUtil::intro_sort_with_comparator(&mut a1, ReverseOrder::new())?;
            a2.sort_by(|x, y| y.cmp(x)); // reverse order
            assert_eq!(a1, a2);

            ArrayUtil::intro_sort(&mut a1)?;
            a2.sort();
            assert_eq!(a1, a2);
        }
        Ok(())
    }
    fn create_sparse_random_array(random: &mut StdRng, max_size: i32) -> Vec<i32> {
        let size = random.gen_range(0..=max_size);
        let mut array = Vec::with_capacity(size as usize);

        for _ in 0..size {
            array.push(random.gen_range(0..2));
        }
        array
    }
    // This is a test for LUCENE-3054 (which fails without the merge sort fall back with stack
    // overflow in most cases)
    #[test]
    fn test_quick_to_heap_sort_fallback() -> Result<(), LuceneError> {
        let mut random = random();
        let num = at_least(&mut random, 10);
        for _ in 0..num {
            let mut a1 = create_sparse_random_array(&mut random, 40_000);
            let mut a2 = a1.clone();
            ArrayUtil::intro_sort(&mut a1)?;
            a2.sort();
            assert_eq!(a1, a2);
        }
        Ok(())
    }
    #[test]
    fn test_tim_sort() -> Result<(), LuceneError> {
        let mut random = random();
        let num = at_least(&mut random, 50);

        for _ in 0..num {
            let mut a1 = create_random_array(&mut random, 2000);
            let mut a2 = a1.clone();

            ArrayUtil::tim_sort(&mut a1)?;
            a2.sort();
            assert_eq!(a1, a2);

            a1 = create_random_array(&mut random, 2000);
            a2 = a1.clone();
            ArrayUtil::tim_sort_with_comparator(&mut a1, ReverseOrder::new())?;
            a2.sort_by(|a, b| b.cmp(a));
            assert_eq!(a1, a2);
            // reverse back, so we can test that completely backwards sorted array (worst case) is
            // working:
            ArrayUtil::tim_sort(&mut a1)?;
            a2.sort();
            assert_eq!(a1, a2);
        }
        Ok(())
    }
    #[derive(Debug, Clone, Default)]
    struct Item {
        val: i32,
        order: i32,
    }

    impl Item {
        fn new(val: i32, order: i32) -> Self {
            Item { val, order }
        }
    }
    impl Display for Item {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Item {{ val: {}, order: {} }}", self.val, self.order)
        }
    }

    impl Eq for Item {}

    impl PartialEq<Self> for Item {
        fn eq(&self, _other: &Self) -> bool {
            todo!()
        }
    }

    impl PartialOrd<Self> for Item {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Item {
        fn cmp(&self, other: &Self) -> Ordering {
            self.order.cmp(&other.order)
        }
    }

    #[test]
    fn test_merge_sort_stability() -> Result<(), LuceneError> {
        let mut random = random();
        let mut items = Vec::with_capacity(100);

        for i in 0..100 {
            // half of the items have value but same order. The value of this items is sorted,
            // so they should always be in order after sorting.
            // The other half has defined order, but no (-1) value (they should appear after
            // all above, when sorted).
            let equal = random.gen_bool(0.5);
            if equal {
                items.push(Item::new(i + 1, 0));
            } else {
                items.push(Item::new(-1, random.gen_range(1..=1000)));
            }
        }
        if cfg!(feature = "test_log_verbose") {
            println!("Before: {:?}", items);
        }
        // if you replace this with ArrayUtil.quickSort(), test should fail:
        ArrayUtil::tim_sort(&mut items)?;

        if cfg!(feature = "test_log_verbose") {
            println!("Sorted: {:?}", items);
        }

        let mut last = &items[0];

        for item in &items[1..] {
            if item.order == 0 {
                assert!(item.val > last.val);
            }
            assert!(item.order >= last.order);

            last = item;
        }
        Ok(())
    }
    #[test]
    fn test_tim_sort_stability() -> Result<(), LuceneError> {
        let mut random = rand::thread_rng();
        let mut items = Vec::with_capacity(100);

        for i in 0..100 {
            // half of the items have value but same order. The value of this items is sorted,
            // so they should always be in order after sorting.
            // The other half has defined order, but no (-1) value (they should appear after
            // all above, when sorted).
            let equal = random.gen_bool(0.5);
            if equal {
                items.push(Item::new(i + 1, 0)); // "equal" items
            } else {
                items.push(Item::new(-1, random.gen_range(1..=1000))); // Items with defined order
            }
        }

        if cfg!(feature = "test_log_verbose") {
            println!("Before: {:?}", items);
        }
        ArrayUtil::tim_sort(&mut items)?;

        if cfg!(feature = "test_log_verbose") {
            println!("Sorted: {:?}", items);
        }

        let mut last = &items[0];

        for item in &items[1..] {
            if item.order == 0 {
                // order of "equal" items should be not mixed up
                assert!(item.val > last.val, "Expected sorted value for equal items");
            }
            assert!(item.order >= last.order, "Expected sorted order");
            last = item;
        }
        Ok(())
    }
    // should produce no exceptions
    #[test]
    fn test_empty_array_sort() -> Result<(), LuceneError> {
        let mut a: Vec<i32> = Vec::new();
        ArrayUtil::intro_sort(&mut a)?;
        ArrayUtil::tim_sort(&mut a)?;
        ArrayUtil::intro_sort_with_comparator(&mut a, ReverseOrder::new())?;
        ArrayUtil::tim_sort_with_comparator(&mut a, ReverseOrder::new())?;
        Ok(())
    }

    #[test]
    fn test_select() -> Result<(), LuceneError> {
        let mut random = random();
        for _ in 0..100 {
            do_test_select(&mut random)?
        }
        Ok(())
    }

    fn do_test_select(random: &mut StdRng) -> Result<(), LuceneError> {
        let from = random.gen_range(0..5) as usize;
        let to = from + TestUtil::next_int(random, 1, 10_000) as usize;
        let max = if random.gen_bool(0.5) {
            random.gen_range(0..100)
        } else {
            random.gen_range(0..100_000)
        };

        let arr: Vec<i32> = (0..from + to + random.gen_range(0..5))
            .map(|_| TestUtil::next_int(random, 0, max))
            .collect();

        let k = TestUtil::next_int(random, from as i32, (to - 1) as i32) as usize;

        let mut expected = arr.clone();
        expected[from..to].sort();

        let mut actual = arr.clone();
        ArrayUtil::select(
            &mut actual,
            from as i32,
            to as i32,
            k as i32,
            &mut NaturalOrder::new(),
        )?;

        assert_eq!(expected[k], actual[k]);

        for (i, &value) in actual.iter().enumerate() {
            if i < from || i >= to {
                assert_eq!(arr[i], value);
            } else if i <= k {
                assert!(value <= actual[k]);
            } else {
                assert!(value >= actual[k]);
            }
        }
        Ok(())
    }

    #[test]
    fn test_grow_exact() -> Result<(), LuceneError> {
        let mut random = random();
        let mut arr: Vec<i16> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert_eq!(arr, vec![1, 2, 3, 0]);
        let mut arr: Vec<i16> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert_eq!(arr, vec![1, 2, 3, 0, 0]);
        let mut arr: Vec<i16> = vec![1, 2, 3];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<i32> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert_eq!(arr, vec![1, 2, 3, 0]);
        let mut arr: Vec<i32> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert_eq!(arr, vec![1, 2, 3, 0, 0]);
        let mut arr: Vec<i32> = vec![1, 2, 3];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<i64> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert_eq!(arr, vec![1, 2, 3, 0]);
        let mut arr: Vec<i64> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert_eq!(arr, vec![1, 2, 3, 0, 0]);
        let mut arr: Vec<i64> = vec![1, 2, 3];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<f32> = vec![0.1, 0.2, 0.3];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert!((arr[3] - 0.0).abs() < 0.001);
        let mut arr: Vec<f32> = vec![0.1, 0.2, 0.3];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert!((arr[3] - 0.0).abs() < 0.001);
        assert!((arr[4] - 0.0).abs() < 0.001);
        let mut arr: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<f64> = vec![0.1, 0.2, 0.3];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert!((arr[3] - 0.0).abs() < 0.001);
        let mut arr: Vec<f64> = vec![0.1, 0.2, 0.3];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert!((arr[3] - 0.0).abs() < 0.001);
        assert!((arr[4] - 0.0).abs() < 0.001);
        let mut arr: Vec<f64> = vec![0.1, 0.2, 0.3];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<i8> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert_eq!(arr, vec![1, 2, 3, 0]);
        let mut arr: Vec<i8> = vec![1, 2, 3];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert_eq!(arr, vec![1, 2, 3, 0, 0]);
        let mut arr: Vec<i8> = vec![1, 2, 3];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<char> = vec!['a', 'b', 'c'];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert_eq!(arr, vec!['a', 'b', 'c', '\0']);
        let mut arr: Vec<char> = vec!['a', 'b', 'c'];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert_eq!(arr, vec!['a', 'b', 'c', '\0', '\0']);
        let mut arr: Vec<char> = vec!['a', 'b', 'c'];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        let mut arr: Vec<Option<String>> = vec![
            Some("a1".to_string()),
            Some("b2".to_string()),
            Some("c3".to_string()),
        ];
        ArrayUtil::grow_exact(&mut arr, 4)?;
        assert_eq!(
            arr,
            vec![
                Some("a1".to_string()),
                Some("b2".to_string()),
                Some("c3".to_string()),
                None
            ]
        );
        let mut arr: Vec<Option<String>> = vec![
            Some("a1".to_string()),
            Some("b2".to_string()),
            Some("c3".to_string()),
        ];
        ArrayUtil::grow_exact(&mut arr, 5)?;
        assert_eq!(
            arr,
            vec![
                Some("a1".to_string()),
                Some("b2".to_string()),
                Some("c3".to_string()),
                None,
                None
            ]
        );
        let mut arr: Vec<Option<String>> = vec![
            Some("a".to_string()),
            Some("b".to_string()),
            Some("c".to_string()),
        ];
        let result = ArrayUtil::grow_exact(&mut arr, random.gen_range(0..3));
        assert!(matches!(result, Err(LuceneError::ArrayIndexOutOfBounds(_))));

        Ok(())
    }

    #[test]
    fn test_grow_in_range() -> Result<(), LuceneError> {
        let mut array: Vec<i32> = vec![1, 2, 3];
        // If minLength is negative, maxLength does not matter
        // TODO

        // If minLength > maxLength, we throw an exception
        let result = ArrayUtil::grow_in_range(&mut array, 1, 0);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        let result = ArrayUtil::grow_in_range(&mut array, 4, 3);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        let result = ArrayUtil::grow_in_range(&mut array, 5, 4);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));

        // If minLength is sufficient, we return the array
        ArrayUtil::grow_in_range(&mut array, 1, 4)?;
        assert_eq!(array, vec![1, 2, 3]);
        ArrayUtil::grow_in_range(&mut array, 1, 2)?;
        assert_eq!(array, vec![1, 2, 3]);
        ArrayUtil::grow_in_range(&mut array, 1, 1)?;
        assert_eq!(array, vec![1, 2, 3]);

        let min_length = 4;
        let max_length = i32::MAX;

        let mut vec = vec![1, 2, 3];
        ArrayUtil::grow_in_range(&mut vec, min_length, max_length)?;
        assert_eq!(
            ArrayUtil::oversize(min_length, std::mem::size_of::<i32>() as i32),
            vec.len() as i32
        );

        // The array grows to maxLength if maxLength is limiting
        let mut vec = vec![1, 2, 3];
        ArrayUtil::grow_in_range(&mut vec, min_length, min_length)?;
        assert_eq!(min_length, vec.len() as i32);
        Ok(())
    }
    #[test]
    fn test_copy_of_sub_array() {
        let short_array: Vec<i16> = vec![1, 2, 3];
        assert_eq!(vec![1], ArrayUtil::copy_of_sub_array(&short_array, 0, 1));
        assert_eq!(
            vec![1, 2, 3],
            ArrayUtil::copy_of_sub_array(&short_array, 0, 3)
        );
        assert_eq!(
            Vec::<i16>::new(),
            ArrayUtil::copy_of_sub_array(&short_array, 0, 0)
        );

        let int_array: Vec<i32> = vec![1, 2, 3];
        assert_eq!(vec![1, 2], ArrayUtil::copy_of_sub_array(&int_array, 0, 2));
        assert_eq!(
            vec![1, 2, 3],
            ArrayUtil::copy_of_sub_array(&int_array, 0, 3)
        );
        assert_eq!(
            Vec::<i32>::new(),
            ArrayUtil::copy_of_sub_array(&int_array, 1, 1)
        );

        let long_array: Vec<i64> = vec![1, 2, 3];
        assert_eq!(vec![2], ArrayUtil::copy_of_sub_array(&long_array, 1, 2));
        assert_eq!(
            vec![1, 2, 3],
            ArrayUtil::copy_of_sub_array(&long_array, 0, 3)
        );
        assert_eq!(
            Vec::<i64>::new(),
            ArrayUtil::copy_of_sub_array(&long_array, 2, 2)
        );

        let float_array: Vec<f32> = vec![0.1, 0.2, 0.3];
        assert_eq!(
            vec![0.2, 0.3],
            ArrayUtil::copy_of_sub_array(&float_array, 1, 3)
        );
        assert_eq!(
            vec![0.1, 0.2, 0.3],
            ArrayUtil::copy_of_sub_array(&float_array, 0, 3)
        );
        assert_eq!(
            Vec::<f32>::new(),
            ArrayUtil::copy_of_sub_array(&float_array, 0, 0)
        );

        let double_array: Vec<f64> = vec![0.1, 0.2, 0.3];
        assert_eq!(vec![0.3], ArrayUtil::copy_of_sub_array(&double_array, 2, 3));
        assert_eq!(
            vec![0.1, 0.2, 0.3],
            ArrayUtil::copy_of_sub_array(&double_array, 0, 3)
        );
        assert_eq!(
            Vec::<f64>::new(),
            ArrayUtil::copy_of_sub_array(&double_array, 1, 1)
        );

        let byte_array: Vec<u8> = vec![1, 2, 3];
        assert_eq!(vec![1], ArrayUtil::copy_of_sub_array(&byte_array, 0, 1));
        assert_eq!(
            vec![1, 2, 3],
            ArrayUtil::copy_of_sub_array(&byte_array, 0, 3)
        );
        assert_eq!(
            Vec::<u8>::new(),
            ArrayUtil::copy_of_sub_array(&byte_array, 1, 1)
        );

        let char_array: Vec<char> = vec!['a', 'b', 'c'];
        assert_eq!(
            vec!['a', 'b'],
            ArrayUtil::copy_of_sub_array(&char_array, 0, 2)
        );
        assert_eq!(
            vec!['a', 'b', 'c'],
            ArrayUtil::copy_of_sub_array(&char_array, 0, 3)
        );
        assert_eq!(
            Vec::<char>::new(),
            ArrayUtil::copy_of_sub_array(&char_array, 1, 1)
        );

        let object_array: Vec<String> = vec!["a1".to_string(), "b2".to_string(), "c3".to_string()];
        assert_eq!(
            vec!["a1".to_string()],
            ArrayUtil::clone_of_sub_array(&object_array, 0, 1)
        );
        assert_eq!(
            vec!["a1".to_string(), "b2".to_string(), "c3".to_string()],
            ArrayUtil::clone_of_sub_array(&object_array, 0, 3)
        );
        assert_eq!(
            Vec::<String>::new(),
            ArrayUtil::clone_of_sub_array(&object_array, 1, 1)
        );
    }
    #[test]
    fn test_compare_unsigned4() {
        let mut random = random();
        let a_offset = TestUtil::next_int(&mut random, 0, 3) as usize;
        let mut a = vec![0u8; BitUtil::INT_BYTES + a_offset];
        let b_offset = TestUtil::next_int(&mut random, 0, 3) as usize;
        let mut b = vec![0u8; BitUtil::INT_BYTES + b_offset];
        for i in 0..BitUtil::INT_BYTES {
            a[a_offset + i] = random.gen::<u8>();
            loop {
                b[b_offset + i] = random.gen::<u8>();
                if b[b_offset + i] != a[a_offset + i] {
                    break;
                }
            }
        }

        for i in 0..BitUtil::INT_BYTES {
            let result = a[a_offset..a_offset + BitUtil::INT_BYTES]
                .cmp(&b[b_offset..b_offset + BitUtil::INT_BYTES]);
            let expected: i32 = match result {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };

            let cmp = U32byteArrayComparator;
            let actual = cmp.compare(&a, a_offset, &b, b_offset);
            assert_eq!(expected.signum(), actual.signum());

            b[b_offset + i] = a[a_offset + i];
        }

        let cmp = U32byteArrayComparator;
        assert_eq!(cmp.compare(&a, a_offset, &b, b_offset), 0);
    }

    #[test]
    fn test_compare_unsigned8() {
        let mut random = random();
        let a_offset = TestUtil::next_int(&mut random, 0, 7) as usize;
        let mut a = vec![0u8; BitUtil::LONG_BYTES + a_offset];
        let b_offset = TestUtil::next_int(&mut random, 0, 7) as usize;
        let mut b = vec![0u8; BitUtil::LONG_BYTES + b_offset];
        for i in 0..BitUtil::LONG_BYTES {
            a[a_offset + i] = random.gen::<u8>();
            loop {
                b[b_offset + i] = random.gen::<u8>();
                if b[b_offset + i] != a[a_offset + i] {
                    break;
                }
            }
        }
        for i in 0..BitUtil::LONG_BYTES {
            let result = a[a_offset..a_offset + BitUtil::LONG_BYTES]
                .cmp(&b[b_offset..b_offset + BitUtil::LONG_BYTES]);
            let expected: i32 = match result {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            let cmp = U64byteArrayComparator;
            let actual = cmp.compare(&a, a_offset, &b, b_offset);
            assert_eq!(expected.signum(), actual.signum());
            b[b_offset + i] = a[a_offset + i];
        }
        let cmp = U64byteArrayComparator;
        assert_eq!(cmp.compare(&a, a_offset, &b, b_offset), 0);
    }
}
