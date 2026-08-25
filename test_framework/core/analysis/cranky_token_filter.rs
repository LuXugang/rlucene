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
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use rand::RngExt;
use rand::prelude::StdRng;
use std::io::Error;
use std::sync::Arc;

/// Throws IO errors from random [`TokenStream`] methods.
///
/// This can be used to simulate a buggy analyzer in `IndexWriter`, where we must delete the
/// document but not abort everything in the buffer.
pub struct CrankyTokenFilter<TS>
where
  TS: TokenStream,
{
  random: Arc<Mutex<StdRng>>,
  thing_to_do: i32,
  token_filter_base: TokenFilterBase<TS>,
}

impl<TS> CrankyTokenFilter<TS>
where
  TS: TokenStream,
{
  /// Creates a new `CrankyTokenFilter`.
  pub fn new(input: TS, random: Arc<Mutex<StdRng>>) -> Self {
    Self {
      random,
      thing_to_do: 0,
      token_filter_base: TokenFilterBase::new(input),
    }
  }
}

impl<TS> Closeable for CrankyTokenFilter<TS>
where
  TS: TokenStream,
{
  fn close(&mut self) -> Result<()> {
    self.token_filter_base.close()?;
    if self.thing_to_do == 3 && self.random.lock().random_bool(0.5) {
      return Err(LuceneError::io(Error::other(
        "Fake I/O error from TokenStream::close()",
      )));
    }
    Ok(())
  }
}

impl<TS> TokenStream for CrankyTokenFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    if self.thing_to_do == 0 && self.random.lock().random_bool(0.5) {
      return Err(LuceneError::io(Error::other(
        "Fake I/O error from TokenStream::increment_token()",
      )));
    }
    self.token_filter_base.input.increment_token()
  }

  fn end(&mut self) -> Result<()> {
    self.token_filter_base.end()?;
    if self.thing_to_do == 1 && self.random.lock().random_bool(0.5) {
      return Err(LuceneError::io(Error::other(
        "Fake I/O error from TokenStream::end()",
      )));
    }
    Ok(())
  }

  fn reset(&mut self) -> Result<()> {
    self.token_filter_base.reset()?;
    let mut random = self.random.lock();
    self.thing_to_do = random.random_range(0..100);
    if self.thing_to_do == 2 && random.random_bool(0.5) {
      return Err(LuceneError::io(Error::other(
        "Fake I/O error from TokenStream::reset()",
      )));
    }
    Ok(())
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    self.token_filter_base.set_reader(input)
  }

  fn set_reader_test_point(&mut self) -> Result<()> {
    self.token_filter_base.set_reader_test_point()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.token_filter_base.input.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.token_filter_base.input.get_attribute_source_mut()
  }
}

impl<TS> TokenFilter for CrankyTokenFilter<TS> where TS: TokenStream {}
