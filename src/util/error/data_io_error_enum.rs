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
use std::io::Error;
use std::string::FromUtf8Error;
use crate::util::error::illegal_argument::IllegalArgument;

#[derive(Debug)]
pub enum DataIOError {
    Io(Error),
    Utf8(FromUtf8Error),
    IA(IllegalArgument),
}

impl DataIOError{
    pub fn argument(msg: impl Into<String>) -> Self {
        Self::IA(IllegalArgument::new(msg))
    }
    pub fn io(err: Error) -> Self {
        Self::Io(err)
    }
    pub fn utf8(err: FromUtf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl fmt::Display for DataIOError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataIOError::Io(err) => write!(f, "IO error: {}", err),
            DataIOError::Utf8(err) => write!(f, "UTF-8 conversion error: {}", err),
            DataIOError::IA(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for DataIOError {}

impl From<Error> for DataIOError {
    fn from(err: Error) -> Self {
        DataIOError::Io(err)
    }
}

impl From<FromUtf8Error> for DataIOError {
    fn from(err: FromUtf8Error) -> Self {
        DataIOError::Utf8(err)
    }
}

impl From<IllegalArgument> for DataIOError {
    fn from(err: IllegalArgument) -> Self {
        DataIOError::IA(err)
    }
}