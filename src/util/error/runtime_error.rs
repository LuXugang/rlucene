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
use std::fmt;
use std::fmt::{Debug};
use crate::util::error::illegal_argument::IllegalArgument;
use crate::util::error::illegal_state::IllegalState;

#[derive(Debug)]
pub enum RuntimeError {
    Argument(IllegalArgument),
    State(IllegalState),
}

impl RuntimeError {
    pub fn argument(msg: impl Into<String>) -> Self {
        Self::Argument(IllegalArgument::new(msg))
    }

    pub fn state(msg: impl Into<String>) -> Self {
        Self::State(IllegalState::new(msg))
    }
}

impl From<IllegalArgument> for RuntimeError {
    fn from(err: IllegalArgument) -> Self {
        RuntimeError::Argument(err)
    }
}

impl From<IllegalState> for RuntimeError {
    fn from(err: IllegalState) -> Self {
        RuntimeError::State(err)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Argument(err) => write!(f, "{}", err),
            RuntimeError::State(err) => write!(f, "{}", err),
        }
    }
}



impl std::error::Error for RuntimeError {}