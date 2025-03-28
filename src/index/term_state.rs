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
use crate::index::base_terms_enum::TermStateImpl1;
use crate::index::dummy::dummy_term_state_type::DummyTermState;
use crate::util::error::lucene_error::Result;
use std::fmt::{Debug, Display, Formatter};

/// Encapsulates all required internal state to position the associated [`TermsEnum`] without re-seeking.
pub trait TermState: Debug + Display + Clone {
    /// Copies the content of the given `TermState` to this instance.
    fn copy_from(&mut self, other: &impl TermState) -> Result<()>;
    fn to_string(&self) -> String {
        "TermState".to_string()
    }
}

pub enum TermStateEnum {
    Dummy(DummyTermState),
    Impl1(TermStateImpl1),
}

impl Debug for TermStateEnum {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
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
    fn copy_from(&mut self, _other: &impl TermState) -> Result<()> {
        todo!()
    }

    fn to_string(&self) -> String {
        todo!()
    }
}
