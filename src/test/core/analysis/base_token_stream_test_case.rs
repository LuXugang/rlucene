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
use crate::core::analysis::token_attributes::char_term_attribute::CharTermAttribute;
use crate::core::analysis::token_attributes::flags_attribute::FlagsAttribute;
use crate::core::analysis::token_attributes::keyword_attribute::KeywordAttribute;
use crate::core::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::core::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::core::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::core::analysis::token_attributes::position_length_attribute::PositionLengthAttribute;
use crate::core::analysis::token_attributes::type_attribute::TypeAttribute;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::index::BytesRef;
use crate::core::search::boost_attribute::BoostAttribute;
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;

pub trait BaseTokenStreamTestCase {
  #[allow(clippy::too_many_arguments)]
  fn assert_token_stream_contents<TS>(
    ts: &mut TS,
    output: &[&str],
    start_offsets: Option<&[i32]>,
    end_offsets: Option<&[i32]>,
    types: Option<&[&str]>,
    pos_increments: Option<&[i32]>,
    pos_lengths: Option<&[i32]>,
    final_offset: Option<i32>,
    final_pos_inc: Option<i32>,
    keyword_atts: Option<&[bool]>,
    graph_offsets_are_correct: bool,
    payloads: Option<&[Option<Vec<u8>>]>,
    flags: Option<&[i32]>,
    boost: Option<&[f32]>,
  ) -> Result<()>
  where
    TS: TokenStream,
  {
    let (
      offset_att,
      type_att,
      pos_incr_att,
      pos_length_att,
      keyword_att,
      payload_att,
      flags_att,
      boost_att,
    ) = {
      let attr = ts.get_attribute_source();
      let attribute_names = attr.get_attribute_name()?;

      if !output.is_empty() {
        assert!(attribute_names.contains("CharTermAttribute"));
        assert!(attribute_names.contains("TermToBytesRefAttribute"));
        // TODO IMPORTANT BytesRefBuilderTermAttributeImpl未实现
      }

      let mut offset_att = false;
      if start_offsets.is_some() || end_offsets.is_some() || final_offset.is_some() {
        assert!(attribute_names.contains("OffsetAttribute"));
        offset_att = true;
      }

      let mut type_att = false;
      if types.is_some() {
        assert!(attribute_names.contains("TypeAttribute"));
        type_att = true;
      }

      let mut pos_incr_att = false;
      if pos_increments.is_some() || final_pos_inc.is_some() {
        assert!(attribute_names.contains("PositionIncrementAttribute"));
        pos_incr_att = true;
      }

      let mut pos_length_att = false;
      if pos_lengths.is_some() {
        assert!(attribute_names.contains("PositionLengthAttribute"));
        pos_length_att = true;
      }

      let mut keyword_att = false;
      if keyword_atts.is_some() {
        assert!(attribute_names.contains("KeywordAttribute"));
        keyword_att = true;
      }

      let mut payload_att = false;
      if payloads.is_some() {
        assert!(attribute_names.contains("PayloadAttribute"));
        payload_att = true;
      }

      let mut flags_att = false;
      if flags.is_some() {
        assert!(attribute_names.contains("FlagsAttribute"));
        flags_att = true;
      }

      let mut boost_att = false;
      if boost.is_some() {
        assert!(attribute_names.contains("BoostAttribute"));
        boost_att = true;
      }

      (
        offset_att,
        type_att,
        pos_incr_att,
        pos_length_att,
        keyword_att,
        payload_att,
        flags_att,
        boost_att,
      )
    };

    let mut pos_to_start_offset: HashMap<i32, i32> = HashMap::new();
    let mut pos_to_end_offset: HashMap<i32, i32> = HashMap::new();

    ts.reset()?;
    let mut pos = -1;
    let mut last_start_offset = 0;

    for i in 0..output.len() {
      ts.get_attribute_source_mut().clear_attributes();
      {
        let attr = ts.get_attribute_source_mut();
        attr.set_empty().append_str(Some("bogusTerm"));

        if offset_att {
          attr.set_offset(14584724, 24683243)?;
        }
        if type_att {
          attr.set_type("bogusType");
        }
        if pos_incr_att {
          PositionIncrementAttribute::set_position_increment(attr, 45987657)?;
        }
        if pos_length_att {
          PositionLengthAttribute::set_position_length(attr, 45987653)?;
        }
        if keyword_att {
          KeywordAttribute::set_keyword(attr, (i & 1) == 0)?;
        }
        if payload_att {
          PayloadAttribute::set_payload(
            attr,
            BytesRef::from_bytes(vec![0x00, (-0x21i8) as u8, 0x12, (-0x43i8) as u8, 0x24]),
          )
        }
        if flags_att {
          FlagsAttribute::set_flags(attr, !0);
        }
        if boost_att {
          BoostAttribute::set_boost(attr, -1.0);
        }
      }
      {
        let attr = ts.get_attribute_source_mut();
        attr.get_and_reset_clear_called()?;
      }
      assert!(ts.increment_token()?, "token {} does not exist", i);
      let attr = ts.get_attribute_source_mut();
      assert!(
        attr.get_and_reset_clear_called()?,
        "clearAttributes() was not called correctly in TokenStream chain at token {}",
        i
      );

      assert_eq!(output[i], attr.to_string(), "term {}", i);

      if let Some(start_offsets) = start_offsets {
        assert_eq!(start_offsets[i], OffsetAttribute::start_offset(attr));
      }

      if let Some(end_offsets) = end_offsets {
        assert_eq!(end_offsets[i], OffsetAttribute::end_offset(attr));
      }
      if let Some(types) = types {
        assert_eq!(types[i], TypeAttribute::type_value(attr));
      }
      if let Some(pos_increments) = pos_increments {
        assert_eq!(
          pos_increments[i],
          PositionIncrementAttribute::get_position_increment(attr)
        );
      }
      if let Some(pos_lengths) = pos_lengths {
        assert_eq!(
          pos_lengths[i],
          PositionLengthAttribute::get_position_length(attr)
        );
      }
      if let Some(keyword_atts) = keyword_atts {
        assert_eq!(keyword_atts[i], KeywordAttribute::is_keyword(attr)?);
      }
      if let Some(flags) = flags {
        assert_eq!(flags[i], FlagsAttribute::get_flags(attr));
      }
      if let Some(boost) = boost {
        assert!((boost[i] - BoostAttribute::get_boost(attr)).abs() <= 0.001);
      }
      if let Some(payloads) = payloads
        && let Some(payload) = &payloads[i]
      {
        assert_eq!(
          &BytesRef::from_bytes(payload.clone()),
          PayloadAttribute::get_payload(attr)
        );
      }
      if pos_incr_att {
        if i == 0 {
          assert!(PositionIncrementAttribute::get_position_increment(attr) >= 1);
        } else {
          assert!(PositionIncrementAttribute::get_position_increment(attr) >= 0);
        }
      }
      if pos_length_att {
        assert!(PositionLengthAttribute::get_position_length(attr) >= 1);
      }

      if offset_att {
        let start_offset = OffsetAttribute::start_offset(attr);
        let end_offset = OffsetAttribute::end_offset(attr);

        if let Some(final_offset) = final_offset {
          assert!(start_offset <= final_offset);
          assert!(end_offset <= final_offset);
        }

        assert!(OffsetAttribute::start_offset(attr) >= last_start_offset);
        last_start_offset = OffsetAttribute::start_offset(attr);

        if graph_offsets_are_correct && pos_length_att && pos_incr_att {
          let pos_inc = PositionIncrementAttribute::get_position_increment(attr);
          pos += pos_inc;

          let pos_length = PositionLengthAttribute::get_position_length(attr);

          if let Some(expected_start_offset) = pos_to_start_offset.get(&pos) {
            assert_eq!(*expected_start_offset, start_offset);
          } else {
            pos_to_start_offset.insert(pos, start_offset);
          }

          let end_pos = pos + pos_length;

          if let Some(expected_end_offset) = pos_to_end_offset.get(&end_pos) {
            assert_eq!(*expected_end_offset, end_offset);
          } else {
            pos_to_end_offset.insert(end_pos, end_offset);
          }
        }
      }
    }

    if ts.increment_token()? {
      unreachable!("")
    }

    {
      let attr = ts.get_attribute_source_mut();
      if !output.is_empty() {
        attr.set_empty().append_str(Some("bogusTerm"));
      }
      if offset_att {
        attr.set_offset(14584724, 24683243)?;
      }
      if type_att {
        attr.set_type("bogusType");
      }
      if pos_incr_att {
        PositionIncrementAttribute::set_position_increment(attr, 45987657)?;
      }
      if pos_length_att {
        PositionLengthAttribute::set_position_length(attr, 45987653)?;
      }
      if keyword_att {
        KeywordAttribute::set_keyword(attr, true)?;
      }
      if payload_att {
        PayloadAttribute::set_payload(
          attr,
          BytesRef::from_bytes(vec![0x00, (-0x21i8) as u8, 0x12, (-0x43i8) as u8, 0x24]),
        );
      }
      if flags_att {
        FlagsAttribute::set_flags(attr, !0);
      }
      if boost_att {
        BoostAttribute::set_boost(attr, -1.0);
      }
      attr.get_and_reset_clear_called()?;
    }

    ts.end()?;
    assert!(ts.get_attribute_source_mut().get_and_reset_clear_called()?);

    if let Some(final_offset) = final_offset {
      assert_eq!(
        final_offset,
        OffsetAttribute::end_offset(ts.get_attribute_source())
      );
    }
    if offset_att {
      assert!(OffsetAttribute::end_offset(ts.get_attribute_source()) >= 0);
    }
    if let Some(final_pos_inc) = final_pos_inc {
      assert_eq!(
        final_pos_inc,
        PositionIncrementAttribute::get_position_increment(ts.get_attribute_source())
      );
    }

    ts.close()?;
    Ok(())
  }
}

pub trait CheckClearAttributesAttribute: AttributeImpl {
  fn get_and_reset_clear_called(&mut self) -> bool;
}
pub struct CheckClearAttributesAttributeImpl {
  clear_called: bool,
}
impl Default for CheckClearAttributesAttributeImpl {
  fn default() -> Self {
    Self::new()
  }
}

impl CheckClearAttributesAttributeImpl {
  pub fn new() -> Self {
    CheckClearAttributesAttributeImpl {
      clear_called: false,
    }
  }
}

impl Attribute for CheckClearAttributesAttributeImpl {}

impl AttributeImpl for CheckClearAttributesAttributeImpl {
  fn clear(&mut self) {
    self.clear_called = true;
  }

  type AttributeImpl = CheckClearAttributesAttributeImpl;

  fn copy_to(&self, other: &mut Self::AttributeImpl) {
    other.clear()
  }
}

impl Clone for CheckClearAttributesAttributeImpl {
  fn clone(&self) -> Self {
    Self {
      clear_called: self.clear_called,
    }
  }
}

impl CheckClearAttributesAttribute for CheckClearAttributesAttributeImpl {
  fn get_and_reset_clear_called(&mut self) -> bool {
    self.clear_called = false;
    self.clear_called
  }
}
