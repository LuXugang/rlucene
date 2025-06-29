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
use std::env;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadAdvice {
    ///  Normal behavior. Data is expected to be read mostly sequentially. The
    /// system is expected to cache the hottest pages.
    Normal,
    ///Data is expected to be read in a random-access fashion, either by
    /// `IndexInput#seek(i64)` seeking often and reading relatively i16
    /// sequences of bytes at once, or by reading data through the
    /// `RandomAccessInput` abstraction in random order.
    Random,
    /// Data is expected to be read sequentially with very little seeking at
    /// most. The system may read ahead aggressively and free pages soon
    /// after they are accessed.
    Sequential,
    ///
    ///Data is treated as random-access memory in practice. `Directory`
    /// implementations may explicitly load the content of the file in
    /// memory, or provide hints to the system so that it loads the content
    /// of the file into the page cache at open time. This should only be used
    /// on very small files that can be expected to fit in RAM with very
    /// high confidence.
    RandomPreload,
}

impl ReadAdvice {
    pub fn from_str_custom(s: &str) -> Option<ReadAdvice> {
        match s.to_uppercase().as_str() {
            "NORMAL" => Some(ReadAdvice::Normal),
            "RANDOM" => Some(ReadAdvice::Random),
            "SEQUENTIAL" => Some(ReadAdvice::Sequential),
            "RANDOM PRELOAD" => Some(ReadAdvice::RandomPreload),
            _ => None,
        }
    }
    pub fn default_read_advice() -> ReadAdvice {
        env::var("lucene.store.defaultReadAdvice")
            .ok()
            .and_then(|value| ReadAdvice::from_str_custom(&value))
            .unwrap_or(ReadAdvice::Random)
    }
}
