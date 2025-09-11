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
use crate::core::analysis::reusable_string_reader::ReusableStringReader;
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::Attributes;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashMap;
use std::marker::PhantomData;

pub trait Analyzer {
    fn create_components<TS>(&self, field: &str) -> TokenStreamComponents<TS>
    where
        TS: TokenStream;

    fn get_position_increment_gap(&self, _field_name: &str) -> i32 {
        0
    }
    fn get_offset_gap(&self, _field_name: &str) -> i32 {
        1
    }
    fn get_analyzer_base<TS, RS>(&mut self) -> &mut AnalyzerBase<TS, RS>
    where
        TS: TokenStream,
        RS: ReuseStrategy<TS>;
    fn token_stream<'a, TS, RS>(&'a mut self, field_name: &str, text: &str) -> Result<&'a mut TS>
    where
        TS: TokenStream,
        RS: ReuseStrategy<TS> + 'a,
    {
        // We don’t reuse ReusableStringReader here like Java Lucene does.
        let mut str_reader = ReusableStringReader::new();
        str_reader.set_value(text);
        let r = self.init_reader(field_name, ReaderEnum::ReusedString(str_reader));
        let analyzer_base: &mut AnalyzerBase<TS, RS> = self.get_analyzer_base();
        let components = analyzer_base
            .reuse_strategy
            .get_reusable_components(field_name)?;
        if components.is_none() {
            let v: TokenStreamComponents<TS> = self.create_components(field_name);
            let analyzer_base: &mut AnalyzerBase<TS, RS> = self.get_analyzer_base();
            analyzer_base
                .reuse_strategy
                .get_reusable_components(field_name)?;
            analyzer_base
                .reuse_strategy
                .set_reusable_components(field_name, v)?;
        }
        let analyzer_base: &mut AnalyzerBase<TS, RS> = self.get_analyzer_base();
        let components = analyzer_base
            .reuse_strategy
            .get_reusable_components(field_name)?
            .unwrap();
        components.set_reader(r)?;
        Ok(components.get_token_stream())
    }

    fn init_reader(&self, _filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
        reader
    }
}
pub struct AnalyzerBase<TS, RS>
where
    TS: TokenStream,
    RS: ReuseStrategy<TS>,
{
    reuse_strategy: RS,
    _phantom: PhantomData<TS>,
}
impl<TS, RS> AnalyzerBase<TS, RS>
where
    TS: TokenStream,
    RS: ReuseStrategy<TS>,
{
}

pub trait ReuseStrategy<TS>
where
    TS: TokenStream,
{
    fn get_reusable_components(
        &mut self,
        field_name: &str,
    ) -> Result<Option<&mut TokenStreamComponents<TS>>>;
    fn set_reusable_components(
        &mut self,
        field_name: &str,
        components: TokenStreamComponents<TS>,
    ) -> Result<()>;
}
pub struct GlobalReuseStrategy<TS>
where
    TS: TokenStream,
{
    store_value: Option<StoredValue<TS>>,
}
impl<TS> ReuseStrategy<TS> for GlobalReuseStrategy<TS>
where
    TS: TokenStream,
{
    fn get_reusable_components(
        &mut self,
        _field_name: &str,
    ) -> Result<Option<&mut TokenStreamComponents<TS>>> {
        match self.store_value {
            Some(ref mut v) => match v {
                StoredValue::Global(components) => Ok(Some(components)),
                _ => Err(LuceneError::illegal_state("should not be here")),
            },
            _ => Ok(None),
        }
    }

    fn set_reusable_components(
        &mut self,
        _field_name: &str,
        components: TokenStreamComponents<TS>,
    ) -> Result<()> {
        match self.store_value {
            Some(ref mut v) => {
                *v = StoredValue::Global(components);
                Ok(())
            },
            None => Err(LuceneError::already_closed("this Analyzer is closed")),
        }
    }
}
pub struct PerFieldReuseStrategy<TS>
where
    TS: TokenStream,
{
    store_value: Option<StoredValue<TS>>,
}
impl<TS> ReuseStrategy<TS> for PerFieldReuseStrategy<TS>
where
    TS: TokenStream,
{
    fn get_reusable_components(
        &mut self,
        field_name: &str,
    ) -> Result<Option<&mut TokenStreamComponents<TS>>>
    where
        TS: TokenStream,
    {
        match self.store_value {
            Some(ref mut v) => match v {
                StoredValue::PerField(map) => Ok(map.get_mut(field_name)),
                _ => Err(LuceneError::illegal_state("should not be here")),
            },
            _ => Ok(None),
        }
    }

    fn set_reusable_components(
        &mut self,
        field_name: &str,
        components: TokenStreamComponents<TS>,
    ) -> Result<()> {
        match self.store_value {
            Some(ref mut v) => match v {
                StoredValue::PerField(map) => {
                    map.insert(field_name.to_string(), components);
                    Ok(())
                },
                _ => Err(LuceneError::illegal_state("should not be here")),
            },
            None => Err(LuceneError::already_closed("this Analyzer is closed")),
        }
    }
}

pub enum StoredValue<TS>
where
    TS: TokenStream,
{
    PerField(HashMap<String, TokenStreamComponents<TS>>),
    Global(TokenStreamComponents<TS>),
}

pub struct TokenStreamComponents<TS>
where
    TS: TokenStream,
{
    sink: Option<TS>,
}
impl<TS> TokenStreamComponents<TS>
where
    TS: TokenStream,
{
    pub fn new(sink: TS) -> Self {
        Self { sink: Some(sink) }
    }
    fn set_reader(&mut self, reader: ReaderEnum) -> Result<()> {
        self.sink.as_mut().unwrap().set_reader(reader)
    }
    pub fn get_token_stream(&mut self) -> &mut TS {
        self.sink.as_mut().unwrap()
    }
    pub fn take_token_stream(&mut self) -> Option<TS> {
        self.sink.take()
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
