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
use crate::analysis::reader::{Reader, ReaderEnum};
use crate::util::error::lucene_error::Result;
/// `CharFilter` can be chained to filter a `Reader`.
/// They can be used as a `Reader` with additional offset correction.
/// [`Tokenizer`](crate::analysis::tokenizer::Tokenizer)s will automatically use [`correct_offset`](Self::correct_offset) if a `CharFilter` subclass is used.
pub trait CharFilter: Reader {
    /// The underlying character-input stream.
    fn get_reader(&self) -> &ReaderEnum;
    fn get_reader_mut(&mut self) -> &mut ReaderEnum;
    /// Closes the underlying input stream.
    fn close(&mut self) -> Result<()> {
        self.get_reader_mut().close()
    }
    /// override to correct the current offset.
    fn correct(&self, current_off: i32) -> i32;
    /// Chains the corrected offset through the input CharFilter(s).
    fn correct_offset(&self, current_off: i32) -> i32 {
        let corrected = self.correct(current_off);
        let base_reader = self.get_reader();
        base_reader.correct_offset(corrected)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::analysis::char_filter::CharFilter;
    use crate::analysis::reader::{Reader, ReaderEnum};
    use crate::analysis::reusable_string_reader::ReusableStringReader;
    use crate::util::error::lucene_error::Result;

    #[allow(dead_code)]
    struct TestCharFilter;
    #[test]
    fn test_char_filter1() -> Result<()> {
        let mut reader = ReusableStringReader::new();
        reader.set_value("");
        let cs = CharFilter1::new(ReaderEnum::ReusedString(reader));
        assert_eq!(1, cs.correct_offset(0), "corrected offset is invalid");
        Ok(())
    }
    #[test]
    fn test_char_filter2() -> Result<()> {
        let mut reader = ReusableStringReader::new();
        reader.set_value("");
        let cs = CharFilter2::new(ReaderEnum::ReusedString(reader));
        assert_eq!(2, cs.correct_offset(0), "corrected offset is invalid");
        Ok(())
    }

    #[test]
    fn test_char_filter12() -> Result<()> {
        let mut reader = ReusableStringReader::new();
        reader.set_value("");
        let cs = CharFilter2::new(ReaderEnum::CharFilter1(CharFilter1::new(
            ReaderEnum::ReusedString(reader),
        )));
        assert_eq!(3, cs.correct_offset(0), "corrected offset is invalid");
        Ok(())
    }

    #[test]
    fn test_char_filter11() -> Result<()> {
        let mut reader = ReusableStringReader::new();
        reader.set_value("");
        let cs = CharFilter1::new(ReaderEnum::CharFilter1(CharFilter1::new(
            ReaderEnum::ReusedString(reader),
        )));
        assert_eq!(2, cs.correct_offset(0), "corrected offset is invalid");
        Ok(())
    }

    #[derive(Clone, Debug)]
    pub struct CharFilter1 {
        input: Box<ReaderEnum>,
    }
    impl CharFilter1 {
        pub fn new(input: ReaderEnum) -> Self {
            Self {
                input: Box::new(input),
            }
        }
    }

    impl Reader for CharFilter1 {
        fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
            self.input.read_range(buf, off, len)
        }

        fn close(&mut self) -> Result<()> {
            CharFilter::close(self)
        }
    }

    impl CharFilter for CharFilter1 {
        fn get_reader(&self) -> &ReaderEnum {
            &self.input
        }

        fn get_reader_mut(&mut self) -> &mut ReaderEnum {
            &mut self.input
        }

        fn correct(&self, current_off: i32) -> i32 {
            current_off + 1
        }
    }
    #[derive(Clone, Debug)]
    pub struct CharFilter2 {
        input: Box<ReaderEnum>,
    }
    impl CharFilter2 {
        pub fn new(input: ReaderEnum) -> Self {
            Self {
                input: Box::new(input),
            }
        }
    }

    impl Reader for CharFilter2 {
        fn read_range(&mut self, buf: &mut [char], off: usize, len: usize) -> Result<i32> {
            self.input.read_range(buf, off, len)
        }

        fn close(&mut self) -> Result<()> {
            CharFilter::close(self)
        }
    }

    impl CharFilter for CharFilter2 {
        fn get_reader(&self) -> &ReaderEnum {
            &self.input
        }

        fn get_reader_mut(&mut self) -> &mut ReaderEnum {
            &mut self.input
        }

        fn correct(&self, current_off: i32) -> i32 {
            current_off + 2
        }
    }
}
