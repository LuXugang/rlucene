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
use crate::codecs::Codec;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::LuceneError;
use std::collections::HashSet;

pub struct Lucene90LiveDocsFormat;

impl Lucene90LiveDocsFormat {
    pub fn new() -> Lucene90LiveDocsFormat {
        Lucene90LiveDocsFormat {}
    }
}

impl LiveDocsFormat for Lucene90LiveDocsFormat {
    fn read_live_docs<D: Directory, B: Bits, C: Codec>(
        &self,
        dir: &D,
        info: &SegmentCommitInfo<D, C>,
        context: &IOContext,
    ) -> Result<B, LuceneError> {
        todo!()
    }

    fn write_live_docs<D: Directory, B: Bits, C: Codec>(
        &self,
        bits: &B,
        dir: &mut D,
        info: &SegmentCommitInfo<D, C>,
        new_del_count: i32,
        context: &IOContext,
    ) -> Result<(), LuceneError> {
        todo!()
    }

    fn files<D: Directory, C: Codec>(
        &self,
        info: &SegmentCommitInfo<D, C>,
        files: &mut HashSet<String>,
    ) -> Result<(), LuceneError> {
        todo!()
    }
}
