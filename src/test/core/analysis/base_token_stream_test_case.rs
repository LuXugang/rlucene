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
use crate::core::util::attribute::Attribute;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;

pub trait BaseTokenStreamTestCase {
  #[allow(clippy::too_many_arguments)]
  fn assert_token_stream_contents<TS>(
    _ts: &mut TS,
    _output: &[&str],
    _start_offsets: Option<&[i32]>,
    _end_offsets: Option<&[i32]>,
    _types: Option<&[&str]>,
    _pos_increments: Option<&[i32]>,
    _pos_lengths: Option<&[i32]>,
    _final_offset: Option<i32>,
    _final_pos_inc: Option<i32>,
    _keyword_atts: Option<&[bool]>,
    _graph_offsets_are_correct: bool,
    _payloads: Option<&[Option<Vec<u8>>]>,
    _flags: Option<&[i32]>,
    _boost: Option<&[f32]>,
  ) -> Result<()>
  where
    TS: TokenStream,
  {
    // let mut pos_to_start_offset: HashMap<i32, i32> = HashMap::new();
    // let mut pos_to_end_offset: HashMap<i32, i32> = HashMap::new();
    //
    // ts.reset()?;
    // let mut pos = -1;
    // let mut last_start_offset = 0;
    //
    // for i in 0..output.len() {
    //     ts.get_attribute_source_mut().clear_attributes();
    //     let attr = ts.get_attribute_source_mut();
    //     attr.set_empty().append_str(Some("bogusTerm"));
    //     attr.set_offset(14584724, 24683243)?;
    //     attr.set_type("bogusType");
    //     attr.set_position_increment(45987657)?;
    //     attr.set_position_length(45987653)?;
    //     attr.set_keyword((i & 1) == 0)?;
    //     attr.set_payload(Some(BytesRef::from_bytes(vec![
    //             0x00,
    //             (-0x21i8) as u8,
    //             0x12,
    //             (-0x43i8) as u8,
    //             0x24,
    //         ])))?;
    //     attr.set_flags(!0)?;
    //     attr.set_boost(-1.0)?;
    //
    //     check_clear_att.get_and_reset_clear_called()?;
    //     assert!(ts.increment_token()?, "token {} does not exist", i);
    //     assert!(
    //         check_clear_att.get_and_reset_clear_called()?,
    //         "clearAttributes() was not called correctly in TokenStream chain at token {}",
    //         i
    //     );
    //
    //     let term_att_ref = term_att.as_ref().unwrap();
    //     assert_eq!(output[i], term_att_ref.to_string()?, "term {}", i);
    //
    //     if let (Some(start_offsets), Some(offset_att)) = (start_offsets, offset_att.as_ref()) {
    //         assert_eq!(
    //             start_offsets[i],
    //             offset_att.start_offset()?,
    //             "startOffset {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(end_offsets), Some(offset_att)) = (end_offsets, offset_att.as_ref()) {
    //         assert_eq!(
    //             end_offsets[i],
    //             offset_att.end_offset()?,
    //             "endOffset {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(types), Some(type_att)) = (types, type_att.as_ref()) {
    //         assert_eq!(
    //             types[i],
    //             type_att.type_()?,
    //             "type {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(pos_increments), Some(pos_incr_att)) =
    //         (pos_increments, pos_incr_att.as_ref())
    //     {
    //         assert_eq!(
    //             pos_increments[i],
    //             pos_incr_att.get_position_increment()?,
    //             "posIncrement {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(pos_lengths), Some(pos_length_att)) = (pos_lengths, pos_length_att.as_ref()) {
    //         assert_eq!(
    //             pos_lengths[i],
    //             pos_length_att.get_position_length()?,
    //             "posLength {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(keyword_atts), Some(keyword_att)) = (keyword_atts, keyword_att.as_ref()) {
    //         assert_eq!(
    //             keyword_atts[i],
    //             keyword_att.is_keyword()?,
    //             "keywordAtt {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(flags), Some(flags_att)) = (flags, flags_att.as_ref()) {
    //         assert_eq!(
    //             flags[i],
    //             flags_att.get_flags()?,
    //             "flagsAtt {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(boost), Some(boost_att)) = (boost, boost_att.as_ref()) {
    //         assert!(
    //             (boost[i] - boost_att.get_boost()?).abs() <= 0.001,
    //             "boostAtt {} term={}",
    //             i,
    //             term_att_ref.to_string()?
    //         );
    //     }
    //     if let (Some(payloads), Some(payload_att)) = (payloads, payload_att.as_ref()) {
    //         match &payloads[i] {
    //             Some(payload) => {
    //                 assert_eq!(
    //                     Some(BytesRef::from_bytes(payload.clone())),
    //                     payload_att.get_payload()?,
    //                     "payloads {}",
    //                     i
    //                 );
    //             }
    //             None => {
    //                 assert!(payload_att.get_payload()?.is_none(), "payloads {}", i);
    //             }
    //         }
    //     }
    //     if let Some(pos_incr_att) = pos_incr_att.as_ref() {
    //         if i == 0 {
    //             assert!(
    //                 pos_incr_att.get_position_increment()? >= 1,
    //                 "first posIncrement must be >= 1"
    //             );
    //         } else {
    //             assert!(
    //                 pos_incr_att.get_position_increment()? >= 0,
    //                 "posIncrement must be >= 0"
    //             );
    //         }
    //     }
    //     if let Some(pos_length_att) = pos_length_att.as_ref() {
    //         let pos_length = pos_length_att.get_position_length()?;
    //         assert!(pos_length >= 1, "posLength must be >= 1; got: {}", pos_length);
    //     }
    //     if let Some(offset_att) = offset_att.as_ref() {
    //         let start_offset = offset_att.start_offset()?;
    //         let end_offset = offset_att.end_offset()?;
    //
    //         if let Some(final_offset) = final_offset {
    //             assert!(
    //                 start_offset <= final_offset,
    //                 "startOffset (= {}) must be <= finalOffset (= {}) term={}",
    //                 start_offset,
    //                 final_offset,
    //                 term_att_ref.to_string()?
    //             );
    //             assert!(
    //                 end_offset <= final_offset,
    //                 "endOffset must be <= finalOffset: got endOffset={} vs finalOffset={} term={}",
    //                 end_offset,
    //                 final_offset,
    //                 term_att_ref.to_string()?
    //             );
    //         }
    //
    //         assert!(
    //             start_offset >= last_start_offset,
    //             "offsets must not go backwards startOffset={} is < lastStartOffset={} term={}",
    //             start_offset,
    //             last_start_offset,
    //             term_att_ref.to_string()?
    //         );
    //         last_start_offset = start_offset;
    //
    //         if graph_offsets_are_correct && pos_length_att.is_some() && pos_incr_att.is_some() {
    //             let pos_inc = pos_incr_att.as_ref().unwrap().get_position_increment()?;
    //             pos += pos_inc;
    //
    //             let pos_length = pos_length_att.as_ref().unwrap().get_position_length()?;
    //
    //             match pos_to_start_offset.get(&pos) {
    //                 None => {
    //                     pos_to_start_offset.insert(pos, start_offset);
    //                 }
    //                 Some(prev_start_offset) => {
    //                     assert_eq!(
    //                         *prev_start_offset,
    //                         start_offset,
    //                         "{} inconsistent startOffset: pos={} posLen={} token={}",
    //                         i,
    //                         pos,
    //                         pos_length,
    //                         term_att_ref.to_string()?
    //                     );
    //                 }
    //             }
    //
    //             let end_pos = pos + pos_length;
    //             match pos_to_end_offset.get(&end_pos) {
    //                 None => {
    //                     pos_to_end_offset.insert(end_pos, end_offset);
    //                 }
    //                 Some(prev_end_offset) => {
    //                     assert_eq!(
    //                         *prev_end_offset,
    //                         end_offset,
    //                         "inconsistent endOffset {} pos={} posLen={} token={}",
    //                         i,
    //                         pos,
    //                         pos_length,
    //                         term_att_ref.to_string()?
    //                     );
    //                 }
    //             }
    //         }
    //     }
    // }
    //
    // if ts.increment_token()? {
    //     let extra = ts.get_attribute::<CharTermAttribute>()?;
    //     panic!(
    //         "TokenStream has more tokens than expected (expected count={}); extra token={}",
    //         output.len(),
    //         extra.to_string()?
    //     );
    // }
    //
    // ts.clear_attributes()?;
    // if let Some(term_att) = term_att.as_mut() {
    //     term_att.set_empty()?;
    //     term_att.append("bogusTerm")?;
    // }
    // if let Some(offset_att) = offset_att.as_mut() {
    //     offset_att.set_offset(14584724, 24683243)?;
    // }
    // if let Some(type_att) = type_att.as_mut() {
    //     type_att.set_type("bogusType")?;
    // }
    // if let Some(pos_incr_att) = pos_incr_att.as_mut() {
    //     pos_incr_att.set_position_increment(45987657)?;
    // }
    // if let Some(pos_length_att) = pos_length_att.as_mut() {
    //     pos_length_att.set_position_length(45987653)?;
    // }
    // if let Some(keyword_att) = keyword_att.as_mut() {
    //     keyword_att.set_keyword(true)?;
    // }
    // if let Some(payload_att) = payload_att.as_mut() {
    //     payload_att.set_payload(Some(BytesRef::from_bytes(vec![
    //         0x00,
    //         (-0x21i8) as u8,
    //         0x12,
    //         (-0x43i8) as u8,
    //         0x24,
    //     ])))?;
    // }
    // if let Some(flags_att) = flags_att.as_mut() {
    //     flags_att.set_flags(!0)?;
    // }
    // if let Some(boost_att) = boost_att.as_mut() {
    //     boost_att.set_boost(-1.0)?;
    // }
    //
    // check_clear_att.get_and_reset_clear_called()?;
    // ts.end()?;
    // assert!(
    //     check_clear_att.get_and_reset_clear_called()?,
    //     "super.end()/clearAttributes() was not called correctly in end()"
    // );
    //
    // if let (Some(final_offset), Some(offset_att)) = (final_offset, offset_att.as_ref()) {
    //     assert_eq!(final_offset, offset_att.end_offset()?, "finalOffset");
    // }
    // if let Some(offset_att) = offset_att.as_ref() {
    //     assert!(offset_att.end_offset()? >= 0, "finalOffset must be >= 0");
    // }
    // if let (Some(final_pos_inc), Some(pos_incr_att)) = (final_pos_inc, pos_incr_att.as_ref()) {
    //     assert_eq!(
    //         final_pos_inc,
    //         pos_incr_att.get_position_increment()?,
    //         "finalPosInc"
    //     );
    // }
    //
    // ts.close()?;
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
