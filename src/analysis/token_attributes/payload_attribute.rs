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
use crate::index::BytesRef;
use crate::util::attribute::Attribute;
/// The payload of a token.
///
/// The payload is stored in the index at each position, and can be used to
/// influence scoring when using payload-based queries.
///
/// **Note:** Because the payload will be stored at each position, it's usually
/// best to use the minimum number of bytes necessary. Some codec
/// implementations may optimize payload storage when all payloads have the same
/// length.
///
/// See also: [`PostingsEnum`](crate::index::postings_enum::PostingsEnum)
pub trait PayloadAttribute: Attribute {
    /// Returns this token's payload.
    ///
    /// See also: [`Self::set_payload`]
    fn get_payload(&self) -> &BytesRef<Vec<u8>>;
    /// Sets this token's payload.
    ///
    /// See also: [`Self::get_payload`]
    fn set_payload(&mut self, payload: BytesRef<Vec<u8>>);
}
