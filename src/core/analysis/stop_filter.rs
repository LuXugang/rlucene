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
use crate::core::analysis::char_array_set::CharArraySet;
use crate::core::analysis::filtering_token_filter::{
  FilteringTokenFilter, FilteringTokenFilterBase,
};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;

pub struct StopFilter {
  // None is the immutable empty set used by StandardAnalyzer.
  stop_words: Option<Arc<CharArraySet>>,
}
impl StopFilter {
  pub fn make_stop_set<I>(stop_words: I) -> Arc<CharArraySet>
  where
    I: IntoIterator,
    I::Item: AsRef<str>,
  {
    Self::make_stop_set_with_ignore_case(stop_words, false)
  }

  pub fn make_stop_set_with_ignore_case<I>(stop_words: I, ignore_case: bool) -> Arc<CharArraySet>
  where
    I: IntoIterator,
    I::Item: AsRef<str>,
  {
    let mut stop_set = CharArraySet::new(ignore_case);
    stop_set.add_all(stop_words);
    Arc::new(stop_set)
  }

  pub fn new<T, S>(input: T, stop_words: S) -> FilteringTokenFilter<T, StopFilter>
  where
    T: TokenStream,
    S: Into<Arc<CharArraySet>>,
  {
    let v = StopFilter {
      stop_words: Some(stop_words.into()),
    };
    FilteringTokenFilter::new(input, v)
  }

  pub(crate) fn with_empty_stop_words<T>(input: T) -> FilteringTokenFilter<T, StopFilter>
  where
    T: TokenStream,
  {
    FilteringTokenFilter::new(input, Self { stop_words: None })
  }
}

impl FilteringTokenFilterBase for StopFilter {
  fn accept(&self, att: &Attributes) -> Result<bool> {
    let length = att.length()?;
    debug_assert!(length <= i32::MAX as usize);
    let token = &att.buffer()?[..length];
    Ok(
      self
        .stop_words
        .as_ref()
        .is_none_or(|stop_words| !stop_words.contains_key(token, 0, length)),
    )
  }
}
