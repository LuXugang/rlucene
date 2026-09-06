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
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Rust adaptation of Java's `instanceof IndexInput` and cast-based seek path.
///
/// Ordinary data inputs use the defaults: `false` from `is_index_input` and
/// unsupported positioning operations. Index inputs implement all three methods
/// by forwarding to `IndexInput`; wrappers preserve their underlying capability.
pub trait DataInputExt {
  /// Reports whether this input supports the [`IndexInput`](crate::core::store::index_input::IndexInput)-specific seek path.
  /// This avoids runtime downcasting through a trait object.
  fn is_index_input(&self) -> bool {
    false
  }
  fn seek_in_data_input(&mut self, _pos: usize) -> Result<()> {
    debug_assert!(self.is_index_input());
    Err(LuceneError::unsupported_operation(
      "Seek not implement for this DataInput",
    ))
  }
  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    debug_assert!(self.is_index_input());
    Err(LuceneError::unsupported_operation(
      "get_file_pointer not implement for this DataInput",
    ))
  }
}

impl<T: ?Sized + DataInputExt> DataInputExt for &mut T {
  fn is_index_input(&self) -> bool {
    (**self).is_index_input()
  }

  fn seek_in_data_input(&mut self, _pos: usize) -> Result<()> {
    (**self).seek_in_data_input(_pos)
  }

  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    (**self).get_file_pointer_in_data_input()
  }
}

impl<T: ?Sized + DataInputExt> DataInputExt for Box<T> {
  fn is_index_input(&self) -> bool {
    (**self).is_index_input()
  }

  fn seek_in_data_input(&mut self, _pos: usize) -> Result<()> {
    (**self).seek_in_data_input(_pos)
  }

  fn get_file_pointer_in_data_input(&self) -> Result<usize> {
    (**self).get_file_pointer_in_data_input()
  }
}

/// Implements the extension methods inside a `DataInputExt` implementation for
/// a type implementing `IndexInput`.
macro_rules! impl_index_input_ext {
  () => {
    fn is_index_input(&self) -> bool {
      true
    }

    fn seek_in_data_input(
      &mut self,
      pos: usize,
    ) -> $crate::core::util::error::lucene_error::Result<()> {
      $crate::core::store::index_input::IndexInput::seek(self, pos)
    }

    fn get_file_pointer_in_data_input(
      &self,
    ) -> $crate::core::util::error::lucene_error::Result<usize> {
      $crate::core::store::index_input::IndexInput::get_file_pointer(self)
    }
  };
}
pub(crate) use impl_index_input_ext;
