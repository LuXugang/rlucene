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
use std::fmt::Display;

/// Per-document scoring factors.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Impact {
    /// Term frequency of the term in the document.
    pub freq: i32,

    /// Norm factor of the document.
    pub norm: i64,
}

impl Impact {
    /// Constructor
    pub fn new(freq: i32, norm: i64) -> Self {
        Self { freq, norm }
    }
}

impl Display for Impact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{freq={},norm={}}}", self.freq, self.norm)
    }
}
