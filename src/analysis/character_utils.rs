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
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::SliceCopyOps;
use std::io::{Read, Take};

pub struct CharacterUtils;
impl CharacterUtils {
    pub fn new_character_buffer(buffer_size: usize) -> Result<CharacterBuffer> {
        if buffer_size == 0 {
            return Err(LuceneError::illegal_argument("buffer_size must be > 0"));
        }
        Ok(CharacterBuffer::new(vec!['\0'; buffer_size], 0, 0))
    }
    pub fn get_lower_case(buffer: &mut [char], offset: usize, limit: usize) {
        debug_assert!(buffer.len() >= limit);
        debug_assert!(offset <= buffer.len());

        for i in offset..limit {
            buffer[i] = buffer[i].to_lowercase().next().unwrap_or(buffer[i]);
        }
    }
    pub fn get_upper_case(buffer: &mut [char], offset: usize, limit: usize) {
        assert!(buffer.len() >= limit);
        assert!(offset <= buffer.len());

        for i in offset..limit {
            buffer[i] = buffer[i].to_uppercase().next().unwrap_or(buffer[i]);
        }
    }
    pub fn get_code_points(
        src: &[char],
        src_off: usize,
        src_len: usize,
        dest: &mut [i32],
        dest_off: usize,
    ) -> Result<usize> {
        if src_len > src.len().saturating_sub(src_off) {
            return Err(LuceneError::illegal_argument(
                "src_off + src_len out of bounds",
            ));
        }
        if dest_off > dest.len() || src_len > dest.len().saturating_sub(dest_off) {
            return Err(LuceneError::illegal_argument(
                "dest_off + src_len out of bounds",
            ));
        }

        let mut count = 0;
        for i in 0..src_len {
            dest[dest_off + count] = src[src_off + i] as i32;
            count += 1;
        }
        Ok(count)
    }
    pub fn get_chars(
        src: &[i32],
        src_off: usize,
        src_len: usize,
        dest: &mut [char],
        dest_off: usize,
    ) -> Result<usize> {
        let mut written = 0;
        for i in src_off..src_off + src_len {
            let cp = u32::try_from(src[i])
                .map_err(|_| LuceneError::illegal_argument("code point must be >= 0"))?;
            let ch = std::char::from_u32(cp)
                .ok_or_else(|| LuceneError::illegal_argument("invalid Unicode code point"))?;
            dest[dest_off + written] = ch;
            written += 1;
        }
        Ok(written)
    }
    pub fn fill_with_num<R: Read>(
        buffer: &mut CharacterBuffer,
        reader: &mut R,
        num_chars: usize,
    ) -> Result<bool> {
        if num_chars < 1 || num_chars > buffer.buffer.len() {
            return Err(LuceneError::illegal_argument(
                "num_chars must be >= 1 and <= buffer size",
            ));
        }

        buffer.offset = 0;
        let read = Self::read_fully(reader, &mut buffer.buffer, 0, num_chars)?;
        buffer.length = read;
        Ok(read == num_chars)
    }
    pub fn fill<R: Read>(buffer: &mut CharacterBuffer, reader: &mut R) -> Result<bool> {
        Self::fill_with_num(buffer, reader, buffer.buffer.len())
    }
    pub fn read_fully<R: Read>(
        reader: &mut R,
        dest: &mut [char],
        offset: usize,
        len: usize,
    ) -> Result<usize> {
        if offset > dest.len() {
            return Err(LuceneError::illegal_state("offset is out of bounds"));
        }
        if len > dest.len().saturating_sub(offset) {
            return Err(LuceneError::illegal_state("dest is too small"));
        }

        let mut limited: Take<&mut R> = reader.take(len as u64);

        let mut s = String::new();
        limited.read_to_string(&mut s)?;
        let chars: Vec<char> = s.chars().take(len).collect();
        let count = chars.len();
        dest.copy_from(&chars[0..count], offset);
        Ok(count)
    }
}

pub struct CharacterBuffer {
    buffer: Vec<char>,
    offset: usize,
    length: usize,
}
impl CharacterBuffer {
    pub fn new(buffer: Vec<char>, offset: usize, length: usize) -> Self {
        CharacterBuffer {
            buffer,
            offset,
            length,
        }
    }
    /// Returns the internal buffer
    pub fn get_buffer(&self) -> &[char] {
        &self.buffer
    }

    /// Returns the data offset in the internal buffer.
    pub fn get_offset(&self) -> usize {
        self.offset
    }
    /// Return the length of the data in the internal buffer starting at [`getOffset()`](Self::get_offset)
    pub fn length(&self) -> usize {
        self.length
    }

