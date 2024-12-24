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
use crate::util::packed::{Decoder, Encoder};
// pub const PACKED_BULK_OPS: [BulkOperationPacked<T>; 64] = [];

trait BulkOperation: Decoder + Encoder {
    fn write_long(block: i64, blocks: &mut [u8], mut blocks_offset: usize) -> usize {
        for j in 1..=8 {
            blocks[blocks_offset] = ((block as u64) >> (64 - (j << 3))) as u8;
            blocks_offset += 1;
        }
        blocks_offset
    }
    /// For every number of bits per value, there is a minimum number of blocks (b) / values (v)
    /// you need to write in order to reach the next block boundary:
    ///
    /// - 16 bits per value -> b=2, v=1
    /// - 24 bits per value -> b=3, v=1
    /// - 50 bits per value -> b=25, v=4
    /// - 63 bits per value -> b=63, v=8
    ///
    /// A bulk read consists of copying `iterations * v` values that are contained in
    /// `iterations * b` blocks into a `Vec<i64>` (higher values of `iterations` are likely to yield
    /// a better throughput): this requires `iterations * (b + 8v)` bytes of memory.
    ///
    /// This method computes `iterations` as `ram_budget / (b + 8v)` (since an i64 is 8 bytes).
    ///
    /// # Arguments
    /// - `value_count`: The total number of values.
    /// - `ram_budget`: The available RAM budget in bytes.
    ///
    /// # Returns
    /// The number of iterations to perform.
    fn compute_iterations(&self, value_count: usize, ram_budget: usize) -> usize {
        let byte_value_count = Decoder::byte_block_count(self);
        let iterations = ram_budget / (byte_value_count + 8 * byte_value_count);
        if iterations == 0 {
            // At least 1 iteration is required
            1
        } else if (iterations - 1) * byte_value_count >= value_count {
            // Don't allocate for more than the size of the reader
            value_count.div_ceil(byte_value_count)
        } else {
            iterations
        }
    }
}
