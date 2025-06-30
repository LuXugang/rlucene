/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::index::index_options::IndexOptions;
/// This class tracks the number and position / offset parameters of terms being
/// added to the index. The information collected in this class is also used to
/// calculate the normalization factor for a field.
pub struct FieldInvertState<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    index_created_version_major: i32,
    name: String,
    index_options: IndexOptions,
    pub(crate) position: i32,
    pub(crate) length: i32,
    pub(crate) num_overlap: i32,
    pub(crate) offset: i32,
    pub(crate) max_term_frequency: i32,
    pub(crate) unique_term_count: i32,
    // we must track these across field instances (multi-valued case)
    pub(crate) last_start_offset: i32,
    pub(crate) last_position: i32,
    pub(crate) offset_attribute: Option<O>,
    pub(crate) payload_attribute: Option<P>,
    pub(crate) term_freq_attribute: Option<T>,
}
impl<O, P, T> Default for FieldInvertState<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    fn default() -> Self {
        FieldInvertState {
            index_created_version_major: 6,
            name: String::new(),
            index_options: IndexOptions::None,
            position: -1,
            length: 0,
            num_overlap: 0,
            offset: 0,
            max_term_frequency: 0,
            unique_term_count: 0,
            last_start_offset: 0,
            last_position: 0,
            offset_attribute: None,
            payload_attribute: None,
            term_freq_attribute: None,
        }
    }
}

impl<O, P, T> FieldInvertState<O, P, T>
where
    O: OffsetAttribute,
    P: PayloadAttribute,
    T: TermFrequencyAttribute,
{
    /// Creates {code FieldInvertState} for the specified field name.
    pub fn new(
        index_created_version_major: i32,
        name: String,
        index_options: IndexOptions,
    ) -> Self {
        FieldInvertState {
            index_created_version_major,
            name,
            index_options,
            position: 0,
            length: 0,
            num_overlap: 0,
            max_term_frequency: 0,
            unique_term_count: 0,
            offset: 0,
            last_start_offset: 0,
            last_position: 0,
            offset_attribute: None,
            payload_attribute: None,
            term_freq_attribute: None,
        }
    }
    /// Creates {code FieldInvertState} for the specified field name and values
    /// for all fields.
    #[allow(clippy::too_many_arguments)]
    pub fn with_states(
        index_created_version_major: i32,
        name: String,
        index_options: IndexOptions,
        position: i32,
        length: i32,
        num_overlap: i32,
        offset: i32,
        max_term_frequency: i32,
        unique_term_count: i32,
    ) -> Self {
        let mut state = Self::new(index_created_version_major, name, index_options);
        state.position = position;
        state.length = length;
        state.num_overlap = num_overlap;
        state.offset = offset;
        state.max_term_frequency = max_term_frequency;
        state.unique_term_count = unique_term_count;
        state
    }
    /// Re-initialize the state
    pub fn reset(&mut self) {
        self.position = -1;
        self.length = 0;
        self.num_overlap = 0;
        self.offset = 0;
        self.max_term_frequency = 0;
        self.unique_term_count = 0;
        self.last_start_offset = 0;
        self.last_position = 0;
    }
    // TODO: setAttributeSource

    /// Get the last processed term position.
    pub fn position(&self) -> i32 {
        self.position
    }
    /// Get total number of terms in this field.
    pub fn length(&self) -> i32 {
        self.length
    }

    /// Set length value.
    pub fn set_length(&mut self, length: i32) {
        self.length = length;
    }

    /// Get the number of terms with `position_increment == 0`.
    pub fn num_overlap(&self) -> i32 {
        self.num_overlap
    }

    /// Set number of terms with `position_increment == 0`.
    pub fn set_num_overlap(&mut self, num_overlap: i32) {
        self.num_overlap = num_overlap;
    }

    /// Get end offset of the last processed term.
    pub fn get_offset(&self) -> i32 {
        self.offset
    }

    /// Get the maximum term-frequency encountered for any term in the field. A
    /// field containing "the quick brown fox jumps over the lazy dog" would
    /// have a value of 2, because "the" appears twice.
    pub fn get_max_term_frequency(&self) -> i32 {
        self.max_term_frequency
    }

    /// Return the number of unique terms encountered in this field.
    pub fn get_unique_term_count(&self) -> i32 {
        self.unique_term_count
    }

    /// Return the field's name.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Return the version that was used to create the index, or 6 if it was
    /// created before 7.0.
    pub fn get_index_created_version_major(&self) -> i32 {
        self.index_created_version_major
    }

    /// Get the index options for this field.
    pub fn get_index_options(&self) -> IndexOptions {
        self.index_options
    }
}
