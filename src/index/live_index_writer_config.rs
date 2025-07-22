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
use crate::analysis::analyzer::Analyzer;
use crate::codecs::Codec;
use crate::index::flush_policy::FlushPolicy;
use crate::index::sort::Sort;
use crate::search::similarities::similarities::Similarity;
use crate::util::info_stream::InfoStreamLock;

pub trait LiveIndexWriterConfig {
    type Analyzer: Analyzer;
    fn get_analyzer(&self) -> &Self::Analyzer;

    type Similarity: Similarity;
    fn get_similarity(&self) -> &Self::Similarity;

    type Codec: Codec;
    fn get_codec(&self) -> &Self::Codec;

    fn get_index_sort(&self) -> Option<Sort>;

    fn get_use_compound_file(&self) -> bool;

    fn get_soft_deletes_field(&self) -> Option<&str>;

    fn get_info_stream(&self) -> InfoStreamLock;

    fn get_parent_field(&self) -> Option<&str>;

    type FlushPolicy: FlushPolicy;
    fn get_flush_policy(&self) -> &Self::FlushPolicy;

    fn get_ram_buffer_size_mb(&self) -> f64;

    fn get_ram_per_thread_hard_limit_mb(&self) -> i32;

    fn get_max_buffered_docs(&self) -> i32;

    fn get_check_pending_flush_on_update(&self) -> bool;
}
