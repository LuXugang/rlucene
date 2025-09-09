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
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
/// Pair of states
pub(crate) struct StatePair {
    pub(crate) s1: i32,
    pub(crate) s2: i32,
    // only mike knows what it does (do not expose)
    pub(crate) s: i32,
}

impl StatePair {
    pub(crate) fn new_with_s(s: i32, s1: i32, s2: i32) -> Self {
        StatePair { s1, s2, s }
    }

    /// Constructs a new state pair.
    pub(crate) fn new(s1: i32, s2: i32) -> Self {
        StatePair { s1, s2, s: -1 }
    }
}

impl PartialEq for StatePair {
    fn eq(&self, other: &Self) -> bool {
        self.s1 == other.s1 && self.s2 == other.s2
    }
}

impl Eq for StatePair {}

impl Hash for StatePair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.s1.hash(state);
        self.s2.hash(state);
    }
}

impl fmt::Display for StatePair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(s1={} s2={})",
            std::any::type_name::<Self>(),
            self.s1,
            self.s2
        )
    }
}
