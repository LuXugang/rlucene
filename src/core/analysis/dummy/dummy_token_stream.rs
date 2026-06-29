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
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::close::Closeable;

#[derive(Debug)]
pub struct DummyTokenStream;

impl Closeable for DummyTokenStream {
  fn close(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }
}

impl TokenStream for DummyTokenStream {
  fn increment_token(&mut self) -> crate::core::util::error::lucene_error::Result<bool> {
    dummy_unreachable!()
  }

  fn end(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }

  fn default_end(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }

  fn reset(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }

  fn default_reset(&mut self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }

  fn get_attribute_source(&self) -> &Attributes {
    dummy_unreachable!()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    dummy_unreachable!()
  }
}
