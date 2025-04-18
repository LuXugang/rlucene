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
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use crate::util::math_util::MathUtil;

/// Reads skip lists with multiple levels.
///
/// See [`MultiLevelSkipListWriter`](crate::codecs::multi_level_skip_list_writer) for details on how multi‑level skip lists are encoded.
///
/// Implementors must provide the `read_skip_data(&mut self, level: i32, input: &mut I)`
/// method to define the actual format of the skip data.
#[allow(dead_code)]
pub struct MultiLevelSkipListReader<I>
where
    I: IndexInput,
{
    /// the maximum number of skip levels possible for this index
    pub(crate) max_number_of_skip_levels: i32,

    /// number of levels in this skip list
    pub(crate) number_of_skip_levels: i32,

    doc_count: i32,

    /// skipStream for each level.
    // TODO: if IndexInput impl Default , we could use Default for padding when we need take ownership in `#load_skip_levels`
    // then there no need wrap with `Option`
    skip_stream: Vec<Option<I>>,

    /// The start pointer of each skip level.
    skip_pointer: Vec<i64>,

    /// skipInterval of each level.
    skip_interval: Vec<i32>,

    /// Number of docs skipped per level. It's possible for some values to overflow a signed int, but
    /// this has been accounted for.
    num_skipped: Vec<i32>,

    /// Doc id of current skip entry per level.
    pub(crate) skip_doc: Vec<i32>,

    /// Doc id of last read skip entry with docId <= target.
    last_doc: i32,

    /// Child pointer of current skip entry per level.
    child_pointer: Vec<i64>,

    /// childPointer of last read skip entry with docId <= target.
    last_child_pointer: i64,

    skip_multiplier: i32,
}
impl<I: IndexInput> MultiLevelSkipListReader<I> {
    /// Creates a new `MultiLevelSkipListReader` with the given skip stream, maximum skip levels,
    /// base skip interval, and skip multiplier.
    pub fn new(
        first_skip_stream: I,
        max_skip_levels: usize,
        base_skip_interval: i32,
        skip_multiplier: i32,
    ) -> Self {
        let mut skip_stream = Vec::with_capacity(max_skip_levels);
        skip_stream.push(Some(first_skip_stream));
        skip_stream.resize_with(max_skip_levels, || unimplemented!());
        let mut skip_interval = vec![0; max_skip_levels];
        skip_interval[0] = base_skip_interval;
        for i in 1..max_skip_levels {
            skip_interval[i] = skip_interval[i - 1] * skip_multiplier;
        }

        Self {
            max_number_of_skip_levels: max_skip_levels as i32,
            number_of_skip_levels: 1,
            doc_count: 0,
            skip_stream,
            skip_pointer: vec![0; max_skip_levels],
            skip_interval,
            num_skipped: vec![0; max_skip_levels],
            skip_doc: vec![0; max_skip_levels],
            last_doc: 0,
            child_pointer: vec![0; max_skip_levels],
            last_child_pointer: 0,
            skip_multiplier,
        }
    }

    /// Returns the id of the doc to which the last call of [`skip_to`](Self::skip_to) has skipped.
    pub fn doc(&self) -> i32 {
        self.last_doc
    }

    /// Skips entries to the first beyond the current whose document number is
    /// greater than or equal to `target`.  
    /// Returns the current doc count.
    pub fn skip_to(
        &mut self,
        target: i32,
        base: &mut impl MultiLevelSkipListReaderBase,
    ) -> Result<i32> {
        // walk up the levels until highest level is found that has a skip
        // for this target
        let mut level = 0;
        while level < (self.number_of_skip_levels) - 1 && target > self.skip_doc[level as usize + 1]
        {
            level += 1;
        }

        while level >= 0 {
            let idx = level as usize;
            if target > self.skip_doc[idx] {
                if !self.load_next_skip(idx, base)? {
                    continue;
                }
            } else {
                // no more skips on this level, go down one level
                if level > 0 {
                    let lower = (level - 1) as usize;
                    let fp = self.skip_stream[lower].as_ref().unwrap().get_file_pointer();
                    if self.last_child_pointer > fp {
                        self.seek_child(lower)?;
                    }
                }
                level -= 1;
            }
        }
        Ok(self.num_skipped[0] - self.skip_interval[0] - 1)
    }
    fn load_next_skip(
        &mut self,
        level: usize,
        base: &mut impl MultiLevelSkipListReaderBase,
    ) -> Result<bool> {
        // we have to skip, the target document is greater than the current
        // skip list entry
        self.set_last_skip_data(level);

        self.num_skipped[level] = self.num_skipped[level].wrapping_add(self.skip_interval[level]);
        // numSkipped may overflow a signed int, so compare as unsigned.
        if (self.num_skipped[level] as u32) > (self.doc_count as u32) {
            // this skip list is exhausted
            self.skip_doc[level] = i32::MAX;
            if self.number_of_skip_levels > level as i32 {
                self.number_of_skip_levels = level as i32;
            }
            return Ok(false);
        }

        // read next skip data
        let delta = base.read_skip_data(level, self.skip_stream[level].as_mut().unwrap())?;
        self.skip_doc[level] = self.skip_doc[level].wrapping_add(delta);

        if level != 0 {
            let ptr = self.read_child_pointer(level)?;
            self.child_pointer[level] = ptr + self.skip_pointer[level - 1];
        }

        Ok(true)
    }

