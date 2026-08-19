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
use std::fmt::{Display, Formatter};

use crate::core::codecs::lucene101::lucene101_postings_format::IntBlockTermState;
use crate::core::index::base_terms_enum::BaseTermsEnumTermStateImpl;
use crate::core::index::ord_term_state::OrdTermState;
use crate::core::index::term_state::TermState;
use crate::core::index::term_states::EmptyTermState;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::impl_from_for_enum;

/// Holds all state required for
/// [`PostingsReaderBase`](crate::core::codecs::postings_reader_base::PostingsReaderBase)
/// to produce a [`PostingsEnum`](crate::core::index::postings_enum::PostingsEnum)
/// without re-seeking the terms dict.
#[derive(Default, Clone)]
pub struct BlockTermState {
  /// how many docs have this term
  pub doc_freq: i32,
  /// total number of occurrences of this term
  pub total_term_freq: i64,
  /// the term's ord in the current block
  pub term_block_ord: i32,
  /// fp into the terms dict primary file (_X.tim) that holds this term
  pub block_file_pointer: i64,
  ord: OrdTermState,
}

impl Display for BlockTermState {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} docFreq={} totalTermFreq={} termBlockOrd={} blockFP={}",
      self.ord, self.doc_freq, self.total_term_freq, self.term_block_ord, self.block_file_pointer
    )
  }
}

impl TermState for BlockTermState {
  fn copy_from(&mut self, other: &Self) -> Result<()> {
    self.doc_freq = other.doc_freq;
    self.total_term_freq = other.total_term_freq;
    self.term_block_ord = other.term_block_ord;
    self.block_file_pointer = other.block_file_pointer;
    self.ord = other.ord.clone();
    Ok(())
  }
}

#[derive(Clone)]
pub enum TermStateEnum {
  Int(IntBlockTermState),
  Block(BlockTermState),
  Empty(EmptyTermState),
  Ord(OrdTermState),
  BaseTermsEnum(BaseTermsEnumTermStateImpl),
}
impl_from_for_enum!(
    TermStateEnum,
    IntBlockTermState => Int,
    BlockTermState => Block,
    EmptyTermState => Empty,
    OrdTermState => Ord,
    BaseTermsEnumTermStateImpl => BaseTermsEnum,
);

impl TermStateEnum {
  pub fn get_block_term_state_mut(&mut self) -> Result<&mut BlockTermState> {
    match self {
      TermStateEnum::Int(int) => Ok(&mut int.base),
      TermStateEnum::Block(block) => Ok(block),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }
  pub fn get_block_term_state(&self) -> Result<&BlockTermState> {
    match self {
      TermStateEnum::Int(int) => Ok(&int.base),
      TermStateEnum::Block(block) => Ok(block),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }
  pub fn ord(&self) -> Result<i64> {
    match self {
      TermStateEnum::Int(int) => Ok(int.base.ord.ord),
      TermStateEnum::Block(block) => Ok(block.ord.ord),
      TermStateEnum::Ord(ord) => Ok(ord.ord),
      _ => Err(LuceneError::unsupported_operation("")),
    }
  }
  pub fn set_ord(&mut self, ord: i64) -> Result<()> {
    match self {
      TermStateEnum::Int(v) => v.base.ord.ord = ord,
      TermStateEnum::Block(v) => v.ord.ord = ord,
      TermStateEnum::Ord(v) => v.ord = ord,
      _ => return Err(LuceneError::unsupported_operation("")),
    }
    Ok(())
  }
}

impl Display for TermStateEnum {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TermStateEnum::Int(v) => v.fmt(f),
      TermStateEnum::Block(v) => v.fmt(f),
      TermStateEnum::Empty(v) => v.fmt(f),
      TermStateEnum::Ord(v) => v.fmt(f),
      TermStateEnum::BaseTermsEnum(v) => v.fmt(f),
    }
  }
}

impl TermState for TermStateEnum {
  fn copy_from(&mut self, other: &Self) -> Result<()> {
    match (self, other) {
      (TermStateEnum::Int(int), TermStateEnum::Int(o)) => int.copy_from(o),
      (TermStateEnum::Block(block), TermStateEnum::Block(o)) => block.copy_from(o),
      (TermStateEnum::Empty(empty), TermStateEnum::Empty(o)) => empty.copy_from(o),
      (TermStateEnum::Ord(ord), TermStateEnum::Ord(o)) => ord.copy_from(o),
      (TermStateEnum::BaseTermsEnum(ord), TermStateEnum::BaseTermsEnum(o)) => ord.copy_from(o),
      _ => Err(LuceneError::illegal_state(
        "TermState variants must match when copying",
      )),
    }
  }
}
