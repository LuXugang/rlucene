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
use crate::analysis::common::analysis_impl::core::whitespace_analyzer::WhitespaceAnalyzer;
use crate::core::analysis::reader::{Reader, ReaderEnum};
use crate::core::analysis::reusable_string_reader::ReusableStringReader;
use crate::core::analysis::standard::standard_analyzer::StandardAnalyzer;
use crate::core::analysis::token_stream::{AnalyzerTokenStreams, TokenStream, TokenStreams};
use crate::core::index::BytesRef;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::impl_from_for_enum;
#[cfg(test)]
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    pub static REUSE_STRATEGY: RefCell<Option<ReuseStrategyEnum>> = const { RefCell::new(None) };
}

pub trait Analyzer {
  fn create_components(&self, field: &str) -> Result<TokenStreamComponents>;
  /// Default reuse strategy is GlobalReuseStrategy
  fn init_reuse_strategy(&self) -> ReuseStrategyEnum {
    ReuseStrategyEnum::Global(Box::default())
  }
  type TokenStream<TS>: TokenStream
  where
    TS: TokenStream;
  fn normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
  where
    TS: TokenStream;
  fn default_normalize_from_ts<TS>(&self, _field_name: &str, in_: TS) -> Result<TS>
  where
    TS: TokenStream,
  {
    Ok(in_)
  }

  fn ensure_reuse_strategy<'a>(
    &'a self,
    slot: &'a mut Option<ReuseStrategyEnum>,
  ) -> &'a mut ReuseStrategyEnum {
    if slot.is_none() {
      *slot = Some(self.init_reuse_strategy());
    }
    slot.as_mut().unwrap()
  }
  fn token_stream<R>(&self, field_name: &str, input: R) -> Result<()>
  where
    R: Into<ReaderEnum>,
  {
    let reader = self.init_reader(field_name, input.into());
    REUSE_STRATEGY.with(move |reuse_strategy| {
      (|| -> Result<()> {
        let mut reuse_strategy = reuse_strategy.borrow_mut();
        let reuse_strategy = self.ensure_reuse_strategy(&mut reuse_strategy);

        let mut components = reuse_strategy.get_reusable_components(field_name)?;
        if components.is_none() {
          let v = self.create_components(field_name)?;
          reuse_strategy.set_reusable_components(field_name, v)?;
          components = reuse_strategy.get_reusable_components(field_name)?;
        }

        let components = components.as_mut().unwrap();
        components.set_reader(reader)?;
        Ok(())
      })()
    })?;
    Ok(())
  }

  fn normalize(&self, field_name: &str, text: &str) -> Result<BytesRef<Vec<u8>>> {
    let mut str_reader = ReusableStringReader::new();
    str_reader.set_value(text);
    let mut reader =
      self.init_reader_for_normalization(field_name, ReaderEnum::ReusedString(str_reader));

    let mut buf = ['\0'; 64];
    let mut filtered = String::new();
    loop {
      let len = buf.len();
      let read = reader.read_range(&mut buf, 0, len)?;
      if read == -1 {
        break;
      }
      for &ch in &buf[..read as usize] {
        filtered.push(ch);
      }
    }

    let att = self.attribute_factory(field_name);
    debug_assert!(text.len() <= i32::MAX as usize);
    let mut ts = self.normalize_from_ts(
      field_name,
      StringTokenStream::new(att, &filtered, text.len() as i32),
    )?;

    ts.reset()?;
    if !ts.increment_token()? {
      return Err(LuceneError::illegal_state(format!(
        "expected 1 token but got 0 for analyzer and input \"{}\"",
        text
      )));
    }
    let term_att = ts.get_attribute_source_mut();
    let term = match term_att.get_bytes_ref()? {
      Some(t) => BytesRef::deep_copy_of(&*t),
      None => {
        return Err(LuceneError::illegal_state(format!(
          "CharTermAttribute is missing for analyzer and input \"{}\"",
          text
        )));
      },
    };
    if ts.increment_token()? {
      return Err(LuceneError::illegal_state(format!(
        "expected 1 token but got more for analyzer and input \"{}\"",
        text
      )));
    }
    ts.end()?;
    Ok(term)
  }

  fn init_reader(&self, _filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
    reader
  }

  fn init_reader_for_normalization(&self, _filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
    reader
  }

  fn attribute_factory(&self, _field_name: &str) -> Attributes {
    Attributes::default()
  }
  fn get_position_increment_gap(&self, _field_name: &str) -> i32 {
    0
  }
  fn get_offset_gap(&self, _field_name: &str) -> i32 {
    1
  }
}
impl_from_for_enum!(
    AnalyzerEnum,
    WhitespaceAnalyzer=> Whitespace,
    StandardAnalyzer => Standard,
);
#[cfg(test)]
impl_from_for_enum!(
    AnalyzerEnum,
    MockAnalyzer=> Mock,
);

