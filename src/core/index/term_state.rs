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

use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Encapsulates all required internal state to position the associated
/// [`TermsEnum`](crate::core::index::terms_enum::TermsEnum) without re-seeking.
pub trait TermState: Display + Clone {
    /// Copies the content of the given `TermState` to this instance.
    fn copy_from(&mut self, other: &Self) -> Result<()>;
}

// TermState
pub enum TermStateEnum2<A, B> {
    A(A),
    B(B),
}

impl<A, B> Display for TermStateEnum2<A, B>
where
    A: TermState,
    B: TermState,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TermStateEnum2::A(t) => write!(f, "TermStateEnum::A({t})"),
            TermStateEnum2::B(s) => write!(f, "TermStateEnum::B({s})"),
        }
    }
}

impl<A, B> Clone for TermStateEnum2<A, B>
where
    A: TermState,
    B: TermState,
{
    fn clone(&self) -> Self {
        match self {
            TermStateEnum2::A(t) => TermStateEnum2::A(t.clone()),
            TermStateEnum2::B(s) => TermStateEnum2::B(s.clone()),
        }
    }
}

impl<A, B> TermState for TermStateEnum2<A, B>
where
    A: TermState,
    B: TermState,
{
    fn copy_from(&mut self, other: &Self) -> Result<()> {
        match (self, other) {
            (TermStateEnum2::A(t), TermStateEnum2::A(o)) => t.copy_from(o),
            (TermStateEnum2::B(s), TermStateEnum2::B(o)) => s.copy_from(o),
            _ => Err(LuceneError::illegal_state(
                "TermState variants must match when copying",
            )),
        }
    }
}
