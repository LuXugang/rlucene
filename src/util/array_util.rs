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
use crate::util::bit_util::BitUtil;
use crate::util::error::lucene_error::LuceneError;
use crate::util::selector::Selector;
use crate::util::{
    ArrayIntroSorter, ArrayTimSorter, Comparator, IntroSelector, IntroSelectorBase,
    IntroSelectorBaseDefault, NaturalOrder, Sorter, TimSorter, VecCopyOps,
};
use std::cmp::Ordering;
use std::mem;

pub struct ArrayUtil;
impl ArrayUtil {
    pub const MAX_ARRAY_LENGTH: i32 = 0;
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
    fn parse_int_default(chars: &[char], offset: i32, len: i32) -> Result<i32, LuceneError> {
        Self::parse_int(chars, offset, len, 10)
    }

    /// Parses the string argument as if it were an `i32` value and returns the result.
    /// Throws a `LuceneError::NumberFormat` if the string does not represent an `i32` quantity.
    /// The second argument specifies the radix to use when parsing the value.
    fn parse_int(
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

    fn parse(
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
        if new_length as usize > current_length {
            let additional = new_length as usize - current_length;
            vec.reserve_exact(additional);
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
    pub fn grow_no_copy<T>(vec: &mut [T], min_size: i32) -> Result<Option<Vec<T>>, LuceneError>
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
        T: Copy,
    {
        Self::copy_of_sub_array(array, 0, array.len() as i32)
    }

    /// Copies the specified range of the given array into a new sub-array.
    ///
    /// # Arguments
    ///
    /// * `array` - The input byte slice.
    /// * `from` - The initial index of range to be copied (inclusive).
    /// * `to` - The final index of range to be copied (exclusive).
    ///
    /// # Returns
    /// A new `Vec<T>` containing the specified sub-array.
    pub fn copy_of_sub_array<T>(array: &[T], from: i32, to: i32) -> Vec<T>
    where
        T: Copy,
    {
        debug_assert!(from >= 0 && to >= 0 && (to - from) >= 0);
        let mut copy = Vec::with_capacity((to - from) as usize);
        copy.copy_from(&array[from as usize..to as usize], 0);
        copy
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
    /// The return value has the same contract as [`std::cmp::Ord::cmp`](std::cmp::Ord::cmp).
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
        match BitUtil::get_i64_be(a, a_i).cmp(&BitUtil::get_i64_be(b, b_i)) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}
pub struct U32byteArrayComparator;
impl ByteArrayComparator for U32byteArrayComparator {
    fn compare(&self, a: &[u8], a_i: usize, b: &[u8], b_i: usize) -> i32 {
        match BitUtil::get_i32_be(a, a_i).cmp(&BitUtil::get_i32_be(b, b_i)) {
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
