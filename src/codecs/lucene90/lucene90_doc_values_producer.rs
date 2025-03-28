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
use crate::codecs::lucene90_doc_values_format::{
    Lucene90DocValuesFormat, SKIP_INDEX_JUMP_LENGTH_PER_LEVEL,
};
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_skipper::DocValuesSkipper;
use crate::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::store::{DataInput, IndexInput};

#[derive(Debug, Clone, Copy)]
struct DocValuesSkipperEntry {
    pub offset: i64,
    pub length: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub doc_count: i32,
    pub max_doc_id: i32,
}

pub struct DocValuesSkipperImpl<I>
where
    I: IndexInput,
{
    min_doc_id: [i32; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    max_doc_id: [i32; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    min_value: [i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    max_value: [i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    doc_count: [i32; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    levels: i32,
    input: I::Slice,
    entry: DocValuesSkipperEntry,
}
impl<I> DocValuesSkipperImpl<I>
where
    I: IndexInput,
{
    pub fn new(input: I::Slice, entry: DocValuesSkipperEntry) -> Self {
        Self {
            min_doc_id: [-1; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            max_doc_id: [-1; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            min_value: [0; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            max_value: [0; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            doc_count: [0; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            levels: 1,
            input,
            entry,
        }
    }
}
impl<I> DocValuesSkipper for DocValuesSkipperImpl<I>
where
    I: IndexInput,
{
    fn advance(&mut self, target: i32) -> crate::util::error::lucene_error::Result<()> {
        if target > self.entry.max_doc_id {
            // skipper is exhausted
            for i in 0..Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL {
                self.min_doc_id[i] = NO_MORE_DOCS;
                self.max_doc_id[i] = NO_MORE_DOCS;
            }
        } else {
            // find next interval
            debug_assert!(
                target > self.max_doc_id[0],
                "target must be bigger than current interval"
            );

            loop {
                self.levels = self.input.read_byte()? as i32;

                debug_assert!(
                    self.levels <= Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL as i32
                        && self.levels > 0,
                    "level out of range [{}]",
                    self.levels
                );

                let mut valid = true;

                // check if current interval is competitive or we can jump to the next position
                for level in (0..self.levels as usize).rev() {
                    let max_doc = self.input.read_int()?;
                    self.max_doc_id[level] = max_doc;
                    if max_doc < target {
                        IndexInput::skip_bytes(
                            &mut self.input,
                            SKIP_INDEX_JUMP_LENGTH_PER_LEVEL[level],
                        )?;
                        valid = false;
                        break;
                    }
                    self.min_doc_id[level] = self.input.read_int()?;
                    self.max_value[level] = self.input.read_long()?;
                    self.min_value[level] = self.input.read_long()?;
                    self.doc_count[level] = self.input.read_int()?;
                }

                if valid {
                    // adjust levels
                    while (self.levels as usize) < Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL
                        && self.max_doc_id[self.levels as usize] >= target
                    {
                        self.levels += 1;
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    fn num_levels(&self) -> i32 {
        self.levels
    }

    fn min_doc_id(&self, level: i32) -> i32 {
        self.min_doc_id[level as usize]
    }

    fn max_doc_id(&self, level: i32) -> i32 {
        self.max_doc_id[level as usize]
    }

    fn min_value(&self, level: i32) -> i64 {
        self.min_value[level as usize]
    }

    fn max_value(&self, level: i32) -> i64 {
        self.max_value[level as usize]
    }

    fn doc_count_level(&self, level: i32) -> i32 {
        self.doc_count[level as usize]
    }

    fn global_min_value(&self) -> i64 {
        self.entry.min_value
    }

    fn global_max_value(&self) -> i64 {
        self.entry.max_value
    }

    fn global_doc_count(&self) -> i32 {
        self.entry.doc_count
    }
}
