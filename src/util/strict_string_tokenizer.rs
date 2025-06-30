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
use crate::util::error::IllegalStateError;

/// Used for parsing version strings so we don't have to use the overkill of
/// `String.split` or `StringTokenizer` (which silently skips empty tokens).
pub struct StrictStringTokenizer<'a> {
    s: &'a str,
    delimiter: char,
    pos: Option<usize>,
}

impl<'a> StrictStringTokenizer<'a> {
    pub fn new(s: &'a str, delimiter: char) -> Self {
        Self {
            s,
            delimiter,
            pos: Some(0),
        }
    }

    pub(crate) fn next_token(&mut self) -> Result<&'a str, IllegalStateError> {
        if let Some(start) = self.pos {
            if start >= self.s.len() {
                self.pos = None;
                return Err(IllegalStateError::new("no more tokens"));
            }

            if let Some(end) = self.s[start..].find(self.delimiter) {
                let token = &self.s[start..start + end];
                self.pos = Some(start + end + 1);
                Ok(token)
            } else {
                let token = &self.s[start..];
                self.pos = None;
                Ok(token)
            }
        } else {
            Err(IllegalStateError::new("no more tokens"))
        }
    }

    pub(crate) fn has_more_tokens(&self) -> bool {
        self.pos.is_some()
    }
}
