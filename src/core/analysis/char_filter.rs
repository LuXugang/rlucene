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
use crate::core::analysis::reader::{Reader, ReaderEnum};
use crate::core::util::error::lucene_error::Result;
/// `CharFilter` can be chained to filter a `Reader`.
/// They can be used as a `Reader` with additional offset correction.
/// [`Tokenizer`](crate::core::analysis::tokenizer::Tokenizer)s will automatically use [`correct_offset`](Self::correct_offset) if a `CharFilter` subclass is used.
pub trait CharFilter: Reader {
  /// The underlying character-input stream.
  fn get_reader(&self) -> &ReaderEnum;
  fn get_reader_mut(&mut self) -> &mut ReaderEnum;
  /// Closes the underlying input stream.
  fn close(&mut self) -> Result<()> {
    self.get_reader_mut().close()
  }
  /// override to correct the current offset.
  fn correct(&self, current_off: i32) -> i32;
  /// Chains the corrected offset through the input CharFilter(s).
  fn correct_offset(&self, current_off: i32) -> i32 {
    let corrected = self.correct(current_off);
    let base_reader = self.get_reader();
    base_reader.correct_offset(corrected)
  }
}