pub enum AnalyzerEnum {
  Whitespace(WhitespaceAnalyzer),
  Standard(StandardAnalyzer),
  #[cfg(test)]
  Mock(MockAnalyzer),
}
impl Default for AnalyzerEnum {
  fn default() -> Self {
    StandardAnalyzer::default().into()
  }
}
impl Analyzer for AnalyzerEnum {
  fn create_components(&self, field: &str) -> Result<TokenStreamComponents> {
    match self {
      AnalyzerEnum::Whitespace(v) => v.create_components(field),
      AnalyzerEnum::Standard(v) => v.create_components(field),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.create_components(field),
    }
  }

  fn init_reuse_strategy(&self) -> ReuseStrategyEnum {
    match self {
      AnalyzerEnum::Whitespace(v) => v.init_reuse_strategy(),
      AnalyzerEnum::Standard(v) => v.init_reuse_strategy(),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.init_reuse_strategy(),
    }
  }

  type TokenStream<TS>
    = TokenStreams<TS>
  where
    TS: TokenStream;

  fn normalize_from_ts<TS>(&self, field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
  where
    TS: TokenStream,
  {
    match self {
      AnalyzerEnum::Whitespace(v) => Ok(TokenStreams::Whitespace(
        v.normalize_from_ts(field_name, in_)?,
      )),
      AnalyzerEnum::Standard(v) => Ok(TokenStreams::Standard(
        v.normalize_from_ts(field_name, in_)?,
      )),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => Ok(TokenStreams::Mock(v.normalize_from_ts(field_name, in_)?)),
    }
  }

  fn ensure_reuse_strategy<'a>(
    &'a self,
    slot: &'a mut Option<ReuseStrategyEnum>,
  ) -> &'a mut ReuseStrategyEnum {
    match self {
      AnalyzerEnum::Whitespace(v) => v.ensure_reuse_strategy(slot),
      AnalyzerEnum::Standard(v) => v.ensure_reuse_strategy(slot),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.ensure_reuse_strategy(slot),
    }
  }

  fn token_stream<R>(&self, field_name: &str, input: R) -> Result<()>
  where
    R: Into<ReaderEnum>,
  {
    match self {
      AnalyzerEnum::Whitespace(v) => v.token_stream(field_name, input),
      AnalyzerEnum::Standard(v) => v.token_stream(field_name, input),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.token_stream(field_name, input),
    }
  }

  fn normalize(&self, field_name: &str, text: &str) -> Result<BytesRef<Vec<u8>>> {
    match self {
      AnalyzerEnum::Whitespace(v) => v.normalize(field_name, text),
      AnalyzerEnum::Standard(v) => v.normalize(field_name, text),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.normalize(field_name, text),
    }
  }

  fn init_reader(&self, _filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
    match self {
      AnalyzerEnum::Whitespace(v) => v.init_reader(_filed_name, reader),
      AnalyzerEnum::Standard(v) => v.init_reader(_filed_name, reader),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.init_reader(_filed_name, reader),
    }
  }

  fn init_reader_for_normalization(&self, _filed_name: &str, reader: ReaderEnum) -> ReaderEnum {
    match self {
      AnalyzerEnum::Whitespace(v) => v.init_reader_for_normalization(_filed_name, reader),
      AnalyzerEnum::Standard(v) => v.init_reader_for_normalization(_filed_name, reader),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.init_reader_for_normalization(_filed_name, reader),
    }
  }

  fn attribute_factory(&self, field_name: &str) -> Attributes {
    match self {
      AnalyzerEnum::Whitespace(v) => v.attribute_factory(field_name),
      AnalyzerEnum::Standard(v) => v.attribute_factory(field_name),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.attribute_factory(field_name),
    }
  }

  fn get_position_increment_gap(&self, field_name: &str) -> i32 {
    match self {
      AnalyzerEnum::Whitespace(v) => v.get_position_increment_gap(field_name),
      AnalyzerEnum::Standard(v) => v.get_position_increment_gap(field_name),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.get_position_increment_gap(field_name),
    }
  }

  fn get_offset_gap(&self, field_name: &str) -> i32 {
    match self {
      AnalyzerEnum::Whitespace(v) => v.get_offset_gap(field_name),
      AnalyzerEnum::Standard(v) => v.get_offset_gap(field_name),
      #[cfg(test)]
      AnalyzerEnum::Mock(v) => v.get_offset_gap(field_name),
    }
  }
}

pub enum ReuseStrategyEnum {
  Global(Box<GlobalReuseStrategy>),
  PerField(PerFieldReuseStrategy),
}
impl ReuseStrategy for ReuseStrategyEnum {
  fn get_reusable_components(
    &mut self,
    field_name: &str,
  ) -> Result<Option<&mut TokenStreamComponents>> {
    match self {
      ReuseStrategyEnum::Global(v) => v.get_reusable_components(field_name),
      ReuseStrategyEnum::PerField(v) => v.get_reusable_components(field_name),
    }
  }

