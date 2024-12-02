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
use crate::util::error::illegal_argument::IllegalArgument;
use crate::util::error::illegal_state::IllegalState;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgument),

    #[error("{0}")]
    IllegalState(#[from] IllegalState),
}

impl RuntimeError {
    pub fn illegal_argument(msg: impl Into<String>) -> Self {
        RuntimeError::IllegalArgument(IllegalArgument::new(msg))
    }

    pub fn illegal_state(msg: impl Into<String>) -> Self {
        RuntimeError::IllegalState(IllegalState::new(msg))
    }
}
