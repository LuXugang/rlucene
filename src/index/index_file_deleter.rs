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
use crate::index::index_commit::{index_commit_util, IndexCommit};
use crate::index::segment_infos::SegmentInfos;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Holds details for each commit point. This struct is also passed to the deletion policy. Note: This struct has a natural ordering that is inconsistent with equals.
pub(crate) struct CommitPoint<D> {
    pub(crate) files: Vec<String>,
    pub(crate) segments_file_name: String,
    pub(crate) deleted: bool,
    pub(crate) directory_orig: Arc<Mutex<D>>,
    pub(crate) generation: i64,
    pub(crate) user_data: HashMap<String, String>,
    pub(crate) segment_count: usize,
}
impl<D> CommitPoint<D>
where
    D: Directory,
{
    pub(crate) fn new(
        directory_orig: Arc<Mutex<D>>,
        segment_infos: &SegmentInfos<D>,
    ) -> Result<Self> {
        // TODO：是不是只要保存segment的ID就行,避免一些拷贝
        let user_data = segment_infos.get_user_data().clone();
        let segments_file_name = segment_infos
            .get_segments_file_name()
            .ok_or_else(|| LuceneError::illegal_state("segment_N file is none"))?;
        let generation = segment_infos.get_generation();
        let files = segment_infos.files(true)?.into_iter().collect();
        let segment_count = segment_infos.size();

        Ok(CommitPoint {
            files,
            segments_file_name,
            deleted: false,
            directory_orig,
            generation,
            user_data,
            segment_count,
        })
    }
}

impl<D> PartialEq for CommitPoint<D>
where
    D: Directory,
{
    fn eq(&self, other: &Self) -> bool {
        index_commit_util::is_same_commit(self, other)
    }
}

impl<D> Eq for CommitPoint<D> where D: Directory {}

impl<D> PartialOrd for CommitPoint<D>
where
    D: Directory,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        index_commit_util::cmp_commit(self, other)
    }
}

impl<D> Ord for CommitPoint<D>
where
    D: Directory,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<D> Display for CommitPoint<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IndexFileDeleter.CommitPoint({})",
            self.segments_file_name
        )
    }
}

impl<D> IndexCommit for CommitPoint<D>
where
    D: Directory,
{
    fn get_segments_file_name(&self) -> &str {
        &self.segments_file_name
    }

    fn get_file_names(&self) -> Result<&[String]> {
        Ok(self.files.as_slice())
    }

    type Directory = D;

    fn get_directory(&self) -> Arc<Mutex<Self::Directory>> {
        self.directory_orig.clone()
    }

    fn delete(&mut self) -> Result<()> {
        todo!()
    }

    fn is_deleted(&self) -> bool {
        self.deleted
    }

    fn get_segment_count(&self) -> usize {
        self.segment_count
    }

    fn get_generation(&self) -> i64 {
        self.generation
    }

    fn user_data(&self) -> &HashMap<String, String> {
        &self.user_data
    }
}