    /// Resets the CharacterBuffer. All internals are reset to its default values.
    pub fn reset(&mut self) {
        self.offset = 0;
        self.length = 0;
    }
    pub fn as_string(&self) -> String {
        self.buffer[self.offset..self.offset + self.length]
            .iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::character_utils::CharacterUtils;
    use crate::test::util::lucene_test_case::random;
    use crate::test::util::test_util::TestUtil;
    use crate::util::array_util::ArrayUtil;
    use crate::util::error::lucene_error::{LuceneError, Result};
    use std::io::Cursor;
    #[test]
    fn test_lower_upper() -> Result<()> {
        let data = "ABc".to_string();
        let mut reader = Cursor::new(data.into_bytes());
        let mut buffer = CharacterUtils::new_character_buffer(3)?;
        assert!(CharacterUtils::fill_with_num(&mut buffer, &mut reader, 3)?);
        assert_eq!(buffer.length, 3);
        CharacterUtils::get_lower_case(&mut buffer.buffer, 1, 3);
        let s: String = buffer.buffer.iter().collect();
        assert_eq!(s, "Abc");
        CharacterUtils::get_upper_case(&mut buffer.buffer, 1, 3);
        let s2: String = buffer.buffer.iter().collect();
        assert_eq!(s2, "ABC");
        Ok(())
    }
    #[test]
    fn test_conversions() -> Result<()> {
        let mut random = random();
        let orig: Vec<char> = TestUtil::random_unicode_string(&mut random)
            .chars()
            .collect();
        let mut buf = vec![0i32; orig.len()];
        let mut restored = vec!['\0'; buf.len()];
        let o1 = TestUtil::next_int(&mut random, 0, orig.len().min(5) as i32) as usize;
        let o2 = TestUtil::next_int(&mut random, 0, o1 as i32) as usize;
        let o3 = TestUtil::next_int(&mut random, 0, o1 as i32) as usize;
        let code_point_count =
            CharacterUtils::get_code_points(&orig, o1, orig.len() - o1, &mut buf, o2)?;
        let char_count = CharacterUtils::get_chars(&buf, o2, code_point_count, &mut restored, o3)?;
        assert_eq!(orig.len() - o1, char_count);
        let orig_sub = ArrayUtil::copy_of_sub_array(&orig, o1, o1 + char_count);
        let restored_sub = ArrayUtil::copy_of_sub_array(&restored, o3, o3 + char_count);
        assert_eq!(orig_sub, restored_sub);
        Ok(())
    }
    #[test]
    fn test_new_character_buffer() -> Result<()> {
        let cb1 = CharacterUtils::new_character_buffer(1024)?;
        assert_eq!(cb1.buffer.len(), 1024);
        assert_eq!(cb1.offset, 0);
        assert_eq!(cb1.length, 0);

        let cb2 = CharacterUtils::new_character_buffer(2)?;
        assert_eq!(cb2.buffer.len(), 2);
        assert_eq!(cb2.offset, 0);
        assert_eq!(cb2.length, 0);

        let result = CharacterUtils::new_character_buffer(0);
        assert!(matches!(result, Err(LuceneError::IllegalArgument(_))));
        Ok(())
    }
    #[test]
    fn test_fill_no_high_surrogate() -> Result<()> {
        let mut reader = Cursor::new("helloworld".as_bytes());
        let mut buffer = CharacterUtils::new_character_buffer(6)?;
        assert!(CharacterUtils::fill_with_num(&mut buffer, &mut reader, 6)?);
        assert_eq!(buffer.offset, 0);
        assert_eq!(buffer.length, 6);
        let s: String = buffer.get_buffer().iter().collect();
        assert_eq!(s, "hellow");
        assert!(!CharacterUtils::fill_with_num(&mut buffer, &mut reader, 6)?);
        assert_eq!(buffer.offset, 0);
        assert_eq!(buffer.length, 4);
        let s2: String = buffer.buffer[buffer.offset..buffer.offset + buffer.length]
            .iter()
            .collect();
        assert_eq!(s2, "orld");
        assert!(!CharacterUtils::fill_with_num(&mut buffer, &mut reader, 6)?);

        Ok(())
    }
    #[test]
    fn test_fill() -> Result<()> {
        // let input = "1234𐐜789123?𐐜?";
        // let mut reader: &[u8] = input.as_bytes();
        // let char: Vec<char> = input.chars().collect();
        // let mut buffer = CharacterUtils::new_character_buffer(100)?;
        //
        // assert!(CharacterUtils::fill(&mut buffer, &mut reader)?);
        // assert_eq!(buffer.length, 4);
        // assert_eq!(buffer.as_string(), "1234𐐜");
        //
        // assert!(CharacterUtils::fill(&mut buffer, &mut reader)?);
        // assert_eq!(buffer.length, 5);
        // assert_eq!(buffer.as_string(), "78912");
        //
        // assert!(!CharacterUtils::fill(&mut buffer, &mut reader)?);
        // assert_eq!(buffer.length, 4);
        // assert_eq!(buffer.as_string(), "3𝄜𝄜𝄜");
        //
        // assert!(!CharacterUtils::fill(&mut buffer, &mut reader)?);
        // assert_eq!(buffer.length, 3);

        Ok(())
    }
}
