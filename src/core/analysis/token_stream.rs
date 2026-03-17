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
use crate::analysis::common::analysis_impl::core::whitespace_analyzer::WhitespaceAnalyzerTS;
use crate::core::analysis::dummy::dummy_token_stream::DummyTokenStream;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_attributes::packed_token_attribute_impl::PackedTokenAttributeImpl;
use crate::core::document::field::StringTokenStream;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;

pub trait TokenStream {
  fn increment_token(&mut self) -> Result<bool> {
    unreachable!("must be implemented by sub");
  }
  fn end(&mut self) -> Result<()>;
  fn default_end(&mut self) -> Result<()> {
    self.get_attribute_source_mut().end_attributes();
    Ok(())
  }
  fn reset(&mut self) -> Result<()> {
    Ok(())
  }
  fn default_reset(&mut self) -> Result<()> {
    Ok(())
  }
  fn close(&mut self) -> Result<()> {
    Ok(())
  }
  fn get_attribute_source(&self) -> &Attributes;
  fn get_attribute_source_mut(&mut self) -> &mut Attributes;
  fn set_reader(&mut self, _input: ReaderEnum) -> Result<()> {
    Ok(())
  }
  fn set_reader_test_point(&mut self) {}
}

pub struct TokenStreamBase {
  pub(crate) att: Attributes,
}
impl TokenStreamBase {
  pub fn new(att: Attributes) -> Self {
    Self { att }
  }
}

pub fn default_attribute() -> Attributes {
  Attributes::PackedToken(PackedTokenAttributeImpl::new())
}
macro_rules! either_token_stream {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> TokenStream for $name<$( $T ),+>
        where
            $( $T: TokenStream ),+
        {
            #[inline]
            fn increment_token(&mut self) -> Result<bool> {
                match self { $( Self::$Variant(inner) => inner.increment_token(), )+ }
            }

            #[inline]
            fn end(&mut self) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.end(), )+ }
            }

            #[inline]
            fn default_end(&mut self) -> Result<()> {
                match self { $( Self::$Variant(inner) => TokenStream::default_end(inner), )+ }
            }

            #[inline]
            fn reset(&mut self) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.reset(), )+ }
            }

            #[inline]
            fn default_reset(&mut self) -> Result<()> {
                match self { $( Self::$Variant(inner) => TokenStream::default_reset(inner), )+ }
            }

            #[inline]
            fn close(&mut self) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.close(), )+ }
            }

            #[inline]
            fn get_attribute_source(&self) -> &Attributes {
                match self { $( Self::$Variant(inner) => inner.get_attribute_source(), )+ }
            }

            #[inline]
            fn get_attribute_source_mut(&mut self) -> &mut Attributes {
                match self { $( Self::$Variant(inner) => inner.get_attribute_source_mut(), )+ }
            }

            #[inline]
            fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
                match self { $( Self::$Variant(inner) => inner.set_reader(input), )+ }
            }

            #[inline]
            fn set_reader_test_point(&mut self) {
                match self { $( Self::$Variant(inner) => inner.set_reader_test_point(), )+ }
            }
        }
    };
}
either_token_stream!(pub TokenStreamEnum { Whitespace: A, Dummy: B });
either_token_stream!(pub TokenStreamEnum2 { A: A, B: B });

pub type InnerTokenStreams = TokenStreamEnum<WhitespaceAnalyzerTS, DummyTokenStream>;

impl<T> TokenStream for &mut T
where
  T: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    (**self).increment_token()
  }

  fn end(&mut self) -> Result<()> {
    (**self).end()
  }

  fn default_end(&mut self) -> Result<()> {
    (**self).default_end()
  }

  fn reset(&mut self) -> Result<()> {
    (**self).reset()
  }

  fn default_reset(&mut self) -> Result<()> {
    (**self).default_reset()
  }

  fn close(&mut self) -> Result<()> {
    (**self).close()
  }

  fn get_attribute_source(&self) -> &Attributes {
    (**self).get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    (**self).get_attribute_source_mut()
  }

  fn set_reader(&mut self, input: ReaderEnum) -> Result<()> {
    (**self).set_reader(input)
  }

  fn set_reader_test_point(&mut self) {
    (**self).set_reader_test_point()
  }
}

