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
pub trait BufferOps<T: Copy> {
    /// Copies data from the source slice into the buffer at a specified offset.
    ///
    /// # Arguments
    /// * `src` - The source slice to copy from.
    /// * `offset` - The offset in the buffer to start writing to.
    ///
    /// # Panics    
    ///  if the copy would go out of bounds.
    fn copy_from(&mut self, src: &[T], offset: usize);

    /// Reads data from the buffer into the destination slice.
    ///
    /// # Arguments
    /// * `dest` - The destination slice to copy into.
    /// * `offset` - The offset in the buffer to start reading from.
    ///
    /// # Panics
    /// if the read would go out of bounds.
    fn copy_to(&self, dest: &mut [T], offset: usize);
}

impl<T: Copy> BufferOps<T> for Vec<T> {
    fn copy_from(&mut self, src: &[T], offset: usize) {
        debug_assert!(
            offset + src.len() <= self.len(),
            "Copy out of bounds: offset={}, src_len={}, buffer_len={}",
            offset,
            src.len(),
            self.len()
        );

        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), self.as_mut_ptr().add(offset), src.len());
        }
    }

    fn copy_to(&self, dest: &mut [T], offset: usize) {
        debug_assert!(
            offset + dest.len() <= self.len(),
            "Read out of bounds: offset={}, dest_len={}, buffer_len={}",
            offset,
            dest.len(),
            self.len()
        );

        unsafe {
            std::ptr::copy_nonoverlapping(self.as_ptr().add(offset), dest.as_mut_ptr(), dest.len());
        }
    }
}
