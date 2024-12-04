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
use crate::util::error::corrupt_index::CorruptIndex;
use crate::util::error::eof::Eof;
use crate::util::error::illegal_argument::IllegalArgument;
use crate::util::error::illegal_state::IllegalState;
use crate::util::error::index_format_too_new::IndexFormatTooNew;
use crate::util::error::integer_overflow::IntegerOverflow;
use std::io::Error;
use std::string::FromUtf8Error;
use thiserror::Error;
use crate::util::error::index_format_too_old::IndexFormatTooOld;

#[derive(Debug, Error)]
pub enum DataIOError {
    #[error("IO error: {0}")]
    Io(#[from] Error),

    #[error("UTF-8 conversion error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgument),

    #[error("{0}")]
    IllegalState(#[from] IllegalState),

    #[error("{0}")]
    Eof(#[from] Eof),

    #[error("{0}")]
    IntegerOverflow(#[from] IntegerOverflow),

    #[error("{0}")]
    CorruptIndex(#[from] CorruptIndex),

    #[error("{0}")]
    IndexFormatTooNew(#[from] IndexFormatTooNew),
    
    #[error("{0}")]
    IndexFormatTooOld(#[from] IndexFormatTooOld),
}
impl DataIOError {
    pub fn io(err: Error) -> Self {
        DataIOError::Io(err)
    }

    pub fn utf8(err: FromUtf8Error) -> Self {
        DataIOError::FromUtf8Error(err)
    }
    pub fn illegal_argument(msg: impl Into<String>) -> Self {
        DataIOError::IllegalArgument(IllegalArgument::new(msg))
    }
    pub fn illegal_state(msg: impl Into<String>) -> Self {
        DataIOError::IllegalState(IllegalState::new(msg))
    }

    pub fn eof(msg: impl Into<String>) -> Self {
        DataIOError::Eof(Eof::new(msg))
    }

    pub fn integer_overflow(msg: impl Into<String>) -> Self {
        DataIOError::IntegerOverflow(IntegerOverflow::new(msg))
    }

    pub fn corrupt_index(msg: impl Into<String>) -> Self {
        DataIOError::CorruptIndex(CorruptIndex::new(msg))
    }

    pub fn index_format_too_new(msg: impl Into<String>) -> Self {
        DataIOError::IndexFormatTooNew(IndexFormatTooNew::new(msg))
    }
    pub fn index_format_too_old(msg: impl Into<String>) -> Self {
        DataIOError::IndexFormatTooOld(IndexFormatTooOld::new(msg))
    }
}
