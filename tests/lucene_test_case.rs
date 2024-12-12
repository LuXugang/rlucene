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
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::store::flush_info::FlushInfo;
use rlucene::store::fs_directory::FSDirectory;
use rlucene::store::fs_directory_base::FSDirectoryBase;
use rlucene::store::lock_factory::LockFactory;
use rlucene::store::merge_info::MergeInfo;
use rlucene::store::nio_fs_directory::NIOFSDirectory;
use rlucene::store::{IOContext, NativeFSLock, NativeFSLockFactory};
use tempfile::TempDir;
use crate::util::test_error::TestError;

pub struct LuceneTestCase;

// TODO: When we have implemented multiple directories, we need to select one randomly. Currently, we choose NIOFSDirectory.
pub fn new_directory<'a, T, D>() -> Result<FSDirectory<'a, NativeFSLockFactory, NIOFSDirectory>, TestError>
where
    T: LockFactory,
    D: FSDirectoryBase,
{
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().clone();
    let sub_directory = NIOFSDirectory::new();
    Ok(FSDirectory::new(&path, sub_directory)?)
}

pub fn new_io_context(
    random: &mut StdRng,
    old_context: &IOContext,
) -> Result<IOContext, TestError> {
    let read_once = IOContext::read_once_io_context()?;
    if *old_context == read_once {
        // Don't modify the READONCE singleton
        return Ok(old_context.clone());
    }

    // Generate random parameters
    let random_num_docs: u32 = random.gen_range(0..4192);
    let size = random.gen_range(0..512) * random_num_docs as u64;

    if let Some(flush_info) = &old_context.flush_info {
        // Always return at least the estimatedSegmentSize of the incoming IOContext
        return Ok(IOContext::new_with_flush(FlushInfo::new(
            random_num_docs,
            size.max(flush_info.estimated_segment_size),
        ))?);
    } else if let Some(merge_info) = &old_context.merge_info {
        // Always return at least the estimatedMergeBytes of the incoming IOContext
        return Ok(IOContext::new_with_merge(MergeInfo::new(
            random_num_docs as u32,
            size.max(merge_info.estimated_merge_bytes),
            random.gen_bool(0.5), // Randomly decide if it's an external merge
            random.gen_range(1..=100),
        ))?);
    } else {
        // Make a totally random IOContext, except READONCE which has semantic implications
        let context_type = random.gen_range(0..3);
        match context_type {
            0 => Ok(IOContext::default_io_context()?),
            1 => Ok(IOContext::new_with_merge(MergeInfo::new(
                random_num_docs as u32,
                size,
                true,
                -1,
            ))?),
            2 => Ok(IOContext::new_with_flush(FlushInfo::new(
                random_num_docs,
                size,
            ))?),
            _ => Ok(IOContext::default_io_context()?),
        }
    }
}
