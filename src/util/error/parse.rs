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
use crate::util::error::IllegalArgumentError;

#[derive(Debug)]
pub struct Parse {
    pub message: String,
    pub position: i32,
    pub error: Option<IllegalArgumentError>,
}

impl Parse {
    pub fn new(msg: impl Into<String>, position: i32) -> Self {
        Self {
            message: msg.into(),
            position,
            error: None,
        }
    }
    pub fn with_error(msg: impl Into<String>, error: Option<IllegalArgumentError>) -> Self {
        Self {
            message: msg.into(),
            position: 0,
            error,
        }
    }
}

impl std::fmt::Display for Parse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.error.is_some() {
            write!(
                f,
                "Parse Error at {}: {} reason: {}",
                self.position,
                self.message,
                self.error.as_ref().unwrap().message
            )
        } else {
            write!(f, "Parse Error at {}: {}", self.position, self.message)
        }
    }
}

impl std::error::Error for Parse {}
