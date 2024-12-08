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
use crate::store::{DataInput, ReadAdvice};
use crate::util::error::data_io_error_enum::DataIOError;

/**
 * Abstract base class for input from a file in a `Directory`. A random-access input stream.
 * Used for all Lucene index input operations.
 *
 * `IndexInput` may only be used from one thread, because it is not thread safe (it keeps
 * internal state like file position). To allow multithreaded use, every `IndexInput` instance
 * must be cloned before it is used in another thread. Subclasses must therefore implement
 * `clone()`, returning a new `IndexInput` which operates on the same underlying resource, but
 * positioned independently.
 *
 */
pub trait IndexInput: DataInput + Clone {
    /// Returns the current position in this file, where the next read will occur.
    ///
    /// # See Also
    /// [`seek`](IndexInput::seek)
    fn get_file_pointer(&self) -> u64;

    /// Sets the current position in this file, where the next read will occur.
    /// If this position is beyond the end of the file, it will return an `EOFError`,
    /// and the stream will be in an undetermined state.
    ///
    /// # See Also
    /// [`get_file_pointer`](IndexInput::get_file_pointer)
    fn seek(&mut self, pos: u64) -> Result<(), DataIOError>;
    /// Inherits documentation from the parent implementation.
    ///
    /// # Behavior
    /// This is functionally equivalent to seeking to `get_file_pointer() + num_bytes`.
    ///
    /// # See Also
    /// [`get_file_pointer`](IndexInput::get_file_pointer)
    ///
    /// [`seek`](IndexInput::seek)
    fn skip_bytes(&mut self, num_bytes: u64) -> Result<(), DataIOError> {
        let skip_to = self.get_file_pointer() + num_bytes;
        self.seek(skip_to)?;
        Ok(())
    }
    /// The number of bytes in the file.
    fn length(&self) -> u64;

    /// Creates a slice of this index input, with the given description, offset, and length.
    /// The slice is positioned at the beginning.
    fn slice(
        &self,
        slice_description: &str,
        offset: u64,
        length: u64,
    ) -> Result<impl IndexInput, DataIOError>;
    /// Creates a slice with a specific [`ReadAdvice`]. This is typically used by
    /// [`CompoundFormat`] implementations to honor
    /// the [`ReadAdvice`] of each file within the compound file.
    ///
    /// # Note
    /// It is only legal to call this method if this `IndexInput` has been opened with
    /// `ReadAdvice::NORMAL`. However, this method accepts any `ReadAdvice` value except `None` for
    /// the slice.
    ///
    /// The default implementation delegates to [`slice`](IndexInput::slice) and ignores the
    /// `ReadAdvice`.
    fn slice_with_read_advice(
        &self,
        description: &str,
        offset: u64,
        length: u64,
        _read_advice: ReadAdvice,
    ) -> Result<impl IndexInput, DataIOError> {
        self.default_slice_with_read_advice(description, offset, length, _read_advice)
    }
    fn default_slice_with_read_advice(
        &self,
        description: &str,
        offset: u64,
        length: u64,
        read_advice: ReadAdvice,
    ) -> Result<impl IndexInput, DataIOError> {
        self.slice(description, offset, length)
    }
    /// Subclasses call this to get the String for resourceDescription of a slice of this `IndexInput`.
    fn get_full_slice_description(&self, slice_description: &str) -> String {
        format!(" [slice= {} ", slice_description)
    }

    /// Creates a random-access slice of this index input, with the given offset and length.
    ///
    /// # Note
    /// The default implementation calls [`slice`](IndexInput::slice), and it doesn't support random access.
    /// It implements absolute reads as seek+read.
    fn random_access_slice(
        &self,
        offset: u64,
        length: u64,
    ) -> Result<impl RandomAccessInput, DataIOError> {
        self.default_random_access_slice(offset, length)
    }
    fn default_random_access_slice(
        &self,
        offset: u64,
        length: u64,
    ) -> Result<impl RandomAccessInput, DataIOError> {
        self.get_random_access_slice(offset, length)
    }
    /// Optional method: Gives a hint to this input that some bytes will be read in the near future.
    /// `IndexInput` implementations may take advantage of this hint to start fetching pages of data
    /// immediately from storage.
    ///
    /// # Arguments
    /// * `offset` - The starting offset.
    /// * `length` - The number of bytes to prefetch.
    ///
    /// # Note
    /// The default implementation is a no-op.
    fn prefetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        self.default_prefetch(pos, len)
    }

    fn default_prefetch(&mut self, pos: u64, len: u64) -> Result<(), DataIOError> {
        Ok(())
    }
    /**
     * whether `IndexInput` implementation supports random access
     */
    fn is_random_access(&self) -> bool;

    fn get_random_access_slice(
        &self,
        offset: u64,
        length: u64,
    ) -> Result<impl RandomAccessInput, DataIOError>;
}
