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
use crate::index::index_deletion_policy::IndexDeletionPolicy;
use crate::index::index_file_deleter::IndexFileDeleter;
use crate::index::merge_state::DocMap;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct IndexWriter<D, P>
where
    D: Directory,
    P: IndexDeletionPolicy,
{
    tragedy: TragicException,
    closed: bool,
    closing: bool,
    deleter: IndexFileDeleter<D, P>,
}

impl<D, P> IndexWriter<D, P>
where
    D: Directory,
    P: IndexDeletionPolicy,
{
    pub fn set_live_commit_data(&self) {}

    pub fn ensure_open(&self, fail_if_closing: bool) -> Result<()> {
        if self.closed || (fail_if_closing && self.closing) {
            let tragedy = self.tragedy.lock();
            let error_opt = tragedy.as_ref();
            match error_opt {
                Some(err) => Err(LuceneError::already_closed(format!("{err}"))),
                None => Err(LuceneError::illegal_state("no tragic error set")),
            }
        } else {
            Ok(())
        }
    }

    pub fn get_tragic_exception(&self) -> TragicException {
        self.tragedy.clone()
    }
    pub(crate) fn is_deleter_closed(&self) -> Result<bool> {
        self.deleter.is_closed(self)
    }
}
type TragicException = Arc<Mutex<Option<LuceneError>>>;

pub mod index_writer_util {

    use crate::util::array_util::ArrayUtil;
    use crate::util::byte_block_pool_util;
    use crate::util::unicode_util::UnicodeUtil;

    /// Maximum number of documents. In Java Lucene, We subtract 128 to ensure
    /// it's well below the typical JVM's `ArrayUtil.MAX_ARRAY_LENGTH` and
    /// avoid potential overflow issues across JVM implementations.
    /// In Rust Lucene, we keep the same value for consistency.
    pub const MAX_DOCS: i32 = i32::MAX - 128;
    /// Maximum value for the token position in an indexed field.
    pub const MAX_POSITION: i32 = i32::MAX - 128;
    /// A variable that holds the actual maximum number of documents, which can
    /// be adjusted for testing purposes.
    pub const ACTUAL_MAX_DOCS: i32 = MAX_DOCS;

    pub const MAX_TERM_LENGTH: i32 = byte_block_pool_util::BYTE_BLOCK_SIZE - 1;
    pub const WRITE_LOCK_NAME: &str = "write.lock";
    pub const MAX_STORED_STRING_LENGTH: i32 =
        ArrayUtil::MAX_ARRAY_LENGTH as i32 / UnicodeUtil::MAX_UTF8_BYTES_PER_CHAR;
    pub fn get_actual_max_docs() -> i32 {
        ACTUAL_MAX_DOCS
    }
}
#[derive(Default)]
pub struct DocMapIndexWriter;
impl DocMap for DocMapIndexWriter {
    fn get(&self, _doc_id: i32) -> i32 {
        todo!()
    }
}
