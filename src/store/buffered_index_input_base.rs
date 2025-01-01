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
use crate::store::random_access_input::RandomAccessInput;
use crate::store::IndexInput;
use crate::util::error::runtime_error::RuntimeError;
use std::io::Cursor;

pub trait BufferedIndexInputBase: Clone {
    /// Expert: Implements seek functionality. Sets the current position in this file,
    /// where the next call to [`read_internal`](BufferedIndexInputBase::read_internal) will occur.
    ///
    /// # See Also
    /// [`read_internal`](BufferedIndexInputBase::read_internal)
    fn seek_internal(&mut self, pos: u64) -> Result<(), RuntimeError>;
    /// Expert: Implements buffer refill. Reads bytes from the current position in the input.
    ///
    /// # Arguments
    /// * `b` - The buffer to read bytes into.
    fn read_internal(
        &mut self,
        b: &mut Cursor<Vec<u8>>,
        len: u64,
        file_pointer: u64,
    ) -> Result<(), RuntimeError>;

    /// Creates a slice of this index input, with the given description, offset, and length.
    /// The slice is positioned at the beginning.
    fn slice(
        &self,
        slice_description: &str,
        offset: u64,
        length: u64,
    ) -> Result<impl IndexInput + RandomAccessInput, RuntimeError>;

    /// The number of bytes in the file.
    fn length(&self) -> u64;
}
