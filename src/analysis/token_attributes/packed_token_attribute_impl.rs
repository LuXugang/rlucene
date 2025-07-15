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
use crate::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImpl;
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::analysis::token_attributes::position_length_attribute::PositionLengthAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::analysis::token_attributes::type_attribute::{ta_util, TypeAttribute};
use crate::util::attribute::Attribute;
use crate::util::attribute_impl::AttributeImpl;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::hash::{Hash, Hasher};
/// Default implementation of the common attributes used by Lucene:
///
/// - [`CharTermAttribute`]
/// - [`TypeAttribute`]
/// - [`PositionIncrementAttribute`]
/// - [`PositionLengthAttribute`]
/// - [`OffsetAttribute`]
/// - [`TermFrequencyAttribute`]
pub struct PackedTokenAttributeImpl {
    start_offset: i32,
    end_offset: i32,
    type_: String,
    position_increment: i32,
    position_length: i32,
    term_frequency: i32,
    base: CharTermAttributeImpl,
}
impl Default for PackedTokenAttributeImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl PackedTokenAttributeImpl {
    pub fn new() -> Self {
        Self {
            start_offset: 0,
            end_offset: 0,
            type_: ta_util::DEFAULT_TYPE.to_string(),
            position_increment: 1,
            position_length: 1,
            term_frequency: 1,
            base: CharTermAttributeImpl::new(),
        }
    }
}

impl Attribute for PackedTokenAttributeImpl {}

impl TypeAttribute for PackedTokenAttributeImpl {
    /// Returns this Token's lexical type. Defaults to "word".
    fn type_value(&self) -> &str {
        self.type_.as_str()
    }
    /// Set the lexical type.
    fn set_type(&mut self, type_: &str) {
        self.type_ = type_.to_string();
    }
}
impl PositionIncrementAttribute for PackedTokenAttributeImpl {
    /// Set the position increment. The default value is one.
    fn set_position_increment(&mut self, position_increment: i32) -> Result<()> {
        if position_increment < 0 {
            return Err(LuceneError::illegal_state(format!(
                "Increment must be zero or greater: {position_increment}"
            )));
        }
        self.position_increment = position_increment;
        Ok(())
    }

    /// Returns the position increment of this Token.
    fn get_position_increment(&self) -> i32 {
        self.position_increment
    }
}
impl PositionLengthAttribute for PackedTokenAttributeImpl {
    /// Set the position length of this Token.
    fn set_position_length(&mut self, position_length: i32) -> Result<()> {
        if position_length < 1 {
            return Err(LuceneError::illegal_argument(format!(
                "Position length must be 1 or greater: got {position_length}"
            )));
        }
        self.position_length = position_length;
        Ok(())
    }

    /// Returns the position length of this Token.
    fn get_position_length(&self) -> i32 {
        self.position_length
    }
}
impl OffsetAttribute for PackedTokenAttributeImpl {
    /// Returns this token’s starting offset—the position of the first character corresponding to this token in the source text.
    ///
    /// Note that the difference between [`end_offset()`](Self::end_offset) and `start_offset()` may not equal `term_text.len()`, as the term text may have been altered by a stemmer or another filter.
    fn start_offset(&self) -> i32 {
        self.start_offset
    }
    /// Set the starting and ending offset.
    fn set_offset(&mut self, start_offset: i32, end_offset: i32) -> Result<()> {
        if start_offset < 0 || end_offset < start_offset {
            return Err(LuceneError::illegal_argument(format!(
                "start_offset must be non-negative, and end_offset must be >= start_offset; got start_offset={start_offset}, end_offset={end_offset}"
            )));
        }
        self.start_offset = start_offset;
        self.end_offset = end_offset;
        Ok(())
    }
    /// Returns this token’s ending offset—one greater than the position of the last character corresponding to this token in the source text.
    /// The length of the token in the source text is `(end_offset() - start_offset())`.
    fn end_offset(&self) -> i32 {
        self.end_offset
    }
}
impl TermFrequencyAttribute for PackedTokenAttributeImpl {
    fn set_term_frequency(&mut self, term_frequency: i32) -> Result<()> {
        if term_frequency < 1 {
            return Err(LuceneError::illegal_argument(format!(
                "Term frequency must be 1 or greater; got {term_frequency}"
            )));
        }
        self.term_frequency = term_frequency;
        Ok(())
    }

    fn get_term_frequency(&self) -> i32 {
        self.term_frequency
    }
}

impl Clone for PackedTokenAttributeImpl {
    fn clone(&self) -> Self {
        Self {
            start_offset: self.start_offset,
            end_offset: self.end_offset,
            type_: self.type_.clone(),
            position_increment: self.position_increment,
            position_length: self.position_length,
            term_frequency: self.term_frequency,
            base: self.base.clone(),
        }
    }
}

impl AttributeImpl for PackedTokenAttributeImpl {
    /// Resets the attributes
    fn clear(&mut self) {
        self.base.clear();
        self.start_offset = 0;
        self.end_offset = 0;
        self.type_ = ta_util::DEFAULT_TYPE.to_string();
        self.position_increment = 1;
        self.position_length = 1;
        self.term_frequency = 1;
    }

    /// Resets the attributes at end
    fn end(&mut self) {
        self.base.end();
        self.position_increment = 0;
    }

    type AttributeImpl = PackedTokenAttributeImpl;

    fn copy_to(&mut self, to: &mut Self::AttributeImpl) {
        let len = self.base.length();
        let buf = self.base.buffer();
        to.base.copy_buffer(buf, 0, len);
        to.position_increment = self.position_increment;
        to.position_length = self.position_length;
        to.start_offset = self.start_offset;
        to.end_offset = self.end_offset;
        to.type_ = self.type_.clone();
        to.term_frequency = self.term_frequency;
    }
}
impl Hash for PackedTokenAttributeImpl {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start_offset.hash(state);
        self.end_offset.hash(state);
        self.type_.hash(state);
        self.position_increment.hash(state);
        self.position_length.hash(state);
        self.term_frequency.hash(state);
        self.base.hash(state);
    }
}
impl PartialEq for PackedTokenAttributeImpl {
    fn eq(&self, other: &Self) -> bool {
        self.start_offset == other.start_offset
            && self.end_offset == other.end_offset
            && self.position_increment == other.position_increment
            && self.position_length == other.position_length
            && self.term_frequency == other.term_frequency
            && self.type_ == other.type_
    }
}
