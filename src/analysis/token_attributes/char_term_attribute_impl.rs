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
use crate::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::array_util::ArrayUtil;
use crate::util::attribute::Attribute;
use crate::util::attribute_impl::AttributeImpl;
use crate::util::error::lucene_error::Result;
use crate::util::{CoreHelper, HashCode, SliceCopyOps};
use std::borrow::Cow;
use std::fmt::Display;

/// Default implementation of [`CharTermAttribute`].
pub struct CharTermAttributeImpl {
    term_buffer: Vec<char>,
    term_length: usize,
    /// May be used by subclasses to convert to different charsets / encodings for implementing [`get_bytes_ref`](Self::get_bytes_ref).
    pub(crate) builder: BytesRefBuilder<Vec<u8>>,
}
impl CharTermAttributeImpl {
    const MIN_BUFFER_SIZE: usize = 10;

    pub fn new() -> Self {
        // TODO: _bytes_per_element not Specific
        let size = ArrayUtil::oversize(Self::MIN_BUFFER_SIZE, 0);
        Self {
            term_buffer: vec!['\0'; size],
            term_length: 0,
            builder: BytesRefBuilder::new(),
        }
    }
    fn grow_term_buffer(&mut self, new_size: usize) {
        if self.term_buffer.len() < new_size {
            // Not big enough; create a new array with slight
            // over allocation:
            // TODO: _bytes_per_element not Specific
            let new_capacity = ArrayUtil::oversize(new_size, 0);
            self.term_buffer = vec!['\0'; new_capacity];
        }
    }

    pub fn char_at(&self, index: usize) -> char {
        self.term_buffer[index]
    }
    pub fn sub_sequence(&self, start: usize, end: usize) -> &[char] {
        &self.term_buffer[start..end]
    }
    fn append_null(&mut self) -> &mut Self {
        self.resize_buffer(self.term_length + 4);
        self.term_buffer[self.term_length] = 'n';
        self.term_buffer[self.term_length + 1] = 'u';
        self.term_buffer[self.term_length + 2] = 'l';
        self.term_buffer[self.term_length + 3] = 'l';
        self.term_length += 4;
        self
    }
}

impl Attribute for CharTermAttributeImpl {}

impl CharTermAttribute for CharTermAttributeImpl {
    fn length(&self) -> usize {
        self.term_length
    }

    fn copy_buffer(&mut self, buffer: &[char], offset: usize, length: usize) {
        self.grow_term_buffer(length);
        self.term_buffer
            .copy_from(&buffer[offset..offset + length], 0);
        self.term_length = length
    }

    fn buffer(&mut self) -> &mut [char] {
        todo!()
    }

    fn resize_buffer(&mut self, new_size: usize) -> &mut [char] {
        if self.term_buffer.len() < new_size {
            // Not big enough; create a new array with slight
            // over allocation:
            // TODO: _bytes_per_element not Specific
            let new_capacity = ArrayUtil::oversize(new_size, std::mem::size_of::<char>());
            ArrayUtil::grow_with_len(&mut self.term_buffer, new_capacity);
        }
        &mut self.term_buffer
    }

    fn set_length(&mut self, length: usize) -> Result<&mut Self> {
        debug_assert!(self.term_buffer.len() <= i32::MAX as usize);
        CoreHelper::check_from_index_size(0, length as i32, self.term_buffer.len() as i32)?;
        self.term_length = length;
        Ok(self)
    }

    fn set_empty(&mut self) -> &mut Self {
        self.term_length = 0;
        self
    }

    fn append_range(&mut self, csq: &str, start: usize, end: usize) -> &mut Self {
        todo!()
    }

    fn append_char(&mut self, c: char) -> &mut Self {
        self.resize_buffer(self.term_length + 1);
        self.term_buffer[self.term_length] = c;
        self.term_length += 1;
        self
    }

    fn append_str(&mut self, s: Option<&str>) -> &mut Self {
        if s.is_none() {
            return self.append_null();
        }
        let s = s.unwrap();
        let len = s.len();
        self.resize_buffer(self.term_length + len);
        self.term_buffer
            .copy_from(&s.chars().collect::<Vec<char>>()[0..len], self.term_length);
        self.term_length += len;
        self
    }

    fn append_term_attribute(&mut self, ta: Option<&mut impl CharTermAttribute>) -> &mut Self {
        if let Some(other) = ta {
            let len = other.length();
            self.resize_buffer(self.term_length + len);
            self.term_buffer
                .copy_from(&other.buffer()[0..len], self.term_length);
            self.term_length += len;
            self
        } else {
            self.append_null()
        }
    }
}
impl TermToBytesRefAttribute for CharTermAttributeImpl {
    fn get_bytes_ref(&mut self) -> Cow<BytesRef<Vec<u8>>> {
        self.builder
            .copy_chars_with_chars(&self.term_buffer, 0, self.term_length);
        Cow::Borrowed(&self.builder.bytes_ref)
    }
}
impl AttributeImpl for CharTermAttributeImpl {
    fn clear(&mut self) {
        self.term_length = 0;
    }
}
impl HashCode for CharTermAttributeImpl {
    fn hash_code(&self) -> i32 {
        let mut code = self.term_length as i32;
        code = code.wrapping_mul(31).wrapping_add(ArrayUtil::hash_code(
            &self.term_buffer,
            0,
            self.term_length,
        ));
        code
    }
}
impl PartialEq for CharTermAttributeImpl {
    fn eq(&self, other: &Self) -> bool {
        if self.term_length != other.term_length {
            return false;
        }
        self.term_buffer[..self.term_length] == other.term_buffer[..other.term_length]
    }
}
impl Display for CharTermAttributeImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = self.term_buffer[..self.term_length].iter().collect();
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::token_attributes::char_term_attribute::CharTermAttribute;
    use crate::analysis::token_attributes::char_term_attribute_impl::CharTermAttributeImpl;

    #[test]
    fn test_resize() {
        let mut t = CharTermAttributeImpl::new();
        let content: Vec<char> = "hello".chars().collect();
        t.copy_buffer(&content, 0, content.len());

        for i in 0..2000 {
            let buf = t.resize_buffer(i);
            assert!(
                i <= buf.len(),
                "buffer.len() = {}, expected >= {}",
                buf.len(),
                i
            );
            assert_eq!(t.to_string(), "hello");
        }
    }
}
