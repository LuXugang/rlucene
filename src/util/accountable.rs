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
use crate::util::error::lucene_error::Result;
/// An object whose RAM usage can be computed.
///
/// # Note
/// This is an internal API.
pub trait Accountable {
    /// Return the memory usage of this object in bytes. Negative values are
    /// illegal.
    fn ram_bytes_used(&self) -> Result<i64>;

    /// Returns nested resources of this struct. The result should be a
    /// point-in-time snapshot (to avoid race conditions).
    fn get_child_resources<T: Accountable>(&self) -> Vec<T> {
        vec![]
    }
}

#[allow(unused)]
struct EmptyAccountable;
impl EmptyAccountable {
    #[allow(unused)]
    pub fn new() -> Self {
        EmptyAccountable
    }
}
impl Accountable for EmptyAccountable {
    fn ram_bytes_used(&self) -> Result<i64> {
        Ok(0)
    }
}
