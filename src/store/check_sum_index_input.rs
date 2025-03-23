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
use crate::store::index_input::IndexInput;
use crate::util::error::lucene_error::{LuceneError, Result};

const SKIP_BUFFER_SIZE: i32 = 1024;
/// An extension of [`IndexInput`] that computes a checksum as it reads data.
/// Callers can retrieve the checksum using the `get_checksum` method from the implemented trait.
pub trait ChecksumIndexInput: IndexInput {
    /// Returns the current checksum value.
    fn get_checksum(&mut self) -> i64;
    /// Inherits documentation from the parent implementation.
    ///
    /// # Note
    /// [`ChecksumIndexInput`] can only seek forward, and seeks are expensive because they require
    /// reading the bytes between the current position and the target position to update the checksum.
    fn seek(&mut self, pos: i64) -> Result<()> {
        let cur_fp = self.get_file_pointer();
        if pos < cur_fp {
            return Err(LuceneError::illegal_state(format!(
                "cannot seek backwards (pos= {}  getFilePointer()= {})",
                pos, cur_fp
            )));
        }
        self.skip_by_reading(pos - cur_fp)
    }
    /// Skips over `num_bytes` bytes.
    /// The behavior of this method is equivalent to reading the same number of bytes into a buffer
    /// and discarding its content.
    ///
    fn skip_by_reading(&mut self, num_bytes: i64) -> Result<()> {
        let mut skip_buffer = [0u8; SKIP_BUFFER_SIZE as usize];
        let mut skipped = 0;
        while skipped < num_bytes {
            debug_assert!((num_bytes - skipped) <= i32::MAX as i64);
            let step = SKIP_BUFFER_SIZE.min((num_bytes - skipped) as i32);
            self.read_bytes_with_buffer(&mut skip_buffer, 0, step, false)?;
            skipped += step as i64;
        }
        Ok(())
    }
}
