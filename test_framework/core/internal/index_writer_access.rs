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
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

/// Access to [`IndexWriter`] internals exposed to the test framework.
///
/// This API is internal.
pub(crate) trait IndexWriterAccess {
  fn seg_string<D>(&self, iw: &IndexWriter<D>) -> Result<String>
  where
    D: Directory;

  fn get_segment_count<D>(&self, iw: &IndexWriter<D>) -> usize
  where
    D: Directory;

  fn is_closed<D>(&self, iw: &IndexWriter<D>) -> bool
  where
    D: Directory;

  fn get_reader<D>(
    &self,
    iw: &Arc<IndexWriter<D>>,
    apply_deletions: bool,
    write_all_deletes: bool,
  ) -> Result<StandardDirectoryReader<D>>
  where
    D: Directory + 'static;

  fn get_doc_writer_thread_pool_size<D>(&self, iw: &IndexWriter<D>) -> usize
  where
    D: Directory;

  fn is_deleter_closed<D>(&self, iw: &IndexWriter<D>) -> Result<bool>
  where
    D: Directory;

  fn newest_segment<D>(&self, iw: &IndexWriter<D>) -> Option<SegmentCommitInfo<D>>
  where
    D: Directory;
}
