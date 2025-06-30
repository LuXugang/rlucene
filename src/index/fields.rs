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
use crate::index::terms::Terms;
use crate::util::error::lucene_error::Result;
/// Provides a [`Terms`] index for fields that have it, and lists which fields
/// do.
///
/// This is primarily an internal/experimental API (see
/// [`FieldsProducer`](crate::codecs::fields_producer::FieldsProducer)),
/// although it is also used to expose the set of term vectors per document.
pub trait Fields {
    /// Returns an iterator that will step through all field names.
    /// This will not return `None`.
    fn iterator(&self) -> impl Iterator<Item = &String>;

    type Terms: Terms;
    /// Get the [`Terms`] for this field. This will return `None` if the field
    /// does not exist.
    fn terms(&self, field: &str) -> Result<Option<Self::Terms>>;

    /// Returns the number of fields or -1 if the number of distinct field names
    /// is unknown. If >= 0, [`iterator`](Self::iterator) will return as many field names.
    fn size(&self) -> Result<i32>;
}
