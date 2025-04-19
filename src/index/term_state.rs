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
use crate::codecs::block_term_state::BlockTermState;
use crate::codecs::lucene101::lucene101_postings_format::IntBlockTermState;
use crate::index::base_terms_enum::TermStateImpl1;
use crate::index::dummy::dummy_term_state_type::DummyTermState;
use crate::index::ord_term_state::OrdTermState;
use crate::util::error::lucene_error::Result;
use std::fmt::{Display, Formatter};

/// Encapsulates all required internal state to position the associated [`TermsEnum`](crate::index::terms_enum::TermsEnum) without re-seeking.
pub trait TermState: Display + Clone {
    /// Copies the content of the given `TermState` to this instance.
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()>;
}

pub enum TermStateEnum {
    Dummy(DummyTermState),
    Impl1(TermStateImpl1),
    Ord(OrdTermState),
    Block(BlockTermState),
    IntBlock(IntBlockTermState),
}

impl Display for TermStateEnum {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Clone for TermStateEnum {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl TermState for TermStateEnum {
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()> {
        todo!()
    }
}
