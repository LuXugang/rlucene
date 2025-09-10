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
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::analysis::tokenizer::Tokenizer;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::consumer::Consumer;
use crate::core::util::error::lucene_error::Result;

pub trait Analyzer {
    fn get_position_increment_gap(&self, _field_name: &str) -> i32 {
        0
    }
    fn get_offset_gap(&self, _field_name: &str) -> i32 {
        1
    }
}

pub struct TokenStreamComponents<T, TS>
where
    T: Tokenizer,
    TS: TokenStream,
{
    source: Option<ConsumerEnum<T>>,
    sink: TS,
}
impl<T, TS> TokenStreamComponents<T, TS>
where
    T: Tokenizer,
    TS: TokenStream,
{
    pub fn new(source: Option<ConsumerEnum<T>>, sink: TS) -> Self {
        Self { source, sink }
    }
    pub fn with_tokenizer(tokenizer: Option<T>, sink: TS) -> Self {
        let source = tokenizer.map(|t| ConsumerEnum::TokenizerConsumer(TokenizerConsumer::new(t)));
        Self { source, sink }
    }
    pub fn get_token_stream(&mut self) -> &mut TS {
        &mut self.sink
    }
    pub fn get_reader(&self) -> &ConsumerEnum<T> {
        self.source.as_ref().unwrap()
    }
}
pub enum ConsumerEnum<T>
where
    T: Tokenizer,
{
    TokenizerConsumer(TokenizerConsumer<T>),
}
impl<T> Consumer for ConsumerEnum<T>
where
    T: Tokenizer,
{
    type V = ReaderEnum;

    fn accept_mut(&mut self, item: Self::V) -> Result<()> {
        match self {
            ConsumerEnum::TokenizerConsumer(tc) => tc.accept_mut(item),
        }
    }

    fn accept(&self, item: Self::V) -> Result<()> {
        match self {
            ConsumerEnum::TokenizerConsumer(tc) => tc.accept(item),
        }
    }
}

pub struct TokenizerConsumer<T>
where
    T: Tokenizer,
{
    pub tokenizer: T,
}
impl<T: Tokenizer> TokenizerConsumer<T> {
    fn new(tokenizer: T) -> Self {
        Self { tokenizer }
    }
}
impl<T> Consumer for TokenizerConsumer<T>
where
    T: Tokenizer,
{
    type V = ReaderEnum;

    fn accept_mut(&mut self, item: Self::V) -> Result<()> {
        self.tokenizer.set_reader(item)
    }

    fn accept(&self, _item: Self::V) -> Result<()> {
        unimplemented!("")
    }
}

struct StringTokenStream {
    value: String,
    length: i32,
    used: bool,
    att: Attributes,
}
impl StringTokenStream {
    fn new(att: Attributes, value: &str, length: i32) -> Self {
        Self {
            value: value.to_string(),
            length,
            used: true,
            att,
        }
    }
}
impl TokenStream for StringTokenStream {
    fn increment_token(&mut self) -> Result<bool> {
        if self.used {
            return Ok(true);
        }
        // self.clear_attributes();
        self.att.append_str(Some(&self.value));
        self.att.set_offset(0, self.length)?;
        self.used = true;
        Ok(true)
    }

    fn end(&mut self) -> Result<()> {
        self.default_end()?;
        self.att.set_offset(self.length, self.length)
    }

    fn reset(&mut self) -> Result<()> {
        self.used = false;
        Ok(())
    }

    type AttributeSource = Attributes;

    fn get_attribute_source(&self) -> &Self::AttributeSource {
        &self.att
    }

    fn get_attribute_source_mut(&mut self) -> &mut Self::AttributeSource {
        &mut self.att
    }
}
