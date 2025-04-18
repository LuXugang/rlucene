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
use crate::index::impact::Impact;

pub struct Lucene101PostingsReader;
pub mod lucene101_pr_util {
    use crate::codecs::lucene101::lucene101_postings_reader::MutableImpactList;
    use crate::index::impact::Impact;
    use crate::store::{ByteArrayDataInput, DataInput};
    use crate::util::error::lucene_error::Result;

    /// @see [`Lucene101PostingsWriter::writeVInt15`](crate::codecs::lucene101::lucene101_postings_writer::lucene101_pw_util::write_vint15)
    pub(crate) fn read_vint15(input: &mut impl DataInput) -> Result<i32> {
        let s = input.read_short()?;
        if s >= 0 {
            Ok(s as i32)
        } else {
            Ok((s as i32) & 0x7FFF | (input.read_vint()? << 15))
        }
    }

    /// @see [`Lucene101PostingsWriter::writeVLong15`](crate::codecs::lucene101::lucene101_postings_writer::lucene101_pw_util::write_vlong15)
    pub(crate) fn read_vlong15(input: &mut impl DataInput) -> Result<i64> {
        let s = input.read_short()?;
        if s >= 0 {
            Ok(s as i64)
        } else {
            Ok((s as i64) & 0x7FFF | (input.read_vlong()? << 15))
        }
    }
    pub(crate) fn read_impacts<'a>(
        input: &mut ByteArrayDataInput,
        reuse: &'a mut MutableImpactList,
    ) -> Result<&'a [Impact]> {
        let mut freq = 0;
        let mut norm = 0;
        let mut length = 0;

        while input.get_position() < input.length() {
            let freq_delta = input.read_vint()?;
            freq += 1 + (freq_delta >> 1);
            if (freq_delta & 1) != 0 {
                norm += 1 + input.read_zlong()?;
            } else {
                norm += 1;
            }
            let slot = &mut reuse.impacts[length];
            slot.freq = freq;
            slot.norm = norm;
            length += 1;
        }

        reuse.length = length;
        Ok(&reuse.impacts[..length])
    }
}

pub(crate) struct MutableImpactList {
    length: usize,
    impacts: Vec<Impact>,
}
impl MutableImpactList {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let mut impacts = Vec::with_capacity(capacity);
        impacts.resize_with(capacity, || Impact {
            freq: i32::MAX,
            norm: 1,
        });
        MutableImpactList { length: 0, impacts }
    }

    pub(crate) fn get(&self, index: usize) -> &Impact {
        &self.impacts[index]
    }

    pub(crate) fn size(&self) -> usize {
        self.length
    }
}