    /// Initializes the reader, for reuse on a new term.
    pub fn init(&mut self, skip_pointer: i64, df: i32) -> Result<()> {
        self.skip_pointer[0] = skip_pointer;
        self.doc_count = df;
        debug_assert!(
            skip_pointer >= 0 && skip_pointer <= self.skip_stream[0].as_ref().unwrap().length(),
            "invalid skip pointer: {}, length={}",
            skip_pointer,
            self.skip_stream[0].as_ref().unwrap().length()
        );
        self.skip_doc.fill(0);
        self.num_skipped.fill(0);
        self.child_pointer.fill(0);
        let levels = self.number_of_skip_levels as usize;
        for slot in self.skip_stream.iter_mut().take(levels).skip(1) {
            *slot = None;
        }
        self.load_skip_levels()
    }

    /// Loads the skip levels
    fn load_skip_levels(&mut self) -> Result<()> {
        if self.doc_count <= self.skip_interval[0] {
            self.number_of_skip_levels = 1;
        } else {
            self.number_of_skip_levels = 1 + MathUtil::log(
                (self.doc_count / self.skip_interval[0]) as i64,
                self.skip_multiplier,
            )?;
        }
        if self.number_of_skip_levels > self.max_number_of_skip_levels {
            self.number_of_skip_levels = self.max_number_of_skip_levels;
        }
        // take ownership to void borrow issue, return to self.skip_stream later
        let mut stream0 = self.skip_stream[0].take().unwrap();
        stream0.seek(self.skip_pointer[0])?;
        for i in (1..self.number_of_skip_levels as usize).rev() {
            // the length of the current level
            let length = self.read_level_length(&mut stream0)?;
            // the start pointer of the current level
            self.skip_pointer[i] = stream0.get_file_pointer();
            // clone this stream, it is already at the start of the current level
            self.skip_stream[i] = Some(stream0.try_clone()?);
            // move base stream beyond the current level
            stream0.seek(stream0.get_file_pointer() + length)?;
        }
        // use base stream for the lowest level
        self.skip_pointer[0] = stream0.get_file_pointer();
        // return to self.skip_stream
        self.skip_stream[0] = Some(stream0);
        Ok(())
    }
    /// read the length of the current level written via [`MultiLevelSkipListWriter::writeLevelLength`](crate::codecs::multi_level_skip_list_writer::MultiLevelSkipListWriter::write_level_length).
    ///
    /// Parameters:
    /// - `skipStream`: the IndexInput the length shall be read from
    ///
    /// Returns:
    /// - level length
    fn read_level_length(&mut self, skip_stream: &mut impl IndexInput) -> Result<i64> {
        skip_stream.read_vlong()
    }

    /// read the child pointer written via [`MultiLevelSkipListWriter::writeChildPointer(long, DataOutput)`](crate::codecs::multi_level_skip_list_writer::MultiLevelSkipListWriter::write_child_pointer).
    ///
    /// Parameters:
    /// - `skipStream`: the IndexInput the child pointer shall be read from
    ///
    /// Returns:
    /// - child pointer
    fn read_child_pointer(&mut self, skip_stream_level: usize) -> Result<i64> {
        self.skip_stream[skip_stream_level]
            .as_mut()
            .unwrap()
            .read_vlong()
    }
}
impl<I> MultiLevelSkipListReaderAbstract for MultiLevelSkipListReader<I>
where
    I: IndexInput,
{
    fn set_last_skip_data(&mut self, level: usize) {
        self.last_doc = self.skip_doc[level];
        self.last_child_pointer = self.child_pointer[level];
    }
    fn seek_child(&mut self, level: usize) -> Result<()> {
        let stream = self.skip_stream[level].as_mut().unwrap();
        stream.seek(self.last_child_pointer)?;
        self.num_skipped[level] = self.num_skipped[level + 1] - self.skip_interval[level + 1];
        self.skip_doc[level] = self.last_doc;
        if level > 0 {
            self.child_pointer[level] =
                self.read_child_pointer(level)? + self.skip_pointer[level - 1];
        }
        Ok(())
    }
}
#[allow(dead_code)]
pub(crate) trait MultiLevelSkipListReaderAbstract {
    /// Copies the values of the last read skip entry on this level.
    fn set_last_skip_data(&mut self, level: usize);
    /// Seeks the skip entry on the given level
    fn seek_child(&mut self, level: usize) -> Result<()>;
}
#[allow(dead_code)]
pub(crate) trait MultiLevelSkipListReaderBase {
    /// Subclasses must implement the actual skip data encoding in this method.
    ///
    /// Parameters:
    /// - `level`: the level skip data shall be read from  
    /// - `skipStream`: the skip stream to read from
    fn read_skip_data(&mut self, level: usize, skip_stream: &mut impl IndexInput) -> Result<i32>;
}
