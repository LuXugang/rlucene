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
use crate::index::term_state::TermState;
use crate::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Display, Formatter};
#[derive(Debug, Clone)]
pub struct DummyTermState;
impl Display for DummyTermState {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("this method should never be called")
    }
}

impl TermState for DummyTermState {
    fn copy_from(&mut self, _other: &impl TermState) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn to_string(&self) -> String {
        unreachable!("this method should never be called")
    }
}
