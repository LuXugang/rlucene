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
use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::index::segment_commit_info::SegmentCommitInfo;

use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;

pub struct SimpleTextLiveDocsFormat;
impl LiveDocsFormat for SimpleTextLiveDocsFormat {
    fn read_live_docs<D, B>(
        &self,
        dir: &D,
        info: &SegmentCommitInfo<D>,
        context: &IOContext,
    ) -> Result<B, LuceneError>
    where
        D: Directory,
        B: Bits,
    {
        todo!()
    }

    fn write_live_docs<D, B>(
        &self,
        bits: &B,
        dir: &mut D,
        info: &SegmentCommitInfo<D>,
        new_del_count: i32,
        context: &IOContext,
    ) -> Result<(), LuceneError>
    where
        D: Directory,
        B: Bits,
    {
        todo!()
    }

    fn files<D>(
        &self,
        info: &SegmentCommitInfo<D>,
        files: &mut HashSet<String>,
    ) -> Result<(), LuceneError>
    where
        D: Directory,
    {
        todo!()
    }
}
