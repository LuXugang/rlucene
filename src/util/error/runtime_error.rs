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
use crate::util::error::array_index_out_of_bounds::ArrayIndexOutOfBoundsError;
use crate::util::error::illegal_argument::IllegalArgumentError;
use crate::util::error::illegal_state::IllegalStateError;
use std::fmt::Debug;
use std::io::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgumentError),

    #[error("{0}")]
    IllegalState(#[from] IllegalStateError),

    #[error("IO error: {0}")]
    Io(#[from] Error),

    #[error("{0}")]
    ArrayIndexOutOfBounds(#[from] ArrayIndexOutOfBoundsError),

    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("UTF-8 conversion error: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

impl RuntimeError {
    pub fn illegal_argument(msg: impl Into<String>) -> Self {
        RuntimeError::IllegalArgument(IllegalArgumentError::new(msg))
    }

    pub fn illegal_state(msg: impl Into<String>) -> Self {
        RuntimeError::IllegalState(IllegalStateError::new(msg))
    }

    pub fn array_index_out_of_bounds(msg: impl Into<String>) -> Self {
        RuntimeError::ArrayIndexOutOfBounds(ArrayIndexOutOfBoundsError::new(msg))
    }

    pub fn io(err: Error) -> Self {
        RuntimeError::Io(err)
    }

    pub fn utf8_error(err: std::str::Utf8Error) -> Self {
        RuntimeError::Utf8Error(err)
    }

    pub fn from_utf8_error(err: std::string::FromUtf8Error) -> Self {
        RuntimeError::FromUtf8Error(err)
    }
}
