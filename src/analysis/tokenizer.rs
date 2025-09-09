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
use crate::analysis::token_stream::TokenStream;
use crate::util::error::lucene_error::{LuceneError, Result};
/// A `Tokenizer` is a `TokenStream` whose input is a `Reader`.
pub trait Tokenizer: TokenStream {
    fn get_tokenizer_base(&self) -> &mut TokenizerBase;
    /// Releases resources associated with this stream.
    fn close(&mut self) -> Result<()> {
        let base = self.get_tokenizer_base();
        base.input.close()?;
        base.input = ReaderEnum::IllegalState(IllegalStateReader);
        base.input_pending = ReaderEnum::IllegalState(IllegalStateReader);
        Ok(())
    }
    /// Return the corrected offset.
    /// If input is a CharFilter this method calls CharFilter.correctOffset else returns currentOff.
    fn correct_offset(&self, current_off: i32) -> i32 {
        let base = self.get_tokenizer_base();
        base.input.correct_offset(current_off)
    }

    /// Expert: Set a new reader on the Tokenizer.
    /// Typically, an analyzer (in its tokenStream method) will use this to re-use a previously created tokenizer.
    fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
        let base = self.get_tokenizer_base();
        if !matches!(base.input, ReaderEnum::IllegalState(_)) {
            return Err(LuceneError::illegal_state(
                "TokenStream contract violation: close() call missing",
            ));
        }
        base.input_pending = input;
        self.set_reader_test_point();
        Ok(())
    }
    fn reset(&mut self) -> Result<()> {
        TokenStream::reset(self)?;
        let base = self.get_tokenizer_base();
        base.input = std::mem::take(&mut base.input_pending);
        base.input_pending = ReaderEnum::IllegalState(IllegalStateReader);
        Ok(())
    }
    fn set_reader_test_point(&mut self) {}
}

pub struct TokenizerBase {
    /// The text source for this Tokenizer.
    pub(crate) input: ReaderEnum,
    /// Pending reader: not actually assigned to input until reset()
    pub(crate) input_pending: ReaderEnum,
}
impl Default for TokenizerBase {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenizerBase {
    pub fn new() -> Self {
        Self {
            input_pending: ReaderEnum::IllegalState(IllegalStateReader),
            input: ReaderEnum::IllegalState(IllegalStateReader),
        }
    }
    pub fn set_reader(&mut self, input: ReaderEnum) {
        self.input_pending = input;
    }
    pub fn reset(&mut self) -> Result<()> {
        self.input = self.input_pending.clone();
        Ok(())
    }
}
#[derive(Debug, Clone, Default)]
pub struct IllegalStateReader;
impl Reader for IllegalStateReader {
    fn read_range(&mut self, _buf: &mut [char], _off: usize, _len: usize) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "TokenStream contract violation: reset()/close() call missing, \
reset() called multiple times, or subclass does not call super.reset(). \
Please see Javadocs of TokenStream class for more information about the correct consuming workflow.",
        ))
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }
}