  fn set_reusable_components(
    &mut self,
    field_name: &str,
    components: TokenStreamComponents,
  ) -> Result<()> {
    match self {
      ReuseStrategyEnum::Global(v) => v.set_reusable_components(field_name, components),
      ReuseStrategyEnum::PerField(v) => v.set_reusable_components(field_name, components),
    }
  }
}
pub struct AnalyzerBase<RS>
where
  RS: ReuseStrategy,
{
  reuse_strategy: RS,
}
impl AnalyzerBase<GlobalReuseStrategy> {
  pub(crate) fn new() -> Self {
    Self {
      reuse_strategy: GlobalReuseStrategy::default(),
    }
  }
}
impl<RS> AnalyzerBase<RS>
where
  RS: ReuseStrategy,
{
  fn with_rs(reuse_strategy: RS) -> Self {
    Self { reuse_strategy }
  }
}

pub trait ReuseStrategy {
  fn get_reusable_components(
    &mut self,
    field_name: &str,
  ) -> Result<Option<&mut TokenStreamComponents>>;
  fn set_reusable_components(
    &mut self,
    field_name: &str,
    components: TokenStreamComponents,
  ) -> Result<()>;
}
pub struct GlobalReuseStrategy {
  store_value: Option<TokenStreamComponents>,
  first: bool,
}
impl Default for GlobalReuseStrategy {
  fn default() -> Self {
    Self {
      store_value: None,
      first: true,
    }
  }
}
impl ReuseStrategy for GlobalReuseStrategy {
  fn get_reusable_components(
    &mut self,
    _field_name: &str,
  ) -> Result<Option<&mut TokenStreamComponents>> {
    match self.store_value {
      Some(ref mut v) => Ok(Some(v)),
      _ => Ok(None),
    }
  }

  fn set_reusable_components(
    &mut self,
    _field_name: &str,
    components: TokenStreamComponents,
  ) -> Result<()> {
    if self.first {
      self.first = false;
      self.store_value = Some(components);
      return Ok(());
    }
    let v = self
      .store_value
      .as_mut()
      .ok_or_else(|| LuceneError::already_closed("this Analyzer is closed"))?;
    *v = components;
    Ok(())
  }
}
#[derive(Default)]
pub struct PerFieldReuseStrategy {
  store_value: Option<HashMap<String, TokenStreamComponents>>,
}
impl ReuseStrategy for PerFieldReuseStrategy {
  fn get_reusable_components(
    &mut self,
    field_name: &str,
  ) -> Result<Option<&mut TokenStreamComponents>> {
    match self.store_value {
      Some(ref mut v) => Ok(v.get_mut(field_name)),
      _ => Ok(None),
    }
  }

  fn set_reusable_components(
    &mut self,
    field_name: &str,
    components: TokenStreamComponents,
  ) -> Result<()> {
    match self.store_value {
      Some(ref mut v) => {
        let _ = v.insert(field_name.to_string(), components);
        Ok(())
      },
      None => Err(LuceneError::already_closed("this Analyzer is closed")),
    }
  }
}

pub struct TokenStreamComponents {
  sink: AnalyzerTokenStreams,
  max_token_length: Option<usize>,
}
impl TokenStreamComponents {
  pub fn new<T>(sink: T, max_token_length: Option<usize>) -> Self
  where
    T: Into<AnalyzerTokenStreams>,
  {
    Self {
      sink: sink.into(),
      max_token_length,
    }
  }
  fn set_reader(&mut self, reader: ReaderEnum) -> Result<()> {
    match self.sink {
      AnalyzerTokenStreams::Standard(ref mut ts) => {
        let src = &mut ts.base.input.token_filter_base.input;
        src.set_reader(reader)?;
        let max_token_length = self
          .max_token_length
          .ok_or_else(|| LuceneError::illegal_state("max_token_length is not set"))?;
        src.set_max_token_length(max_token_length)?;
      },
      AnalyzerTokenStreams::Whitespace(ref mut ts) => {
        ts.set_reader(reader)?;
      },
      _ => return Err(LuceneError::unsupported_operation("")),
    }
    Ok(())
  }
  pub fn get_token_stream(&mut self) -> &mut AnalyzerTokenStreams {
    &mut self.sink
  }
}

pub struct StringTokenStream {
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

impl Drop for StringTokenStream {
  fn drop(&mut self) {
    self.close().expect("should not fail");
  }
}

impl TokenStream for StringTokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    if self.used {
      return Ok(false);
    }
    self.att.clear_attributes();
    self.att.append_str(Some(&self.value))?;
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

  fn get_attribute_source(&self) -> &Attributes {
    &self.att
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    &mut self.att
  }
}