impl_from_for_enum!(
    TokenStreams,
    WhitespaceAnalyzerTS=> Whitespace,
    StringTokenStream=> StringField,
    crate::core::analysis::analyzer::StringTokenStream=> String,
    DummyTokenStream=> Dummy,
);
pub enum TokenStreams {
  Whitespace(WhitespaceAnalyzerTS),
  StringField(StringTokenStream),
  String(crate::core::analysis::analyzer::StringTokenStream),
  Dummy(DummyTokenStream),
}
impl TokenStream for TokenStreams {
  fn increment_token(&mut self) -> Result<bool> {
    match self {
      TokenStreams::Whitespace(v) => v.increment_token(),
      TokenStreams::StringField(v) => v.increment_token(),
      TokenStreams::String(v) => v.increment_token(),
      TokenStreams::Dummy(v) => v.increment_token(),
    }
  }

  fn end(&mut self) -> Result<()> {
    match self {
      TokenStreams::Whitespace(v) => v.end(),
      TokenStreams::StringField(v) => v.end(),
      TokenStreams::String(v) => v.end(),
      TokenStreams::Dummy(v) => v.end(),
    }
  }

  fn default_end(&mut self) -> Result<()> {
    match self {
      TokenStreams::Whitespace(v) => v.default_end(),
      TokenStreams::StringField(v) => v.default_end(),
      TokenStreams::String(v) => v.default_end(),
      TokenStreams::Dummy(v) => v.default_end(),
    }
  }

  fn reset(&mut self) -> Result<()> {
    match self {
      TokenStreams::Whitespace(v) => v.reset(),
      TokenStreams::StringField(v) => v.reset(),
      TokenStreams::String(v) => v.reset(),
      TokenStreams::Dummy(v) => v.reset(),
    }
  }

  fn default_reset(&mut self) -> Result<()> {
    match self {
      TokenStreams::Whitespace(v) => v.default_reset(),
      TokenStreams::StringField(v) => v.default_reset(),
      TokenStreams::String(v) => v.default_reset(),
      TokenStreams::Dummy(v) => v.default_reset(),
    }
  }

  fn close(&mut self) -> Result<()> {
    match self {
      TokenStreams::Whitespace(v) => v.close(),
      TokenStreams::StringField(v) => v.close(),
      TokenStreams::String(v) => v.close(),
      TokenStreams::Dummy(v) => v.close(),
    }
  }

  fn get_attribute_source(&self) -> &Attributes {
    match self {
      TokenStreams::Whitespace(v) => v.get_attribute_source(),
      TokenStreams::StringField(v) => v.get_attribute_source(),
      TokenStreams::String(v) => v.get_attribute_source(),
      TokenStreams::Dummy(v) => v.get_attribute_source(),
    }
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    match self {
      TokenStreams::Whitespace(v) => v.get_attribute_source_mut(),
      TokenStreams::StringField(v) => v.get_attribute_source_mut(),
      TokenStreams::String(v) => v.get_attribute_source_mut(),
      TokenStreams::Dummy(v) => v.get_attribute_source_mut(),
    }
  }

  fn set_reader(&mut self, _input: ReaderEnum) -> Result<()> {
    match self {
      TokenStreams::Whitespace(v) => v.set_reader(_input),
      TokenStreams::StringField(v) => v.set_reader(_input),
      TokenStreams::String(v) => v.set_reader(_input),
      TokenStreams::Dummy(v) => v.set_reader(_input),
    }
  }

  fn set_reader_test_point(&mut self) {
    match self {
      TokenStreams::Whitespace(v) => v.set_reader_test_point(),
      TokenStreams::StringField(v) => v.set_reader_test_point(),
      TokenStreams::String(v) => v.set_reader_test_point(),
      TokenStreams::Dummy(v) => v.set_reader_test_point(),
    }
  }
}
