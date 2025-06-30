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
use crate::util::attribute::Attribute;

/// This attribute can be used to pass different flags down the `Tokenizer` chain, e.g. from
/// one `TokenFilter` to another one.
///
/// This is completely distinct from [`TypeAttribute`](crate::analysis::token_attributes::type_attribute::TypeAttribute), although they do share similar
/// purposes. The flags can be used to encode information about the token for use by other
/// `TokenFilter`s.

pub trait FlagsAttribute: Attribute {
    /// Get the bitset for any bits that have been set.
    fn get_flags(&self) -> i32;
    /// Set the flags to a new bitset.
    fn set_flags(&mut self, flags: i32);
}
