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
use rlucene::util::error::corrupt_index::CorruptIndexError;
use rlucene::util::error::data_io_error_enum::DataIOError;
use rlucene::util::error::eof::Eof;
use rlucene::util::error::illegal_argument::IllegalArgumentError;
use rlucene::util::error::illegal_state::IllegalStateError;
use rlucene::util::error::index_format_too_new::IndexFormatTooNewError;
use rlucene::util::error::integer_overflow::IntegerOverflow;

use rlucene::util::version::VersionError;
use std::io::Error;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TestError {
    // single error
    #[error("IO error: {0}")]
    Io(#[from] Error),

    #[error("UTF-8 conversion error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("{0}")]
    IllegalArgument(#[from] IllegalArgumentError),

    #[error("{0}")]
    Eof(#[from] Eof),

    #[error("{0}")]
    IntegerOverflow(#[from] IntegerOverflow),

    #[error("{0}")]
    CorruptIndex(#[from] CorruptIndexError),

    #[error("{0}")]
    IndexFormat(#[from] IndexFormatTooNewError),

    #[error("{0}")]
    IllegalState(#[from] IllegalStateError),
    #[error("{0}")]
    DataIOError(#[from] DataIOError),

    #[error("{0}")]
    VersionError(#[from] VersionError),
}
