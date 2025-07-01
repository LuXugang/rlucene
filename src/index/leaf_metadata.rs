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
use derive_getters::Getters;

use crate::index::sort::Sort;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::version::{Version, LATEST};

#[derive(Getters)]
pub struct LeafMetaData {
    /// The major version of the Lucene format used to create this segment.
    pub created_version_major: i32,
    /// The minimum version of Lucene that contributed to this segment.
    pub min_version: Option<Version>,
    /// The sort order of documents in this segment, if any.
    pub sort: Option<Sort>,
    /// Indicates whether this segment contains documents written as blocks.
    pub has_blocks: bool,
}

impl LeafMetaData {
    /// Constructs a new `LeafMetaData` instance.
    pub fn new(
        created_version_major: i32,
        min_version: Option<Version>,
        sort: Option<Sort>,
        has_blocks: bool,
    ) -> Result<Self> {
        if created_version_major > LATEST.major {
            return Err(LuceneError::illegal_argument(format!(
                "created_version_major is in the future: {created_version_major}"
            )));
        }
        if created_version_major < 6 {
            return Err(LuceneError::illegal_argument(format!(
                "created_version_major must be >= 6, got: {created_version_major}"
            )));
        }
        if created_version_major >= 7 && min_version.is_none() {
            return Err(LuceneError::illegal_argument(
                "min_version must be set when created_version_major is >= 7".to_string(),
            ));
        }

        Ok(Self {
            created_version_major,
            min_version,
            sort,
            has_blocks,
        })
    }
}
