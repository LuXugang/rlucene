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
