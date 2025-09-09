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
use crate::core::util::error::IllegalArgumentError;

#[derive(Debug)]
pub struct Parse {
    pub message: String,
    pub position: i32,
    pub error: Option<IllegalArgumentError>,
}

impl Parse {
    pub fn new(msg: impl Into<String>, position: i32) -> Self {
        Self {
            message: msg.into(),
            position,
            error: None,
        }
    }
    pub fn with_error(msg: impl Into<String>, error: Option<IllegalArgumentError>) -> Self {
        Self {
            message: msg.into(),
            position: 0,
            error,
        }
    }
}

impl std::fmt::Display for Parse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.error.is_some() {
            write!(
                f,
                "Parse Error at {}: {} reason: {}",
                self.position,
                self.message,
                self.error.as_ref().unwrap().message
            )
        } else {
            write!(f, "Parse Error at {}: {}", self.position, self.message)
        }
    }
}

impl std::error::Error for Parse {}
