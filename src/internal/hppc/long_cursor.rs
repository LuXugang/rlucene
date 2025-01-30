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

/// Forked from HPPC, holding int index and long value.
///
/// This structure is intended for internal usage within the Lucene system.
#[derive(Debug, Clone, PartialEq)]
pub struct LongCursor {
    /// The current value's index in the container this cursor belongs to.
    /// The meaning of this index is defined by the container (usually it will
    /// be an index in the underlying storage buffer).
    pub index: i32,

    /// The current value.
    pub value: i64,
}

impl Display for LongCursor {
    /// Converts the `LongCursor` to a readable string representation.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[cursor, index: {}, value: {}]", self.index, self.value)
    }
}

impl LongCursor {
    pub fn new(index: i32, value: i64) -> Self {
        Self { index, value }
    }
}
