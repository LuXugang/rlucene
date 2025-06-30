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
use crate::index::merge_state::DocMap;
use crate::util::array_util::ArrayUtil;
use crate::util::byte_block_pool_util;
use crate::util::unicode_util::UnicodeUtil;

pub struct IndexWriter;

impl IndexWriter {
    /// Maximum number of documents. In Java Lucene, We subtract 128 to ensure
    /// it's well below the typical JVM's `ArrayUtil.MAX_ARRAY_LENGTH` and
    /// avoid potential overflow issues across JVM implementations.
    /// In Rust Lucene, we keep the same value for consistency.
    pub const MAX_DOCS: i32 = i32::MAX - 128;
    /// Maximum value for the token position in an indexed field.
    pub const MAX_POSITION: i32 = i32::MAX - 128;
    /// A variable that holds the actual maximum number of documents, which can
    /// be adjusted for testing purposes.
    pub const ACTUAL_MAX_DOCS: i32 = Self::MAX_DOCS;

    pub const MAX_TERM_LENGTH: i32 = byte_block_pool_util::BYTE_BLOCK_SIZE - 1;
    pub const MAX_STORED_STRING_LENGTH: i32 =
        ArrayUtil::MAX_ARRAY_LENGTH as i32 / UnicodeUtil::MAX_UTF8_BYTES_PER_CHAR;
    pub fn set_live_commit_data(&self) {}

    pub fn get_actual_max_docs() -> i32 {
        IndexWriter::ACTUAL_MAX_DOCS
    }
}

#[derive(Default)]
pub struct DocMapIndexWriter;
impl DocMap for DocMapIndexWriter {
    fn get(&self, _doc_id: i32) -> i32 {
        todo!()
    }
}
