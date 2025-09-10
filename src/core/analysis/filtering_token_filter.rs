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
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Abstract base class for `TokenFilter`s that may remove tokens.
/// You must implement [`accept`](FilteringTokenFilter::accept) and return a boolean indicating whether the current token should be preserved.
/// [`increment_token`](TokenStream::increment_token) uses this method to decide if a token should be passed to the caller.
pub trait FilteringTokenFilter {
    /// Override this method and return if the current input token should be returned by #incrementToken.
    fn accept(&mut self) -> bool;
}

pub struct FilteringTokenFilterBase<T>
where
    T: TokenFilter,
{
    skipped_positions: i32,
    base: TokenFilterBase<T>,
}
impl<T> FilteringTokenFilterBase<T>
where
    T: TokenFilter,
{
    pub fn new(input: T) -> Self {
        Self {
            skipped_positions: 0,
            base: TokenFilterBase::new(input),
        }
    }
    pub fn increment_token_with(&mut self) -> Result<bool> {
        todo!()
    }
}

impl<T> TokenStream for FilteringTokenFilterBase<T>
where
    T: TokenFilter,
{
    fn end(&mut self) -> Result<()> {
        self.base.end()?;
        let att = self.base.input.get_attribute_source_mut();
        let pos = match att.get_position_increment() {
            Some(p) => p,
            None => {
                return Err(LuceneError::illegal_state(
                    "PositionIncrementAttribute is missing",
                ));
            },
        };
        att.set_position_increment(pos + self.skipped_positions)?;
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.base.reset()?;
        self.skipped_positions = 0;
        Ok(())
    }

    type AttributeSource = <T as TokenStream>::AttributeSource;

    fn get_attribute_source(&self) -> &Self::AttributeSource {
        self.base.input.get_attribute_source()
    }

    fn get_attribute_source_mut(&mut self) -> &mut Self::AttributeSource {
        self.base.input.get_attribute_source_mut()
    }
}

impl<T> TokenFilter for FilteringTokenFilterBase<T> where T: TokenFilter {}
